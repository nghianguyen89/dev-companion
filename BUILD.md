# Dev Companion Windows build

This repository builds the Windows-only Dev Companion desktop utility and its
selected Codex, Beyond Compare, SourceTree, and XAMPP file workflows.

Install Node.js, pnpm, Rust stable, Visual Studio C++ Build Tools and WebView2 (Tauri 2 prerequisites).
Run `./build-publish.ps1` in PowerShell. Close any running `dev-companion.exe`
first because publishing replaces the portable executable. Missing frontend
packages are installed with pnpm; SDK installation is explicit because it
changes the machine.

Outputs:
- `release/portable/dev-companion.exe`, adjacent `portable-mode` and user guide.
- `src-tauri/target/release/bundle/nsis/Dev Companion_0.2.0_x64-setup.exe`.

The icon is embedded through `src-tauri/tauri.conf.json` and `icons/icon.ico`.
Keep the complete `release/portable` folder writable and together when moving it:

- `portable-mode` enables portable storage discovery.
- `config/`, `backups/`, and `quarantine/` are used when portable mode is
  enabled in Settings.
- `backup/` holds personal application bundles whenever the marker exists,
  including encrypted SourceTree configuration bundles.

## Debug a local build

```powershell
pnpm lint
pnpm check
pnpm test
pnpm build
cargo check --manifest-path src-tauri/Cargo.toml
$env:CODEX_COMPANION_LOG = 'debug'; pnpm tauri dev
```

Do not log or attach conversation content, bundle passwords, authentication
tokens, or credential files in an issue. See `docs/DEBUG_NOTES.md` for known
problems and `docs/VALIDATION.md` for verified checks.

Checks: `node node_modules/eslint/bin/eslint.js .`, `node node_modules/typescript/bin/tsc -b`, `node node_modules/vitest/vitest.mjs run`, `cargo test --manifest-path src-tauri/Cargo.toml --lib`.
