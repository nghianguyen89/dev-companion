# Dev Companion development notes

## Current 0.2.0 contract

The Windows scope includes Codex environment archive v2, create-only file recovery, explicit catalog-cache cleanup, and concrete personal adapters: Beyond Compare `.bcpkg`, SourceTree bookmarks/full encrypted local configuration, and selected XAMPP files/configuration. See [USER_GUIDE.md](USER_GUIDE.md) and [STORAGE_AUDIT.md](STORAGE_AUDIT.md) for current included/excluded paths and limits. Desktop chat/database merging is **unfinished and unverified**.

Environment ZIP uses `environment-manifest.json` with version 2, kind, timestamp, platform, CLI version and entries (`path`, `group`, `bytes`, `sha256`, `manual`). Inspection checks the complete ZIP inventory and every SHA-256; automatic restore never writes manual database/index/settings components. Session `manifest.json` v1 remains supported; `delete-manifest.json` v1 now uses the existing inspection/restore UI. Missing destination sessions folders are supported, and failed rollback is reported.

New modules: `environment.rs` (offline Windows snapshot/preview/token/create-only recovery), `cleanup.rs` (strict remote catalog allowlist), `fs_safety.rs` (ancestor/reparse/path checks). Portable settings load/save use the same marker-selected location. UI changes invalidate previews and disable selection during operations. See [VALIDATION.md](VALIDATION.md) for actual checks.

## Prerequisites

Install a current Node.js release, pnpm, Rust stable, and the official Tauri 2 prerequisites for your platform. Windows additionally needs the Microsoft C++ build tools and WebView2 runtime.

## Commands

```sh
pnpm install
pnpm lint
pnpm check
pnpm test
pnpm build
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
```

`pnpm tauri dev` starts Vite and the native application. No watcher is started by production builds or while the application is idle. On a Windows machine with Tauri prerequisites available, additionally run `pnpm tauri build` to validate the native bundle.

## Restore, history, compatibility, and i18n checks

Rust synthetic fixture tests cover valid restore, skipped conflicts, invalid archives, traversal, rollback, v1 archive compatibility, legacy `session_meta` ID variants, and rejection of malformed/future metadata. Restore-history tests cover bounded storage and refusal to read an unsupported history version. Frontend tests cover English default, Vietnamese selection, typed interpolation, and missing-key fallback. UI strings belong in `src/i18n/en.ts` and `src/i18n/vi.ts`; use typed keys rather than component-local copy. Do not translate Codex data, manifests, session contents, or native error diagnostics.

History is a read-only view of Companion-owned metadata. Never add session JSONL, full archive source paths, inspection tokens, credentials, or native error strings to its DTO or file. It retains 100 entries; schema changes require an explicit format-version strategy and fixtures.

## Local discovery investigation

On the initial Windows development machine, `codex --version` reported `codex-cli 0.153.0`. The CLI exposes `doctor`, `archive`, `delete`, and session-related commands, but Companion invokes none of its mutating commands.

The local home contained 167 legacy rollout files at `sessions/YYYY/MM/DD/rollout-*.jsonl`. Every inspected file began with a `session_meta` JSON record. Its payload included `session_id`/`id`, `timestamp`, `cwd`, `source`, `cli_version`, and Git metadata; subsequent JSONL records contain conversation data and are deliberately not read. `session_index.jsonl` uses `id`, `thread_name`, and `updated_at`; Companion uses it only for optional title and update metadata. This is an observed compatibility target, not a promise that every Codex release will keep the format.

## Logging

Set `CODEX_COMPANION_LOG=info` or `debug` only while diagnosing. Do not log conversation content, credentials, or auth tokens.

## First-response debug checklist

1. Run `pnpm check`, `pnpm lint`, `pnpm test`, and `cargo check --manifest-path src-tauri/Cargo.toml` from the repository root.
2. For UI/native reproduction, use `$env:CODEX_COMPANION_LOG = 'debug'; pnpm tauri dev` in the same PowerShell session.
3. For a generated release, run `./build-publish.ps1` only after closing the running portable executable. It rebuilds the portable EXE and NSIS installer.
4. Record the command, exact error, Windows version, and whether the run was portable or installed. Never include archive passwords, session contents, OAuth/token material, or credential files.

If pnpm cannot read its per-user AppData configuration, retry from a normal user
PowerShell after checking that no security tool is locking that file; do not
delete pnpm configuration as a first response. `docs/DEBUG_NOTES.md` records
known regressions and their verified fixes.
# Milestone 6 validation notes

Deletion tests cover legacy metadata aliases, malformed/unsupported metadata, duplicate/missing selections, archive naming collisions, and injected delete rollback. The security contract is local-only legacy sessions: no speculative support for newer Codex storage formats, no cloud/Desktop deletion, no arbitrary filesystem paths, no symlink following, and no overwrite during recovery.
