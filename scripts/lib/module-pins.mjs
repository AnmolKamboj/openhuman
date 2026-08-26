// Pure parsing and decision logic for the module-pin gates (openhuman#5727).
//
// Everything here is a pure function over strings so it can be tested without a
// git tree; the two CLIs in scripts/ci/ own the git calls and the process exit.
// See scripts/ci/check-module-pins.mjs for what the gate is for.

/** The record names listed in `pub const ALL`, in order. */
export function parseAllList(src) {
  const block = src.match(/pub const ALL: &\[ModuleRecord\] = &\[([\s\S]*?)\];/);
  if (!block) throw new Error('registry.rs: could not find `pub const ALL`');
  return block[1]
    .split(',')
    .map((s) => s.trim())
    .filter((s) => s.length > 0 && !s.startsWith('//'));
}

/** Every `const NAME: ModuleRecord = ModuleRecord { … };`, keyed by NAME. */
export function parseRecords(src) {
  const records = new Map();
  const re = /(?:pub )?const ([A-Z0-9_]+): ModuleRecord = ModuleRecord \{([\s\S]*?)\n\};/g;
  for (const m of src.matchAll(re)) {
    const [, name, body] = m;
    const field = (f) => {
      const hit = body.match(new RegExp(`^\\s*${f}: "([^"]*)"`, 'm'));
      return hit ? hit[1] : null;
    };
    const assets = [];
    const assetRe =
      /PlatformAsset \{\s*host_key: "([^"]+)",\s*archive: "([^"]+)",\s*sha256: "([^"]+)",\s*\}/g;
    for (const a of body.matchAll(assetRe)) {
      assets.push({ hostKey: a[1], archive: a[2], sha256: a[3] });
    }
    records.set(name, { name, id: field('id'), version: field('version'), assets });
  }
  if (records.size === 0) throw new Error('registry.rs: parsed zero ModuleRecord blocks');
  return records;
}

/** `ARTIFACT_CAPABILITIES_PIN` from modules/memory.rs. */
export function parseArtifactCapabilitiesPin(src) {
  const m = src.match(/ARTIFACT_CAPABILITIES_PIN: &str = "([^"]+)"/);
  return m ? m[1] : null;
}

/** The `memory_version` / `memory_sha256` / `memory_archive` literals in a workflow. */
export function parseWorkflowMemoryBlocks(src) {
  return {
    versions: [...src.matchAll(/^\s*memory_version="([^"]+)"/gm)].map((m) => m[1]),
    digests: [...src.matchAll(/^\s*memory_sha256="([^"]+)"/gm)].map((m) => m[1]),
    archives: [...src.matchAll(/^\s*memory_archive="[^"]*\/(tinymemory-module-[^"]+?)"/gm)].map((m) => m[1]),
  };
}

/**
 * Decide what one record's pin state means.
 *
 * `actual` is `git describe --tags` output for the submodule; `exemption` is the
 * matching entry from module-pin-exemptions.json, or undefined.
 *
 * Four outcomes, and three of them are failures — including "exempt but the
 * pins now agree". A stale exemption is not harmless: it is a standing licence
 * for the next real drift on that record to pass unnoticed.
 */
export function classifyPin({ id, version, submodulePath, actual, exemption }) {
  const wanted = `v${version}`;
  if (actual === wanted) {
    if (exemption) {
      return {
        ok: false,
        kind: 'stale-exemption',
        message:
          `"${id}" is listed in module-pin-exemptions.json, but its pins now AGREE ` +
          `(${submodulePath} is at ${actual}). Delete the exemption — a stale exemption ` +
          `hides the next real drift.`,
      };
    }
    return { ok: true, kind: 'match' };
  }
  if (exemption) {
    if (exemption.expect !== actual) {
      return {
        ok: false,
        kind: 'exemption-widened',
        message:
          `"${id}" drift has CHANGED since it was accepted.\n` +
          `    registry.rs version : ${version} (expects tag ${wanted})\n` +
          `    ${submodulePath} is at : ${actual}\n` +
          `    exemption pins      : ${exemption.expect}\n` +
          `    An exemption accepts one specific drift, not any drift. If the new state is\n` +
          `    intended, update \`expect\` and the reason in module-pin-exemptions.json.`,
      };
    }
    return { ok: true, kind: 'exempt', reason: exemption.reason };
  }
  return {
    ok: false,
    kind: 'drift',
    message:
      `"${id}" registry pin and submodule pin describe different releases.\n` +
      `    registry.rs version : ${version}  (would download the v${version} artifact)\n` +
      `    ${submodulePath} is at : ${actual}  (what this build compiles the contract against)\n` +
      `    Fix by moving whichever pin is stale, or — if the two are meant to differ —\n` +
      `    declare it in scripts/ci/module-pin-exemptions.json with a reason.`,
  };
}

/**
 * Every record in `ALL` must be known to the pin map, and every pin-map entry
 * must still be a record.
 *
 * This is the half that survives the next module landing. Six vendored crates
 * were added between 2026-08-20 and 2026-08-27; a gate that enumerated the
 * modules of the day would already be behind, and would report a clean scan
 * while ignoring the newest one.
 */
export function checkPinMapCoverage(activeIds, pinMapIds) {
  const failures = [];
  for (const id of activeIds) {
    if (!pinMapIds.includes(id)) {
      failures.push(
        `modules::registry::ALL contains "${id}", which scripts/ci/check-module-pins.mjs ` +
          `does not know about.\n` +
          `    Add it to PIN_MAP: either name the vendor/ submodule that is the source of\n` +
          `    truth for its version, or set submodule: null with a reason (and sharesWith,\n` +
          `    if it ships out of another crate's release).\n` +
          `    This is deliberate: a new module must not inherit an unchecked pin.`,
      );
    }
  }
  for (const id of pinMapIds) {
    if (!activeIds.includes(id)) {
      failures.push(
        `PIN_MAP has an entry for "${id}", which is no longer in modules::registry::ALL. Remove it.`,
      );
    }
  }
  return failures;
}

/** path -> gitlink sha, parsed from `git ls-tree -r <ref>` output. */
export function parseGitlinks(lsTreeOutput) {
  const map = new Map();
  for (const line of lsTreeOutput.split('\n')) {
    const m = line.match(/^160000 commit ([0-9a-f]{40})\t(.+)$/);
    if (m) map.set(m[2], m[1]);
  }
  return map;
}

/** A deliberate rewind is declared on the branch that does it. */
export function rewindDeclared(text) {
  return /\[pin-rewind\]/i.test(text ?? '');
}
