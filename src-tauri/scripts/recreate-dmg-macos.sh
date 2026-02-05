#!/bin/bash
# Recreate macOS DMG after TDLib bundling
# The original DMG is created by tauri-action BEFORE we bundle TDLib,
# so we need to recreate it with the updated .app bundle.
#
# Usage:
#   ./recreate-dmg-macos.sh <build_type> [target]
#
# Arguments:
#   build_type  - "release" or "debug" (default: release)
#   target      - Optional cross-compilation target (e.g., aarch64-apple-darwin)
#
# Examples:
#   ./recreate-dmg-macos.sh release
#   ./recreate-dmg-macos.sh release aarch64-apple-darwin

set -e

BUILD_TYPE="${1:-release}"
TARGET="${2:-}"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
TAURI_DIR="$(dirname "$SCRIPT_DIR")"

echo "=== macOS DMG Recreation ==="
echo "Build type: ${BUILD_TYPE}"
echo "Target: ${TARGET:-default}"

# --- Determine bundle directories ---
# Tauri v2 puts .app in bundle/macos/ and .dmg in bundle/dmg/
BASE_BUNDLE_DIR=""
for search_dir in \
    "${TAURI_DIR}/target/${TARGET}/${BUILD_TYPE}/bundle" \
    "${TAURI_DIR}/target/${BUILD_TYPE}/bundle" \
    "${TAURI_DIR}/target/release/bundle" \
    "${TAURI_DIR}/target/debug/bundle"; do
    if [ -d "$search_dir/macos" ]; then
        BASE_BUNDLE_DIR="$search_dir"
        break
    fi
done

if [ -z "$BASE_BUNDLE_DIR" ]; then
    echo "Warning: No bundle directory found"
    exit 0
fi

MACOS_DIR="$BASE_BUNDLE_DIR/macos"
DMG_DIR="$BASE_BUNDLE_DIR/dmg"

echo "Bundle base: $BASE_BUNDLE_DIR"

# --- Find the app bundle ---
APP_BUNDLE=$(find "$MACOS_DIR" -name "*.app" -type d 2>/dev/null | head -1)
if [ -z "$APP_BUNDLE" ]; then
    echo "Error: No .app bundle found in $MACOS_DIR"
    exit 1
fi

APP_NAME=$(basename "$APP_BUNDLE" .app)
echo "App bundle: $APP_BUNDLE"

# --- Find the original DMG ---
ORIGINAL_DMG=$(find "$DMG_DIR" -name "*.dmg" -type f 2>/dev/null | head -1)
if [ -z "$ORIGINAL_DMG" ]; then
    echo "Warning: No original DMG found in $DMG_DIR, skipping"
    exit 0
fi

DMG_NAME=$(basename "$ORIGINAL_DMG")
echo "Original DMG: $ORIGINAL_DMG"

# --- Remove old DMG and create new one ---
echo ""
echo "Creating new DMG..."
rm -f "$ORIGINAL_DMG"
hdiutil create -volname "$APP_NAME" -srcfolder "$APP_BUNDLE" -ov -format UDZO "$DMG_DIR/$DMG_NAME"

echo ""
echo "=== DMG recreation complete ==="
ls -lh "$DMG_DIR/$DMG_NAME"
