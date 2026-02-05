#!/bin/bash
# Re-sign macOS app bundle after TDLib bundling
# Required because modifying the bundle invalidates the original code signature.
#
# Usage:
#   ./resign-macos.sh <build_type> [target]
#
# Arguments:
#   build_type  - "release" or "debug" (default: release)
#   target      - Optional cross-compilation target (e.g., aarch64-apple-darwin)
#
# Environment variables:
#   APPLE_SIGNING_IDENTITY  - Signing identity (e.g., "Developer ID Application: ...")
#                             If unset, uses ad-hoc signing ("-") for local testing.
#   APPLE_CERTIFICATE       - Base64-encoded .p12 certificate (CI only)
#   APPLE_CERTIFICATE_PASSWORD - Password for the .p12 certificate (CI only)
#
# Examples:
#   # Local testing (ad-hoc signing)
#   ./resign-macos.sh release
#
#   # CI with identity
#   APPLE_SIGNING_IDENTITY="Developer ID Application: ..." ./resign-macos.sh release aarch64-apple-darwin

set -e

BUILD_TYPE="${1:-release}"
TARGET="${2:-}"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
TAURI_DIR="$(dirname "$SCRIPT_DIR")"

SIGNING_IDENTITY="${APPLE_SIGNING_IDENTITY:--}"

echo "=== macOS App Re-signing ==="
echo "Build type: ${BUILD_TYPE}"
echo "Target: ${TARGET:-default}"
echo "Signing identity: ${SIGNING_IDENTITY}"

# --- CI keychain setup (only when APPLE_CERTIFICATE is provided) ---
KEYCHAIN_PATH=""
if [ -n "$APPLE_CERTIFICATE" ] && [ -n "$APPLE_CERTIFICATE_PASSWORD" ]; then
    echo ""
    echo "Importing certificate into temporary keychain..."
    TEMP_DIR="${RUNNER_TEMP:-/tmp}"
    KEYCHAIN_PATH="$TEMP_DIR/tdlib-signing.keychain-db"
    KEYCHAIN_PASSWORD=$(openssl rand -base64 32)

    security create-keychain -p "$KEYCHAIN_PASSWORD" "$KEYCHAIN_PATH"
    security set-keychain-settings -lut 21600 "$KEYCHAIN_PATH"
    security unlock-keychain -p "$KEYCHAIN_PASSWORD" "$KEYCHAIN_PATH"

    CERT_PATH="$TEMP_DIR/certificate.p12"
    echo "$APPLE_CERTIFICATE" | base64 --decode > "$CERT_PATH"
    security import "$CERT_PATH" -P "$APPLE_CERTIFICATE_PASSWORD" -A -t cert -f pkcs12 -k "$KEYCHAIN_PATH"
    rm "$CERT_PATH"

    security list-keychains -d user -s "$KEYCHAIN_PATH" $(security list-keychains -d user | tr -d '"')
    security set-key-partition-list -S apple-tool:,apple:,codesign: -s -k "$KEYCHAIN_PASSWORD" "$KEYCHAIN_PATH"
    echo "Certificate imported"
fi

# --- Find the app bundle ---
APP_BUNDLE=""
for search_dir in \
    "${TAURI_DIR}/target/${TARGET}/${BUILD_TYPE}/bundle/macos" \
    "${TAURI_DIR}/target/${BUILD_TYPE}/bundle/macos" \
    "${TAURI_DIR}/target/release/bundle/macos" \
    "${TAURI_DIR}/target/debug/bundle/macos"; do
    if [ -d "$search_dir" ]; then
        found=$(find "$search_dir" -name "*.app" -type d 2>/dev/null | head -1)
        if [ -n "$found" ]; then
            APP_BUNDLE="$found"
            break
        fi
    fi
done

if [ -z "$APP_BUNDLE" ]; then
    echo "Warning: No .app bundle found, nothing to sign"
    exit 0
fi

echo ""
echo "Signing app bundle: $APP_BUNDLE"

# --- Sign bundled dylibs first (inner-to-outer signing order) ---
FRAMEWORKS_DIR="$APP_BUNDLE/Contents/Frameworks"
if [ -d "$FRAMEWORKS_DIR" ]; then
    for dylib in "$FRAMEWORKS_DIR"/*.dylib; do
        if [ -f "$dylib" ]; then
            echo "  Signing: $(basename "$dylib")"
            codesign --force --options runtime --sign "$SIGNING_IDENTITY" "$dylib"
        fi
    done
fi

# --- Sign the app bundle ---
echo "  Signing: $(basename "$APP_BUNDLE")"
codesign --force --deep --options runtime --sign "$SIGNING_IDENTITY" "$APP_BUNDLE"

# --- Verify ---
echo ""
echo "Verifying signature..."
codesign --verify --verbose=2 "$APP_BUNDLE" 2>&1 || echo "Warning: Signature verification had issues"

# --- Cleanup CI keychain ---
if [ -n "$KEYCHAIN_PATH" ]; then
    security delete-keychain "$KEYCHAIN_PATH" || true
fi

echo ""
echo "=== Re-signing complete ==="
