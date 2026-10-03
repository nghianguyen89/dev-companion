# Architecture

## Current 0.2.0 contract

The Windows scope now includes environment archive v2, create-only file recovery and explicit catalog-cache cleanup. See [USER_GUIDE.md](USER_GUIDE.md) and [STORAGE_AUDIT.md](STORAGE_AUDIT.md) for the current included/excluded paths and limits. Desktop chat/database merging is **unfinished and unverified**. The milestone descriptions below are historical contracts, not the current feature scope.

Environment ZIP uses `environment-manifest.json` with version 2, kind, timestamp, platform, CLI version and entries (`path`, `group`, `bytes`, `sha256`, `manual`). Inspection checks the complete ZIP inventory and every SHA-256; automatic restore never writes manual database/index/settings components. Session `manifest.json` v1 remains supported; `delete-manifest.json` v1 now uses the existing inspection/restore UI. Missing destination sessions folders are supported, and failed rollback is reported.

New modules: `environment.rs` (offline Windows snapshot/preview/token/create-only recovery), `cleanup.rs` (strict remote catalog allowlist), `fs_safety.rs` (ancestor/reparse/path checks). Portable settings load/save use the same marker-selected location. UI changes invalidate previews and disable selection during operations. See [VALIDATION.md](VALIDATION.md) for actual checks.

## Historical milestones

Dev Companion is split into a React view layer and a Tauri/Rust boundary. The frontend never directly reads managed application filesystems. It invokes narrow Rust commands which return small, serializable view models.

```text
React features -> services/tauri.ts -> Tauri commands -> codex/session_storage/platform/config modules -> local filesystem
```

## Frontend

- `src/app`: application shell and global, dependency-free styles.
- `src/features`: independently owned pages for dashboard, conversations, backup, skills, pets, diagnostics, and settings.
- `src/services`: typed bridge to Tauri commands.
- `src/types`: stable DTOs shared by UI-facing service calls.
- `src/features/file-transfer`: the editable Robocopy form and profiles; it
  receives streamed output/completion events but never starts a process itself.

## Native layer

- `platform.rs`: the only place that resolves OS-dependent paths. `CODEX_HOME` overrides the default `~/.codex` location.
- `codex.rs`: read-only diagnostics and CLI version probes.
- `codex_content.rs`: concrete local skills/pets adapter. It lists only direct
  supported metadata, imports/exports skills and installs pets with new-folder
  semantics, and removes only a confirmed direct pet after reparse-point and
  regular-file checks. It never reads auth/configuration or returns skill text.
- `codex_environment.rs`: concrete Windows-first metadata, CLI, launcher,
  current-user PATH, and managed-`AGENTS.md` adapter. It persists only safe
  environment metadata in settings; authentication stays in each CLI-managed
  `CODEX_HOME` and is never read, copied, logged, or returned to React.
- `session_storage.rs`: version-conscious, read-only legacy rollout-session adapter. It recursively locates `.jsonl` files below the resolved `sessions/` root, reads only each file's first JSON record, and supports only `type: session_meta` with `payload.session_id` or `payload.id`. Optional labels and update timestamps come from `session_index.jsonl`. It never reads message records or guesses a newer schema.
- `backup.rs`: export, inspection, and restore boundary. Restore accepts an opaque token from a successful inspection, revalidates the ZIP immediately before preview and copying, allows only explicitly selected manifest sessions, verifies the supported JSONL metadata variant and matching ID, re-snapshots conflicts at execution, emits structured `{ code, message }` failures, and writes under canonical `CODEX_HOME/sessions` through create-new semantics with rollback. Safety backups are independently versioned ZIPs with their own manifest.
- `restore_history.rs`: a local, read-only audit store at `config/restore-history-v1.json`. It keeps newest-first, capped history DTOs and returns only safe summary fields to React.
- `config.rs`: small JSON settings file, with safe defaults.
- `commands.rs`: the allowlisted interface available to the frontend. Its async
  wrappers send filesystem, process and dialog work to Tauri's blocking pool.
  One gate preserves sequential native operations while keeping the window
  thread responsive; existing commands and DTOs remain unchanged.
- `logging.rs`: warning-level output by default; no periodic writer or background worker.
- `file_transfer.rs`: one concrete direct-`robocopy.exe` adapter. It owns command
  arguments, source direct-child selection exclusions, path/mirror validation,
  output-summary parsing, exit interpretation, a single active child process,
  local logs/history, and dry-run verification.
  Cancellation bypasses the serialized command gate so it can reach an active
  transfer; no pause/resume state is simulated.

The conversations page creates one date formatter per language. Filtering,
selection and refresh reuse it instead of constructing one formatter per date
cell during every table render.

## Portable mode

Portable mode requires a `portable-mode` marker file adjacent to the executable. When enabled in settings and the marker is present, Companion stores its own config and backups alongside the executable in `config/` and `backups/`. It does not move or rewrite Codex data.

## Safety boundary

Restore never overwrites: existing destination files are reported as conflicts and skipped. Rust validates destination roots, relative paths and parent symlinks, then rolls back files created in a failed operation. React receives DTOs and never reads ZIPs or writes the filesystem. `src/i18n` provides typed, dependency-free UI dictionaries; settings writes update the in-memory language value immediately after the native save completes.

Restore history is Companion-owned storage, not Codex storage. Its format version is currently `1`; a malformed, symlinked, or future-version history file is unavailable rather than parsed speculatively. History persistence never contains session content, full archive source paths, tokens, credentials, or native error diagnostics.
# Milestone 6: deletion path

`ConversationsPage` sends only selected IDs and the literal confirmation. The Tauri command re-discovers and canonicalizes `CODEX_HOME/sessions`, accepts only regular non-symlink legacy files, snapshots exactly those files into a versioned quarantine ZIP plus `delete-manifest.json`, then verifies ZIP entries and metadata before calling `remove_file`. A mid-flight failure triggers strict create-new restoration from the verified ZIP; it never overwrites a concurrent file.

## Codex Environment Manager

`CodexEnvironmentsPage` calls narrow blocking-pool commands only. The implicit
Personal/Default environment remains externally managed. Custom metadata uses
the existing settings JSON; removing an entry never removes its home, launcher,
instructions, or CLI authentication. Launcher files live in `%USERPROFILE%\\bin`
and User PATH changes use `HKCU\\Environment` only. `AGENTS.md` edits replace a
single marker-delimited block after a timestamped backup, preserving user text
outside it; missing or malformed markers abort the change.

## Phase 0–1 personal bundle boundary

`personal_bundle.rs` is a small, concrete strict ZIP boundary; it has no
provider trait or catalog. `beyond_compare.rs` accepts only a regular Windows
`.bcpkg` selected by the user and treats its bytes as opaque. Narrow commands
create/inspect/recover this one workflow, while `BeyondComparePage` provides
the single explicit UI card. `platform.rs` owns the separate Companion bundle
and staging locations; no app directory is written.

The existing Codex backup, deletion, environment commands, DTOs and manifest
readers are unchanged. Personal bundles have no operation-history integration.

The product is branded Dev Companion, while `platform.rs` deliberately retains
the `codex-companion` application-data directory and all established archive
kinds for backward compatibility. A future app is added as another concrete
Rust module, card, commands, DTOs, fixtures and documentation; no generic
provider layer is introduced until real shared behavior proves necessary.

## Phase 2 SourceTree boundary

`sourcetree.rs` is a second concrete personal-bundle-v1 reader/writer, separate
from the opaque Beyond Compare package module. It supports Windows only and
only `%LOCALAPPDATA%\\Atlassian\\SourceTree\\bookmarks.xml` with the one
fixture-proven `ArrayOfBookmark > Bookmark > Name + Path` schema. It validates
the source before preview and again before creation, records the detected
installed SourceTree version, hashes the one namespaced XML artifact, and
rejects unknown XML, credentials in URL userinfo, unsafe values, reparse points,
and unsafe ZIP inventories.

SourceTree must be closed for preview, creation, and recovery; Companion only
checks `tasklist.exe` and never terminates it. Recovery always writes a fresh
`bookmarks.xml` to Companion-owned staging after token, archive hash/inventory,
and destination-state revalidation. The regular SourceTree location is shown as
a conflict/manual-placement preview only; Companion never writes or imports it.

## Phase 3 XAMPP files boundary

`xampp.rs` is a third concrete `personal-bundle-v1` reader/writer. It is
Windows-only and accepts only explicitly selected direct children of the
detected XAMPP `htdocs` directory plus four reviewed UTF-8 configuration files:
`apache/conf/httpd.conf`, `apache/conf/extra/httpd-vhosts.conf`, `php/php.ini`,
and `mysql/bin/my.ini`. It detects XAMPP version and architecture from the
fixture-proven release-notes header; XAMPP binaries and MariaDB data are never
read into the bundle.

Apache, MariaDB, and XAMPP-related processes must be stopped for preview,
creation, and recovery; Companion only checks `tasklist.exe`. The strict
inventory hashes every file and rejects unsafe paths, reparse points, unknown
configuration, credential/key indicators in configuration, and ZIP inventory
or size/count/hash mismatches. Known logs, caches, repository metadata and
credential/key file names are excluded. Recovery validates the token, archive
hash/inventory, matching destination version/architecture, and destination
state, then writes create-new files only to Companion-owned staging with
rollback. XAMPP placement and MariaDB import remain manual.
