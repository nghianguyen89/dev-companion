# Dev Companion Windows build

This repository builds the Windows-only Dev Companion desktop utility and its
selected Codex, Beyond Compare, SourceTree, and XAMPP file workflows.

Install Node.js, pnpm, Rust stable, Visual Studio C++ Build Tools and WebView2 (Tauri 2 prerequisites).
Run `./build-publish.ps1` in PowerShell. Missing frontend packages are installed with pnpm; SDK installation is explicit because it changes the machine.

Outputs:
- `release/portable/dev-companion.exe`, adjacent `portable-mode` and user guide.
- `src-tauri/target/release/bundle/nsis/Dev Companion_0.2.0_x64-setup.exe`.

The icon is embedded through `src-tauri/tauri.conf.json` and `icons/icon.ico`.
Portable configuration is always read/written next to the executable when the marker exists. Keep the folder writable.

Checks: `node node_modules/eslint/bin/eslint.js .`, `node node_modules/typescript/bin/tsc -b`, `node node_modules/vitest/vitest.mjs run`, `cargo test --manifest-path src-tauri/Cargo.toml --lib`.
