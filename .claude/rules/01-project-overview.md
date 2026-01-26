# Project Overview

## Tauri Cross-Platform Application

This project is a Tauri v2 application designed to run on multiple platforms:

- **Windows** (Desktop)
- **macOS** (Desktop)
- **Android** (Mobile)
- **iOS** (Mobile)

## Technology Stack

| Layer | Technology | Version |
|-------|------------|---------|
| Frontend | React | 19.1.0 |
| Language | TypeScript | 5.8.3 |
| Build Tool | Vite | 7.0.4 |
| Backend | Rust | 1.93.0 |
| Framework | Tauri | 2.x |

## Project Structure

```
tauri-crossplatform-app/
├── .claude/                # Claude AI configuration
│   ├── rules/              # Modular documentation
│   └── agents/             # Subagent configurations
├── src/                    # React frontend source
├── src-tauri/              # Rust backend source
│   ├── gen/                # Generated platform code
│   │   ├── android/        # Android project
│   │   └── apple/          # iOS/macOS project
│   ├── icons/              # Application icons
│   └── src/                # Rust source code
├── public/                 # Static assets
└── dist/                   # Build output
```

## Key Configuration Files

- `tauri.conf.json` - Tauri configuration
- `Cargo.toml` - Rust dependencies
- `package.json` - Node.js dependencies
- `vite.config.ts` - Vite build configuration
- `tsconfig.json` - TypeScript configuration
