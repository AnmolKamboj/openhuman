#!/usr/bin/env bash
# Print the desktop product's core Cargo gates as a comma-separated list,
# ready to paste into `cargo --features "$(scripts/ci/product-features.sh)"`.
#
# Source of truth: scripts/ci/product-features.txt (one gate per line).
# scripts/ci/check-feature-forwarding.mjs asserts that same file equals the
# list app/src-tauri/Cargo.toml forwards, so the product lanes and the shipped
# app can never diverge.
#
# Why the lanes need this at all: `[features] default` is the CONTRIBUTOR set
# now, not the product set. A lane that relies on default features would stop
# compiling and testing voice, web3, documents, meet, contacts and
# crash-reporting — a silent loss of coverage over code that still ships.
set -euo pipefail

FILE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/product-features.txt"

if [[ ! -f "$FILE" ]]; then
  echo "product-features.txt not found at $FILE" >&2
  exit 2
fi

# Strip comments and blank lines, then join with commas. Refuse to emit an
# empty list: a lane silently running with NO features would look green while
# covering nothing, which is the failure mode this whole guard exists to stop.
LIST="$(sed -e 's/#.*//' -e 's/[[:space:]]//g' "$FILE" | grep -v '^$' | paste -sd, -)"

if [[ -z "$LIST" ]]; then
  echo "product-features.txt parsed to an empty gate list — refusing to emit it" >&2
  exit 2
fi

printf '%s\n' "$LIST"
