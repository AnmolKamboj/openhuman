// Regression tests for status propagation in scripts/ci/rust-coverage-changed.sh.
//
// `run_counted` runs its command inside a pipeline under `set +e` so it can tee
// and count libtest output. That disables errexit for everything underneath,
// which is exactly the condition under which a loop silently swallows a failed
// iteration. These tests pin that a failing coverage module still fails the job.
//
// They evaluate the REAL function bodies, extracted from the script at test
// time, rather than a transcription of them — a copy would keep passing after
// the original regressed, which is the whole failure mode being guarded.

import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const repoRoot = path.join(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
  "..",
);
const runner = path.join(repoRoot, "scripts", "ci", "rust-coverage-changed.sh");

/** Extract the runner's `TESTS_RUN` accumulator declaration, verbatim. */
function extractTestsRunDecl() {
  const source = fs.readFileSync(runner, "utf8");
  const decl = source.split("\n").find((line) => /^TESTS_RUN=/.test(line));
  assert.ok(
    decl,
    `TESTS_RUN= not found in ${runner} — did the accumulator get renamed?`,
  );
  return decl;
}

/** Extract a top-level `name() { … }` block from the runner, verbatim. */
function extractFunction(name) {
  const source = fs.readFileSync(runner, "utf8");
  const start = source.indexOf(`${name}() {`);
  assert.notEqual(
    start,
    -1,
    `${name}() not found in ${runner} — did it get renamed?`,
  );
  const end = source.indexOf("\n}\n", start);
  assert.notEqual(end, -1, `could not find the end of ${name}()`);
  return source.slice(start, end + 3);
}

/**
 * Run a bash snippet with the named runner functions spliced in and the heavy
 * dependencies stubbed.
 *
 * @param {string[]} functions names to lift out of the runner
 * @param {string}   preamble  stubs, defined before the real functions
 * @param {string}   body      the assertion driver
 */
function withRunnerFunctions(functions, preamble, body) {
  const script = [
    "set -euo pipefail",
    extractTestsRunDecl(),
    preamble,
    ...functions.map(extractFunction),
    body,
  ].join("\n");
  try {
    return {
      status: 0,
      output: execFileSync("bash", ["-c", script], { encoding: "utf8" }),
    };
  } catch (err) {
    return {
      status: err.status,
      output: `${err.stdout ?? ""}${err.stderr ?? ""}`,
    };
  }
}

test("a failed raw coverage module fails the run even when a later module succeeds", () => {
  // The exact shape CodeRabbit flagged: module `first` fails, `second` passes.
  // The loop's status is its last iteration's, so without an explicit `return`
  // the failure is discarded and CI goes green on a red suite.
  const res = withRunnerFunctions(
    ["run_counted", "run_integration_target"],
    [
      "log() { printf '%s\\n' \"$*\"; }",
      "raw_coverage_modules() { printf 'first\\nsecond\\n'; }",
      // Neutralise the product-feature gate. Without this the runner skips
      // `raw_coverage_all` before reaching the loop, and the test passes
      // vacuously — asserting failure propagation while never producing a
      // failure. What is under test here is the `|| return`, not the gate.
      "target_features_satisfied() { return 0; }",
      // Fails for `first::`, succeeds otherwise.
      "llvm_cov() { for a in \"$@\"; do case \"$a\" in first::) echo 'module first FAILED'; return 7 ;; esac; done; echo 'module ok'; return 0; }",
    ].join("\n"),
    [
      "if run_counted run_integration_target raw_coverage_all; then",
      "  echo 'WRAPPER-SAID-SUCCESS'; exit 0",
      "else",
      '  echo "WRAPPER-SAID-FAILURE rc=$?"; exit 3',
      "fi",
    ].join("\n"),
  );

  assert.equal(
    res.status,
    3,
    `expected the wrapper to report failure, got:\n${res.output}`,
  );
  assert.match(res.output, /WRAPPER-SAID-FAILURE/);
  // Fail-fast: nothing after the failing module may run.
  assert.doesNotMatch(res.output, /running raw coverage module: second/);
});

test("run_counted propagates the command status, not tee's", () => {
  // `${PIPESTATUS[0]}` rather than `$?`. Reading `$?` after the pipe reports
  // tee, which always succeeds, turning every failing suite green.
  const res = withRunnerFunctions(
    ["run_counted"],
    "boom() { echo 'running 3 tests'; return 9; }",
    "run_counted boom || { echo \"rc=$?\"; exit 0; }; echo 'NO-FAILURE-SEEN'; exit 1",
  );
  assert.equal(res.status, 0, res.output);
  assert.match(res.output, /rc=9/);
});

test("run_counted sums libtest counts across calls and passes success through", () => {
  const res = withRunnerFunctions(
    ["run_counted"],
    "some() { echo 'running 12 tests'; }\nmore() { echo 'running 1 test'; }\nnone() { echo 'running 0 tests'; }",
    'run_counted some; run_counted more; run_counted none; echo "TOTAL=${TESTS_RUN}"',
  );
  assert.equal(res.status, 0, res.output);
  // 12 + 1 + 0 — and "1 test" singular must parse, or a one-test domain looks
  // like a zero-test run and needlessly escalates to the full suite.
  assert.match(res.output, /TOTAL=13/);
});

test("run_counted counts zero for a run that executed no tests", () => {
  const res = withRunnerFunctions(
    ["run_counted"],
    "nothing() { echo 'running 0 tests'; echo 'test result: ok. 0 passed; 12202 filtered out'; }",
    'run_counted nothing; [ "${TESTS_RUN}" -eq 0 ] && echo \'ZERO\' || echo "NONZERO=${TESTS_RUN}"',
  );
  assert.equal(res.status, 0, res.output);
  assert.match(res.output, /ZERO/);
});
