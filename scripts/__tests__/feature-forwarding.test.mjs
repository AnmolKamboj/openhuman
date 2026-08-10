import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';

import {
  checkProductForwarding,
  diffForwarding,
  INTENTIONALLY_NOT_FORWARDED,
  parseCoreDefaultFeatures,
  parseCoreFeatureNames,
  parseProductFeatures,
  parseShellForwardedFeatures,
  stripComments,
} from '../lib/feature-forwarding.mjs';

const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const CHECKER = resolve(REPO_ROOT, 'scripts/ci/check-feature-forwarding.mjs');

// ── parsing ────────────────────────────────────────────────────────────────

test('parses the core default gate list', () => {
  const toml = `
[features]
default = ["tokenjuice-treesitter", "voice", "media"]
voice = ["dep:hound"]
`;
  assert.deepEqual(parseCoreDefaultFeatures(toml), ['tokenjuice-treesitter', 'voice', 'media']);
});

test('parses a multi-line default gate list', () => {
  const toml = `
[features]
default = [
    "voice",
    "media",
]
`;
  assert.deepEqual(parseCoreDefaultFeatures(toml), ['voice', 'media']);
});

test('ignores a default key belonging to another table', () => {
  const toml = `
[some-other-table]
default = ["not-a-gate"]

[features]
default = ["voice"]
`;
  assert.deepEqual(parseCoreDefaultFeatures(toml), ['voice']);
});

test('parses the shell forwarded list across multiple lines', () => {
  const toml = `
openhuman_core = { path = "../..", package = "openhuman", default-features = false, features = [
    "media",
    "voice",
] }
`;
  assert.deepEqual(parseShellForwardedFeatures(toml), {
    defaultFeatures: false,
    features: ['media', 'voice'],
  });
});

test('detects when the shell inherits defaults instead of forwarding', () => {
  const toml = 'openhuman_core = { path = "../..", package = "openhuman" }\n';
  assert.deepEqual(parseShellForwardedFeatures(toml), { defaultFeatures: true, features: [] });
});

test('comment stripping does not truncate on a # inside a quoted value', () => {
  const stripped = stripComments('a = "issue #4901"  # trailing comment\n');
  assert.match(stripped, /issue #4901/);
  assert.doesNotMatch(stripped, /trailing comment/);
});

test('a commented-out gate does not count as forwarded', () => {
  const toml = `
openhuman_core = { path = "../..", package = "openhuman", default-features = false, features = [
    # "voice",
    "media",
] }
`;
  assert.deepEqual(parseShellForwardedFeatures(toml).features, ['media']);
});

// ── drift detection ────────────────────────────────────────────────────────

test('passes when every default gate is forwarded', () => {
  const result = diffForwarding({
    coreDefaults: ['voice', 'media'],
    shell: { defaultFeatures: false, features: ['media', 'voice'] },
  });
  assert.equal(result.ok, true);
  assert.deepEqual(result.missing, []);
});

test('reproduces #4901: a dropped voice gate is reported missing', () => {
  const result = diffForwarding({
    coreDefaults: ['tokenjuice-treesitter', 'voice', 'media'],
    shell: { defaultFeatures: false, features: ['media', 'tokenjuice-treesitter'] },
  });
  assert.equal(result.ok, false);
  assert.deepEqual(result.missing, ['voice']);
});

test('reproduces #4918: a dropped tokenjuice-treesitter gate is reported missing', () => {
  const result = diffForwarding({
    coreDefaults: ['tokenjuice-treesitter', 'voice', 'media'],
    shell: { defaultFeatures: false, features: ['media', 'voice'] },
  });
  assert.equal(result.ok, false);
  assert.deepEqual(result.missing, ['tokenjuice-treesitter']);
});

test('a brand new default gate is covered automatically, with no per-gate wiring', () => {
  const result = diffForwarding({
    coreDefaults: ['voice', 'media', 'some-future-gate'],
    shell: { defaultFeatures: false, features: ['voice', 'media'] },
  });
  assert.equal(result.ok, false);
  assert.deepEqual(result.missing, ['some-future-gate']);
});

test('an allow-listed gate passes and is reported as intentional', () => {
  const result = diffForwarding({
    coreDefaults: ['voice', 'heavy-gate'],
    shell: { defaultFeatures: false, features: ['voice'] },
    allowlist: { 'heavy-gate': 'Adds 400MB of models to the bundle.' },
  });
  assert.equal(result.ok, true);
  assert.deepEqual(result.allowed, ['heavy-gate']);
  assert.deepEqual(result.missing, []);
});

test('an allow-list entry for a gate that IS forwarded is flagged as stale', () => {
  const result = diffForwarding({
    coreDefaults: ['voice'],
    shell: { defaultFeatures: false, features: ['voice'] },
    allowlist: { voice: 'stale entry' },
  });
  assert.equal(result.ok, false);
  assert.deepEqual(result.stale, ['voice']);
});

test('inheriting defaults needs no forwarding', () => {
  const result = diffForwarding({
    coreDefaults: ['voice'],
    shell: { defaultFeatures: true, features: [] },
  });
  assert.equal(result.ok, true);
});

test('a missing dependency fails rather than passing vacuously', () => {
  const result = diffForwarding({ coreDefaults: ['voice'], shell: null });
  assert.equal(result.ok, false);
  assert.equal(result.reason, 'dependency-not-found');
});

// ── product-set forwarding (assertions 1 + 2) ──────────────────────────────

const PRODUCT = ['voice', 'media'];
const CORE_GATES = ['voice', 'media', 'web3', 'tui'];

test('passes when the shell forwards exactly the product set', () => {
  const result = checkProductForwarding({
    productFeatures: PRODUCT,
    coreFeatureNames: CORE_GATES,
    shell: { defaultFeatures: false, features: ['media', 'voice'] },
  });
  assert.equal(result.ok, true);
});

test('reproduces #4901 against the PRODUCT set, not the default set', () => {
  // The point of the rewrite: this must fail even though `default` here is
  // empty, which is precisely the shape that made the old subset check pass
  // vacuously as `default` shrank.
  const result = checkProductForwarding({
    productFeatures: PRODUCT,
    coreFeatureNames: CORE_GATES,
    shell: { defaultFeatures: false, features: ['media'] },
  });
  assert.equal(result.ok, false);
  assert.deepEqual(result.missing, ['voice']);
});

test('a gate the shell forwards but the product does not claim is flagged', () => {
  const result = checkProductForwarding({
    productFeatures: PRODUCT,
    coreFeatureNames: CORE_GATES,
    shell: { defaultFeatures: false, features: ['media', 'voice', 'web3'] },
  });
  assert.equal(result.ok, false);
  assert.deepEqual(result.unexpected, ['web3']);
});

test('a product gate that is not a real core gate is flagged', () => {
  const result = checkProductForwarding({
    productFeatures: ['voice', 'vioce'],
    coreFeatureNames: CORE_GATES,
    shell: { defaultFeatures: false, features: ['voice', 'vioce'] },
  });
  assert.equal(result.ok, false);
  assert.deepEqual(result.unknown, ['vioce']);
});

test('the shell inheriting defaults is now a FAILURE, not a pass', () => {
  // It used to mean "nothing to drift". It now means the shell would inherit
  // the contributor set, which is smaller than the product.
  const result = checkProductForwarding({
    productFeatures: PRODUCT,
    coreFeatureNames: CORE_GATES,
    shell: { defaultFeatures: true, features: [] },
  });
  assert.equal(result.ok, false);
  assert.equal(result.reason, 'shell-inherits-defaults');
});

test('parses the product file, ignoring comments and blank lines', () => {
  const text = '# a comment\n\nvoice\n  media  # trailing\n\n';
  assert.deepEqual(parseProductFeatures(text), ['voice', 'media']);
});

test('parses every gate name from the core [features] table, minus `default`', () => {
  const toml = `
[features]
default = ["media"]
voice = ["dep:hound"]
media = []

[dependencies]
hound = "3"
`;
  assert.deepEqual(parseCoreFeatureNames(toml), ['voice', 'media']);
});

// ── the real manifests + CLI ───────────────────────────────────────────────

test('the checked-in manifests pass the guard', () => {
  const out = execFileSync('node', [CHECKER], { encoding: 'utf8' });
  assert.match(out, /the shell forwards exactly the product gate set/);
  assert.match(out, /every default-ON core gate is forwarded/);
});

test('the real product file and the real shell list are equal', () => {
  const productFeatures = parseProductFeatures(
    readFileSync(resolve(REPO_ROOT, 'scripts/ci/product-features.txt'), 'utf8')
  );
  const coreFeatureNames = parseCoreFeatureNames(
    readFileSync(resolve(REPO_ROOT, 'Cargo.toml'), 'utf8')
  );
  const shell = parseShellForwardedFeatures(
    readFileSync(resolve(REPO_ROOT, 'app/src-tauri/Cargo.toml'), 'utf8')
  );
  // Guards the guard: empty input would make every assertion below vacuous.
  assert.ok(productFeatures.length > 0, 'expected to parse at least one product gate');
  assert.ok(coreFeatureNames.length > 0, 'expected to parse at least one core gate name');
  const result = checkProductForwarding({ productFeatures, coreFeatureNames, shell });
  assert.deepEqual(result.missing, [], 'product gates the shell does not forward');
  assert.deepEqual(result.unexpected, [], 'gates the shell forwards that the product omits');
  assert.deepEqual(result.unknown, [], 'product gates that are not real core gates');
});

test('the shell helper reports an empty gate list instead of dying silently', () => {
  // Regression. The helper filters comments with `grep -v`, which exits 1 when
  // it selects nothing; under `set -e` that aborted the script INSIDE the
  // command substitution, so a comments-only file exited 1 with no output at
  // all and the explicit diagnostic below it was unreachable. A CI lane would
  // have seen a bare failure with nothing naming the cause.
  const tmp = join(tmpdir(), `product-features-empty-${process.pid}.txt`);
  writeFileSync(tmp, '# only a comment\n\n   \n');
  try {
    const result = spawnSync(
      'bash',
      [resolve(REPO_ROOT, 'scripts/ci/product-features.sh'), tmp],
      { encoding: 'utf8' }
    );
    assert.equal(result.status, 2, 'an empty gate list must exit 2, not 1');
    assert.match(result.stderr, /empty gate list/);
    assert.equal(result.stdout.trim(), '', 'nothing may be emitted for an empty list');
  } finally {
    rmSync(tmp, { force: true });
  }
});

test('the shell helper parses a fixture the same way the JS parser does', () => {
  const tmp = join(tmpdir(), `product-features-fixture-${process.pid}.txt`);
  writeFileSync(tmp, '# heading\n\nvoice\n  media  # trailing comment\n\nweb3\n');
  try {
    const out = execFileSync(
      'bash',
      [resolve(REPO_ROOT, 'scripts/ci/product-features.sh'), tmp],
      { encoding: 'utf8' }
    ).trim();
    assert.equal(out, 'voice,media,web3');
    assert.deepEqual(out.split(','), parseProductFeatures(readFileSync(tmp, 'utf8')));
  } finally {
    rmSync(tmp, { force: true });
  }
});

test('the shell script and the JS parser agree on the product set', () => {
  // Two parsers read scripts/ci/product-features.txt: this one, and the shell
  // helper the CI lanes use to build `--features`. If they disagreed, CI would
  // compile a different set than the guard asserts — and the guard would be
  // checking something nobody builds.
  const fromJs = parseProductFeatures(
    readFileSync(resolve(REPO_ROOT, 'scripts/ci/product-features.txt'), 'utf8')
  );
  const fromSh = execFileSync('bash', [resolve(REPO_ROOT, 'scripts/ci/product-features.sh')], {
    encoding: 'utf8',
  })
    .trim()
    .split(',');
  assert.deepEqual(fromSh, fromJs);
});

test('--help exits 0', () => {
  const out = execFileSync('node', [CHECKER, '--help'], { encoding: 'utf8' });
  assert.match(out, /Usage:/);
});

test('the real shell manifest forwards every real core default', () => {
  const coreDefaults = parseCoreDefaultFeatures(
    readFileSync(resolve(REPO_ROOT, 'Cargo.toml'), 'utf8')
  );
  const shell = parseShellForwardedFeatures(
    readFileSync(resolve(REPO_ROOT, 'app/src-tauri/Cargo.toml'), 'utf8')
  );
  // Guards the guard: if the parser silently returned nothing, the assertions
  // below would pass against empty input and prove nothing.
  assert.ok(coreDefaults.length > 0, 'expected to parse at least one core default gate');
  assert.equal(shell.defaultFeatures, false, 'shell is expected to set default-features = false');
  for (const gate of coreDefaults) {
    // Gates the shell intentionally does not forward (e.g. `tui` — a terminal
    // subcommand the desktop app never runs) are exempt, matching the checker.
    if (INTENTIONALLY_NOT_FORWARDED[gate]) continue;
    assert.ok(
      shell.features.includes(gate),
      `core default gate not forwarded to the shell: ${gate}`
    );
  }
});
