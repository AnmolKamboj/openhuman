# Tauri sidecar binaries

The app bundles a Python interpreter as a sidecar for skill runtimes (`runtime-skill-python`). The binary lives in `src-tauri/` (next to `tauri.conf.json`) and must be named by target triple.

## Required file

- **Python**: `src-tauri/runtime-skill-python-<TARGET_TRIPLE>` (e.g. `runtime-skill-python-aarch64-apple-darwin`, or `runtime-skill-python-x86_64-pc-windows-msvc.exe` on Windows)

Get your target triple:

```bash
rustc --print host-tuple
```

## Local development (symlink to system Python)

From the project root:

```bash
node scripts/setup-python-sidecar.mjs
```

This creates a symlink at `src-tauri/runtime-skill-python-<target>` pointing to your system `python3`.

## Production / bundled Python

For a standalone app that does not rely on the user having Python installed:

1. **Windows**: Use the [Python embeddable package](https://www.python.org/downloads/windows/). Copy `python.exe` to `src-tauri/runtime-skill-python-x86_64-pc-windows-msvc.exe` (and the same for `aarch64-pc-windows-msvc` if you support ARM). You may need to bundle the rest of the embeddable folder as resources and set `PYTHONHOME` when spawning.

2. **macOS / Linux**: Use [python-build-standalone](https://github.com/astral-sh/python-build-standalone/releases). Copy the `bin/python3` from the extracted tree to `src-tauri/runtime-skill-python-<target>`. Note: the binary may have dynamic library dependencies; for a fully portable build you may need to bundle the whole distribution as resources.

After adding the binary, run `yarn tauri build` (or `yarn tauri dev`); the sidecar will be included in the bundle.
