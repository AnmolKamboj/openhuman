#!/usr/bin/env bash
#
# Download prebuilt TDLib from tdlib-rs releases using curl (avoids reqwest
# timeout during Cargo build). Extracts to tdlib-cache/ for use with local-tdlib.
#
# Usage:
#   cd src-tauri
#   ./scripts/download-tdlib.sh
#
# Then run builds with LOCAL_TDLIB_PATH set, or use yarn tauri:dev / yarn dev:app.
#

set -euo pipefail

TDLIB_VERSION="1.8.29"
TDLIB_RS_VERSION="1.2.0"
BASE_URL="https://github.com/FedericoBruzzone/tdlib-rs/releases/download"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SRC_TAURI_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
CACHE_DIR="${SRC_TAURI_DIR}/tdlib-cache"

# Detect target (CARGO_CFG_* set during cargo build; uname when run manually)
RAW_OS="${CARGO_CFG_TARGET_OS:-$(uname -s | tr '[:upper:]' '[:lower:]')}"
ARCH="${CARGO_CFG_TARGET_ARCH:-$(uname -m)}"
case "$RAW_OS" in
  darwin|macos) OS_NAME="macos" ;;
  linux) OS_NAME="linux" ;;
  windows) OS_NAME="windows" ;;
  *) OS_NAME="$RAW_OS" ;;
esac
case "$ARCH" in
  arm64|aarch64) ARCH_NAME="aarch64" ;;
  x86_64) ARCH_NAME="x86_64" ;;
  *) ARCH_NAME="$ARCH" ;;
esac

ZIP_NAME="tdlib-${TDLIB_VERSION}-${OS_NAME}-${ARCH_NAME}.zip"
URL="${BASE_URL}/v${TDLIB_RS_VERSION}/${ZIP_NAME}"
EXTRACT_TO="${CACHE_DIR}/${OS_NAME}-${ARCH_NAME}"

if [ -f "${EXTRACT_TO}/lib/libtdjson.${TDLIB_VERSION}.dylib" ] || \
   [ -f "${EXTRACT_TO}/lib/libtdjson.so.${TDLIB_VERSION}" ] || \
   [ -f "${EXTRACT_TO}/lib/tdjson.lib" ]; then
  echo "TDLib already cached at ${EXTRACT_TO}"
  echo "Set: export LOCAL_TDLIB_PATH=${EXTRACT_TO}"
  exit 0
fi

echo "Downloading TDLib ${TDLIB_VERSION} for ${OS_NAME}-${ARCH_NAME}..."
mkdir -p "${CACHE_DIR}"
cd "${CACHE_DIR}"

# curl with retries and 5min timeout to avoid network issues
if ! curl -fSL --retry 3 --retry-delay 10 --connect-timeout 30 --max-time 300 \
  -o "${ZIP_NAME}" "${URL}"; then
  echo "Failed to download ${URL}"
  echo "Try manually: curl -fSL -o ${ZIP_NAME} '${URL}'"
  exit 1
fi

echo "Extracting..."
unzip -o -q "${ZIP_NAME}"
rm -f "${ZIP_NAME}"

# Zip may extract to tdlib-{version}-{os}-{arch}/ or tdlib/; normalize path
for dir in "tdlib-${TDLIB_VERSION}-${OS_NAME}-${ARCH_NAME}" "tdlib"; do
  if [ -d "$dir" ]; then
    rm -rf "${EXTRACT_TO}"
    mv "$dir" "${EXTRACT_TO}"
    break
  fi
done

if [ ! -d "${EXTRACT_TO}/lib" ]; then
  echo "Error: expected lib/ not found after extract"
  exit 1
fi

echo "TDLib cached at ${EXTRACT_TO}"
echo "Set: export LOCAL_TDLIB_PATH=${EXTRACT_TO}"
