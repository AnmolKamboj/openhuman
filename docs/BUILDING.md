# Building & Installing OpenHuman

This guide covers two paths:

1. Build and compile OpenHuman from source
2. Install the latest stable release binaries

## Prerequisites

- `git`
- `node` + `yarn`
- Rust toolchain (see `rust-toolchain.toml`)

## Build from source (local compile)

Run from the repository root:

```bash
# 1) Clone and enter the repo
git clone https://github.com/tinyhumansai/openhuman.git
cd openhuman

# 2) Install JS deps (workspace)
yarn install

# 3) Build Rust core binary
cargo build --manifest-path Cargo.toml --bin openhuman

# 4) Stage core sidecar for the desktop app
cd app
yarn core:stage

# 5) Build desktop app artifacts
yarn build
```

For local development instead of production build:

```bash
yarn dev
```

## Install latest stable release (macOS/Linux)

Use this basic script to detect platform/arch and download the latest stable artifact from GitHub Releases:

```bash
#!/usr/bin/env bash
set -euo pipefail

REPO="tinyhumansai/openhuman"
BASE="https://github.com/${REPO}/releases/latest/download"
OS="$(uname -s)"
ARCH="$(uname -m)"

if [[ "$OS" == "Darwin" ]]; then
  if [[ "$ARCH" == "arm64" ]]; then
    FILE="OpenHuman_0.49.32_aarch64.dmg"
  else
    FILE="OpenHuman_0.49.32_x64.dmg"
  fi
elif [[ "$OS" == "Linux" ]]; then
  # Prefer AppImage for broad compatibility.
  FILE="OpenHuman_0.49.32_amd64.AppImage"
else
  echo "Unsupported OS: $OS"
  exit 1
fi

URL="${BASE}/${FILE}"
echo "Downloading: $URL"
curl -fL "$URL" -o "$FILE"

echo "Downloaded $FILE"
echo "Install it with your platform's normal installer flow."
```

Notes:

- The filename includes the release version and should be updated when a new stable release is cut.
- You can always manually download from:
  - Website: https://tinyhuman.ai/openhuman
  - Latest release: https://github.com/tinyhumansai/openhuman/releases/latest

## Windows (latest stable)

Download directly from the website or latest release page:

- https://tinyhuman.ai/openhuman
- https://github.com/tinyhumansai/openhuman/releases/latest

## Future improvements

- Replace hardcoded filenames with `latest.json` parsing
- Add checksum/signature verification
- Publish one-step global installers for all platforms
