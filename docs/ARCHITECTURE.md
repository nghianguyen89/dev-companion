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
- `codex.rs`: read-only diagnostics, CLI version probes and direct-file backup inventory; it never reads archive contents.
- `codex_content.rs`: concrete local skills/pets adapter. It lists only direct
  supported metadata, imports/exports skills and installs pets with new-folder
  semantics, and removes only a confirmed direct pet after reparse-point and
  regular-file checks. It never reads auth/configuration or returns skill text.
- `codex_environment.rs`: concrete Windows-first metadata, CLI, launcher,
  current-user PATH, and managed-`AGENTS.md` adapter. It persists only safe
  environment metadata in settings; authentication stays in each CLI-managed
  `CODEX_HOME` and is never read, copied, logged, or returned to React.
- `session_storage.rs`: version-conscious, read-only legacy rollout-session adapter. It recursively locates `.jsonl` files below the resolved `sessions/` root, reads only each file's first JSON record, and supports only `type: session_meta` with `payload.session_id` or `payload.id`. Optional labels and update timestamps come from `session_index.jsonl`. It never reads message records or guesses a newer schema.
- `config.rs`: small JSON settings file, with safe defaults.
  Shared settings/history/logs use configs; settings load/save migrate legacy
  config entries with a complete metadata preflight, reject links/collisions,
  and remove only the emptied legacy directory. Windows MoveFileW preserves
  bytes without overwriting or requiring an NTFS hard-link capable drive.
  Old transfer-history log paths are remapped within the validated log root.
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

Portable mode requires a `portable-mode` marker file adjacent to the executable. When its setting is enabled and the marker is present, Companion stores its own config and all managed archives alongside the executable in `configs/` and `backups/`; personal application bundles share `backups/` whenever the marker is present so they travel with `dev-companion.exe`. The SourceTree configuration list may move only its own validated legacy ZIPs from `backup/` into `backups/`, never overwriting a name collision. It does not move or rewrite Codex data.

## Safety boundary

Restore never overwrites: existing destination files are reported as conflicts and skipped. Rust validates destination roots, relative paths and parent symlinks, then rolls back files created in a failed operation. React receives DTOs and never reads ZIPs or writes the filesystem. `src/i18n` provides typed, dependency-free UI dictionaries; settings writes update the in-memory language value immediately after the native save completes.

Restore history is Companion-owned storage, not Codex storage. Its format version is currently `1`; a malformed, symlinked, or future-version history file is unavailable rather than parsed speculatively. History persistence never contains session content, full archive source paths, tokens, credentials, or native error diagnostics.
# Milestone 6: deletion path

`ConversationsPage` sends only selected IDs and the literal confirmation. The Tauri command re-discovers and canonicalizes `CODEX_HOME/sessions`, accepts only regular non-symlink legacy files, snapshots exactly those files into a versioned quarantine ZIP plus `delete-manifest.json`, then verifies ZIP entries and metadata before calling `remove_file`. A mid-flight failure triggers strict create-new restoration from the verified ZIP; it never overwrites a concurrent file. The selected-session backup/restore screen is removed.

## Codex Environment Manager

`CodexEnvironmentsPage` calls narrow blocking-pool commands only. The implicit
Personal/Default environment remains externally managed. Custom metadata uses
the existing settings JSON; removing an entry never removes its home, launcher,
instructions, or CLI authentication. Launcher files live in `%USERPROFILE%\\bin`
and User PATH changes use `HKCU\\Environment` only. `AGENTS.md` edits replace a
single marker-delimited block after a timestamped backup, preserving user text
outside it; missing or malformed markers abort the change.

## Codex Migration

Inspection caches the strictly validated manifest with the ZIP digest. Preview
and restore recheck that digest under a held Windows read lock rather than
decompressing the unchanged ZIP repeatedly. Replacement safety discovery uses
known archive targets plus bounded chat trees/root databases; excluded runtime
trees are pruned before descent. Safety ZIPs use Stored entries and are fully
verified before deleting originals. Restore reports stages and actual file
counts through a per-invocation Tauri channel; unknown totals are indeterminate.
File restoration also reports the current relative filename and I/O substep.
Production restore enqueues these phase/count records (no file contents) for
`platform::app_data_dir()/logs/codex-restore-*.jsonl`. It creates the log/worker
before mutations; a separate worker owns only that log. The restore thread
serializes complete JSON/newline records and uses non-waiting `try_send` to a
1024-record queue. Full/disconnected queues discard diagnostics and count gaps
in the next accepted record's `droppedRecords`; disk errors stop only logging.
Restore/rollback/completion never join the diagnostic worker. A stalled worker
can remain until process exit, and trailing records can be delayed/lost.
Trace pairs distinguish entry to the native notification callback from its
return; producer timestamps do not promise disk/browser receipt. Incomplete
logs no longer indicate where native restore stopped. Path preparation marks checking,
mkdir and rechecking, with each ancestor's pre-metadata path in diagnostic-only
`check-path` events. A separate UI-only worker takes the latest progress every
100ms, releasing its state mutex before sending to the channel. A final update
can therefore reach the UI during an I/O wait without another native event;
channel sends and worker joins never hold the data worker. The frontend ignores
late channel events after the command settles and derives completion from its
result. The static restore inventory is memoized independently of progress.
Restore and rollback copy through a 64KiB buffer, retaining interrupted-read
retry, partial-write handling and ZIP end-of-stream validation. Native events
distinguish the next source read from destination write and report only bytes
whose write returned, alongside the manifest size. Those bytes are not verified
until the subsequent destination SHA check completes.
Destination files use normal OS writeback: copy, verify length and SHA through
the same read/write handle, then close, in both restore and rollback. Windows
sharing allows reads but denies competing writes/deletes until verification
finishes. Verification seeks to zero and reads exactly the expected byte count;
it does not reopen the destination or omit the actual-byte SHA check. Traces
identify size, seek, read and digest boundaries. Per-file forced disk sync was
removed after the target trace ended at that notification; a subsequent run also
stalled during post-close verification, so the reopen path was removed. Both ZIP
creation paths still sync and safety is strictly verified before deletions.
Matching destination hashes confirm readable bytes, not physical persistence
through sudden power loss; retain the source/safety archives until verified.

`codex_migration.rs` is a separate strict ZIP workflow for the current user's
direct `.codex` and `.codex-*` folders. It snapshots selected account,
project-local, and other durable-state components through narrow commands,
validates the complete SHA-256 inventory, and overwrites archived supported
files in matching folder names after `REPLACE CODEX` and a verified safety ZIP.
The chat component uses complete snapshot replacement to remove stale SQLite
WAL/SHM; other destination-only files are kept. Rollback tracks only removed
originals and attempts each even if another cannot be recovered. The legacy
create-only command mode remains readable but is no longer offered by the
migration page. It never merges SQLite or chooses files by timestamp. It never copies official CLI
authentication, machine identity, runtime cache or sandbox data.
The project-local component transfers `.chatgpt-projects` metadata,
instructions and sources only; it does not promise to recreate cloud project
state.
The archive list exposes only direct regular `codex-migration-<timestamp>.zip`
files in the Companion backup directory; its reveal/delete commands revalidate
that narrow filename and reject links or arbitrary paths.

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

`sourcetree_config.rs` is the separate direct-restore workflow for personal
SourceTree file settings. It accepts only the named local settings files
(`accounts.json`, `bookmarks.xml`, `customactions.xml`, `hostedaccounts.xml`,
`opentabs.xml`, `passwd`, `userhosts`, and the current `user.config`), reports
filenames and byte counts rather than contents, and verifies manifest,
inventory and hashes before restoring. New version-2 bundles encrypt every
entry with AES-256; passwords are command-only and never enter plans, settings,
manifests, logs, or history. Version-1 unencrypted bundles remain readable.
Existing files get a Companion-owned safety copy; every replaced target is
verified against the bundled SHA-256 and restore rolls back on a later write
or verification failure. Windows Vault, OAuth/DPAPI-bound secrets and SSH keys
remain out of scope because they are not portable file settings.
The configuration archive list exposes only direct regular
`sourcetree-config-<timestamp>.zip` files in the Companion bundle directory;
reveal and delete revalidate that narrow name and reject links or arbitrary
paths.

SourceTree configuration migration writes each bundled primary file to both
standard Local/Roaming `Atlassian/SourceTree` roots. The source profile in a v3
manifest is provenance, not destination authority; `user.config` is selected
independently for the installed target. Each physical target has a unique
safety-copy path and is hash-verified after replacement.

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

## XAMPP domain management — 2026-10-08

`xampp_domains.rs` is a separate concrete adapter next to the existing strict
XAMPP file-bundle workflow. Portable domain metadata/generated include/public
CA live in `configs/xampp/` beside `backups/`; installed builds use the same
subfolder under Companion AppData. The saved installation also supplies the
existing backup adapter's root. Validate binaries, htdocs and Apache ServerRoot.

Administrator-only apply manages domain CRUD, www aliases, HTTP redirects,
directory indexing, local/LAN access, Windows hosts and machine CA trust.
Private CA/leaf keys remain under `xampp/apache/conf/dev-companion/` with
restricted ACLs. Public-only CA export supports LAN clients; their DNS/hosts
and trust still need setup on each client. Program-scoped Private-profile
firewall access is limited to the local subnet when LAN domains are enabled.

Initialization adopts valid matching legacy CA material and simple HTTP/HTTPS
vhosts (matching document roots, optional www alias). Unsupported aliases or
conflicts abort rather than discard them. Archive identified legacy scripts,
CNF/cert/key files and changed configuration before any replacement. Safety
copies and `recovery.json` remain under `xampp/backup/dev-companion-*/`.
Generated includes and hosts use managed markers; unrelated mappings remain.
OpenSSL creates random serials/SANs and leaf lifetimes capped below CA expiry.

Apply checks Apache syntax, installs public machine trust, applies firewall,
and restarts only the executable matching this installation's PID through the
Windows Apache restart event. A fresh HTTP revision header confirms reload.
Stopped Apache is started. File/settings failures roll back; initial reload
recovery without a previous managed revision is explicitly unverified. Safety
archives remain. Existing bundle format, htdocs-only scope and stopped-process
requirement are unchanged; ordinary bundles exclude all private keys.

The domain list uses a focused XamppDomainEntry view with SSL/www indicators,
primary-URL open/copy, a folder link and tooltip-labelled listing/edit/delete
actions. CA readiness derives from initialized+caReady; each row's SSL indicator
and open/copy scheme also require its saved redirectHttps option. Disabling
redirect does not remove the backend TLS vhost/certificate. Copy uses the existing browser clipboard API with surfaced
errors; typed native open commands resolve only a saved, validated primary
domain. Folder opening rechecks filesystem safety/existence. Both use direct
Explorer arguments and the existing serialized blocking bridge, with no shell
command interpolation or new dependency. Opening does not modify XAMPP.

### XAMPP safety backup retention — 2026-10-08

Domain settings add backward-compatible backupKeepRecent (default 10, 1–100).
The domain overview includes validated recovery archive count/bytes and number
eligible for cleanup. Separate serialized commands save retention or manually
prune; they require Administrator and do not restart Apache.

New archives carry operation.json (version 1, owned kind/id, initial flag,
pending/completed/failed state). Automatic cleanup runs only after successful
live apply/restart and completed-status persistence. Initial/oldest, failed,
pending and legacy unknown-state archives remain protected. The newest N
eligible completed backups are retained in addition to these protected copies.
Malformed manifests, extra content and links/junctions are never deleted;
invalid archives are excluded from the UI's validated inventory. Cleanup
revalidates direct folder/manifest/file metadata and deletes exact flat files,
never original targets or recursively through directories. Cleanup failure
reports a warning without rolling back successfully applied configuration.
