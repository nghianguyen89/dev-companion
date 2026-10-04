# Project context

## Current 0.2.0 contract

The Windows scope now includes environment archive v2, create-only file recovery and explicit catalog-cache cleanup. See [USER_GUIDE.md](USER_GUIDE.md) and [STORAGE_AUDIT.md](STORAGE_AUDIT.md) for the current included/excluded paths and limits. Desktop chat/database merging is **unfinished and unverified**. The milestone descriptions below are historical contracts, not the current feature scope.

Environment ZIP uses `environment-manifest.json` with version 2, kind, timestamp, platform, CLI version and entries (`path`, `group`, `bytes`, `sha256`, `manual`). Inspection checks the complete ZIP inventory and every SHA-256; automatic restore never writes manual database/index/settings components. Session `manifest.json` v1 remains supported; `delete-manifest.json` v1 now uses the existing inspection/restore UI. Missing destination sessions folders are supported, and failed rollback is reported.

New modules: `environment.rs` (offline Windows snapshot/preview/token/create-only recovery), `cleanup.rs` (strict remote catalog allowlist), `fs_safety.rs` (ancestor/reparse/path checks). Portable settings load/save use the same marker-selected location. UI changes invalidate previews and disable selection during operations. See [VALIDATION.md](VALIDATION.md) for actual checks.

## Historical milestones

## Purpose and philosophy

Dev Companion is a lightweight, local, Windows-first utility for selected developer tools. It complements rather than replaces Codex Desktop, Beyond Compare, SourceTree, or XAMPP: Dev Companion owns only its explicit local diagnostics, backup, and staging workflows. Safety, correctness, low idle cost, and portability take priority over convenience.

## Stack and platforms

Tauri 2, Rust, React, TypeScript, and Vite support Windows, macOS, and Linux from the same primary source tree. The app uses simple React state and small typed Tauri commands; it has no Redux, database, polling loop, or background worker.

## Architecture and integration

React calls typed commands through `src/services/tauri.ts`; Rust owns path resolution and Codex storage access. `CODEX_HOME` overrides the fallback `~/.codex`. On the investigated Windows installation, Codex CLI 0.153.0 stores legacy rollout sessions as `.jsonl` files below `sessions/YYYY/MM/DD/`; the first record is `session_meta` and contains IDs, timestamp, cwd, source, CLI version, and optional Git metadata. `session_index.jsonl` supplies an ID-to-thread-name mapping and update timestamp.

The Milestone 2 adapter reads only those metadata records. It never reads message records, writes Codex files, stores conversation content, or invokes mutating Codex CLI commands. A malformed or unsupported individual file is skipped and reported as a count.

## Safety, performance, and portable mode

All filesystem paths are centralized in `src-tauri/src/platform.rs`. Future writes must validate a known root, reject traversal, be explicitly confirmed, avoid silent overwrites, and provide a safety backup where applicable. Discovery runs on explicit UI refresh only; it performs no polling or idle disk activity. A `portable-mode` marker next to the executable makes Companion's own configuration and backups use adjacent `config/` and `backups/` folders; it never moves Codex data.

## Milestones and status

Milestone 5 adds a read-only restore-history view. The local history schema is versioned and retains the newest 100 entries. Each entry has only occurred time, archive filename (never its source path), requested session IDs, restored/skipped counts, optional safety-backup path, outcome (`completed`, `partial`, `rolledBack`, or `failed`), and a stable error code on failure. It never stores JSONL/session content, archive paths, inspection tokens, credentials, or native error text. History write failures are logged locally and never change the restore result.

The session adapter has an explicit compatibility contract: only legacy first-record JSONL `type: session_meta` records with `payload.session_id` or the observed `payload.id` alias are supported. Unknown record types or malformed metadata are counted as unsupported/skipped and cannot be selected, archived, or restored. Restore uses the same narrow validator for archived payloads; it does not infer a new Codex storage format.

## Important locations

- `src-tauri/src/platform.rs`: platform path resolution
- `src-tauri/src/codex.rs`: diagnostics and Codex-facing read-only logic
- `src-tauri/src/session_storage.rs`: session discovery adapter
- `src-tauri/src/commands.rs`: Tauri command boundary
- `src/features/conversations/ConversationsPage.tsx`: local list, filtering, sorting, and refresh UI
- `docs/ARCHITECTURE.md`: system design

## Next recommended milestone

The next phase should add a controlled restore-history view and compatibility fixtures for new Codex session-storage variants. It must remain opt-in, preserve the no-overwrite rule, and must not copy arbitrary archive entries.
# Milestone 6 — local deletion boundary

Deletion is intentionally limited to the documented legacy JSONL session contract below `CODEX_HOME/sessions`. Unsupported/new Codex storage formats, Codex Desktop chats, cloud chats, credentials, tokens, configuration, and every other Codex path are out of scope. The deletion preview exposes only safe metadata; conversation content is never rendered.
