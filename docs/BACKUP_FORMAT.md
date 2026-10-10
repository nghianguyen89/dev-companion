# Backup, safety-backup, and restore-history format

## Current 0.2.0 contract

The Windows scope now includes environment archive v2, create-only file recovery and explicit catalog-cache cleanup. See [USER_GUIDE.md](USER_GUIDE.md) and [STORAGE_AUDIT.md](STORAGE_AUDIT.md) for the current included/excluded paths and limits. Desktop chat/database merging is **unfinished and unverified**. The milestone descriptions below are historical contracts, not the current feature scope.

Environment ZIP uses `environment-manifest.json` with version 2, kind, timestamp, platform, CLI version and entries (`path`, `group`, `bytes`, `sha256`, `manual`). Inspection checks the complete ZIP inventory and every SHA-256; automatic restore never writes manual database/index/settings components. Session `manifest.json` v1 remains supported; `delete-manifest.json` v1 now uses the existing inspection/restore UI. Missing destination sessions folders are supported, and failed rollback is reported.

New modules: `environment.rs` (offline Windows snapshot/preview/token/create-only recovery), `cleanup.rs` (strict remote catalog allowlist), `fs_safety.rs` (ancestor/reparse/path checks). Portable settings load/save use the same marker-selected location. UI changes invalidate previews and disable selection during operations. See [VALIDATION.md](VALIDATION.md) for actual checks.

## Historical milestones

Backup archive v1 contains only explicitly selected session exports. Restore is limited to the validated workflow documented below; deletion, migration, cleanup, and all Codex-data changes outside that workflow are not implemented.

```text
codex-backup-YYYY-MM-DD_HHMMSS[-N].zip
├── manifest.json
├── sessions/
│   └── YYYY/MM/DD/rollout-*.jsonl
└── (no other Codex data)
```

`manifest.json` contains `formatVersion` (currently `1`), an RFC 3339 `createdAt`, source `platform`, `codexCliVersion` when the local read-only version probe succeeds, and `sessions`. Each session entry contains its ID, optional title and timestamps, ZIP-relative `archivePath`, and byte count. The archive contains `manifest.json` plus exactly those selected session files at their relative `sessions/` paths.

The destination is Companion's configured backup directory. A new filename is reserved with create-new semantics and an incrementing suffix if a same-second filename exists; an existing archive is never overwritten.

Before creation, the UI requests a Rust-generated preview showing the selected count, exact aggregate size, format version, and destination directory. The archive creator resolves selected IDs again from the supported discovery adapter rather than accepting a frontend path.

Restore requires a successful inspection token, a second validation immediately before preview/copy, and explicit session selection. Preview lists the canonical `CODEX_HOME/sessions` root, each destination, conflicts, total bytes, create-new operations, and safety-backup status. Only valid JSONL `session_meta` records whose IDs match the manifest can be copied. Existing files are skipped, never replaced; files created before a copy failure are rolled back.

When enabled and the execution-time restore snapshot finds conflicts, Companion creates a separate create-new archive named `codex-safety-backup-YYYY-MM-DD_HHMMSS[-N].zip` before it writes any session. It contains `safety-manifest.json` and only the conflicting regular files under `conflicts/sessions/...`; it never follows a conflict symlink. The manifest has `formatVersion` (currently `1`), RFC 3339 `createdAt`, `kind: "codex-companion-safety-backup"`, `reason: "pre-restore-conflicts"`, and each conflict's ID, archive path, and byte count. If the execution-time snapshot has no conflicts, no safety archive is created. A conflict created after that snapshot is skipped through create-new semantics and is never overwritten.

Restore reads and validates every selected entry fully before filesystem mutation. It uses an execution-time conflict snapshot rather than trusting the preview, rejects unsafe ZIP/path components and existing symlink parents, creates each destination with `create_new`, syncs it, and removes all files created by that operation if a later copy fails. Directory checks are re-run immediately before creation; an OS-level attacker who can replace a checked parent directory between those calls remains outside the guarantees of the standard-library implementation and is treated as a hostile local-machine limitation.

## Milestone 4 phase 1 inspection

Inspection is read-only and does not extract the ZIP. Before interpreting `manifest.json`, Companion rejects ZIP entry names containing absolute paths, drive prefixes, backslashes, empty or dot components, parent traversal, duplicates, or directory entries. The archive must contain one `manifest.json` and exactly the session files listed by manifest `sessions`; additional entries are invalid.

The manifest is strict JSON with no unknown fields. `formatVersion` must equal `1`; `createdAt` and optional session timestamps must be RFC 3339; `platform` and session IDs must not be empty; session `archivePath` values must be unique `sessions/.../*.jsonl` paths; and every listed byte count must equal the ZIP entry's uncompressed size. A missing `codexCliVersion` is reported as a warning, not an error. `manifest.json` is capped at 1 MiB during inspection.

## Compatibility contract and migration strategy

Backup archive format v1 is the only supported format. Existing v1 archives remain valid when their strict manifest and ZIP-entry checks pass. Any other `formatVersion`, including a hypothetical newer value, is shown as unsupported and cannot be restored; Companion does not silently downgrade, coerce, or migrate it. A future format requires a separately specified reader/migration, fixtures for every source version, and tests proving no-overwrite, traversal protection, symlink protection, and rollback still hold.

The embedded session payload compatibility contract is similarly narrow: its first JSONL line must be `type: "session_meta"` and identify the selected session by either `payload.session_id` or the known `payload.id` alias. Unrecognized record types and malformed metadata do not become a fallback format and cannot be restored.

## Restore history v1

Companion stores local restore audit data at `configs/restore-history-v1.json` (or the portable `configs/` directory). The file has `formatVersion: 1` and retains at most 100 newest-first entries. Each entry has `occurredAt`, archive **name** only, `sessionIds`, `restoredCount`, `skippedConflicts`, optional `safetyBackupPath`, `outcome` (`completed`, `partial`, `rolledBack`, `failed`), and optional stable `errorCode`.

History never includes session content, full archive source paths, inspection/restore tokens, credentials, or native error messages. A missing file means empty history. A malformed, symlinked, non-regular, or future-version file is unavailable rather than interpreted. The UI is read-only and does not provide deletion or editing.
# Delete safety archive v1

`codex-delete-safety-*.zip` is a local recovery artifact, separate from ordinary backup archives. It contains exactly `delete-manifest.json` and the selected `sessions/.../*.jsonl` entries. The manifest has `formatVersion: 1`, timestamp, fixed kind, and `{id, archivePath, bytes}` records. The archive is create-new and validated before deletion. To recover, select this ZIP in the session Backup & Restore inspection dialog, preview selected sessions and restore without overwrite. Archives are retained until manually removed.

## Personal bundle v1 — Beyond Compare

`personal-bundle-v1` is a separate ZIP format. It does not alter or embed the
existing `manifest.json`, `delete-manifest.json`, or `environment-manifest.json`
contracts. The only supported v1 inventory is exactly:

```text
personal-manifest.json
apps/beyond-compare/settings.bcpkg
```

The strict, unknown-field-rejecting manifest has `formatVersion: 1`,
`kind: "codex-companion-personal-bundle"`, RFC 3339 `createdAt`,
`platform: "windows"`, `sensitive: true`, and one artifact. That artifact is
fixed to `appId: "beyond-compare"`, adapter version `1`, kind
`"settings-package"`, the namespaced path above, `files: 1`, its byte count,
lowercase SHA-256, `restoreMode: "manual"`, and a boolean
`secretExportDisabledAcknowledged`. `true` records that the user said native
password/token export was disabled; `false` records that the user said the
opaque native export includes saved passwords or FTP/SSH credentials. The `sourceAppVersion` field is required
and must be explicitly `null`: the `.bcpkg` payload is opaque, so Companion
must not guess or parse a version.

Inspection validates the full ZIP inventory, strict timestamp and fixed
manifest values, case-insensitive duplicate names, regular-entry status, SHA-256
and declared byte/file counts. It rejects nested or extra entries, traversal,
absolute/drive/ADS/backslash names, Windows reserved names, trailing dots or
spaces, links, and unsupported/future variants. Limits are two ZIP entries,
1 MiB manifest, 512 MiB artifact, and 512 MiB aggregate artifact bytes.

Creation records the user's explicit credential choice. This cannot be proved
from the opaque package; the artifact and bundle remain sensitive. The bundle
is not password-protected, so use only trusted encrypted transport. There is
no cloud upload, license migration or automatic native import.

Recovery revalidates the inspection token, full bundle hash and ZIP content on
the same opened archive handle, then copies at most the declared artifact bytes
plus one before a fresh create-new Companion staging write. It rolls back only
a package it created if recovery fails and never overwrites a staged file. The
displayed staging path is a Companion destination, never the original source
path. The user completes `Tools > Import Settings` in Beyond Compare manually.

## Personal bundle v1 — SourceTree bookmarks

SourceTree uses a separate, concrete strict inventory. It does not change the
Beyond Compare bundle reader or any Codex archive format:

```text
personal-manifest.json
apps/sourcetree/bookmarks.xml
```

The strict manifest has the same fixed container fields (`formatVersion: 1`,
`kind: "codex-companion-personal-bundle"`, Windows platform, RFC 3339 creation
time and `sensitive: true`) and one SourceTree artifact. That artifact is fixed
to `appId: "sourcetree"`, adapter version `1`, kind `"bookmarks"`, the exact
namespaced path above, `files: 1`, SHA-256, byte count, `restoreMode: "manual"`,
the detected SourceTree version, and the parsed repository-path inventory.
Unknown fields, a different app/version/path, duplicate/case-colliding entries,
links, traversal, ADS, reserved names, hash/size mismatch, or an inventory that
does not exactly match the parsed XML are rejected.

Only the fixture-proven `ArrayOfBookmark > Bookmark > Name + Path` XML shape is
accepted. Attributes, unknown/future fields, malformed XML, unsafe paths, and
URLs containing userinfo are rejected. The original XML is never placed in the
SourceTree configuration location automatically: recovery is a create-new
Companion staging write with rollback on failure. SourceTree placement is
manual, even when the destination is absent; an existing destination is a
manual-review conflict.

## Personal bundle v1 — XAMPP files

The XAMPP bundle is a separate strict `personal-bundle-v1` variant. Its exact
ZIP inventory is `personal-manifest.json` plus manifest-listed regular files
only under `apps/xampp/htdocs/<selected-project>/<relative-file>` and these
configuration paths: `apps/xampp/config/apache/conf/httpd.conf`,
`apps/xampp/config/apache/conf/extra/httpd-vhosts.conf`,
`apps/xampp/config/php/php.ini`, and `apps/xampp/config/mysql/bin/my.ini`.

The strict one-artifact manifest records XAMPP version/architecture, manual
restore mode, aggregate count/bytes, and `{ archivePath, bytes, sha256,
sourceKind }` for every file. Unknown fields, alternate paths/configuration,
duplicate/case-colliding ZIP names, links, traversal, ADS, reserved names, and
count/size/hash mismatches are rejected. Limits are 20,000 files, 64 MiB per
project file, 1 MiB per reviewed configuration file, 512 MiB aggregate, and
4 MiB manifest.

Only direct user-selected `htdocs` projects are eligible. Binaries, MariaDB
data, credentials, keys, logs, caches, repository metadata, and reparse points
are not archived. Recovery revalidates token, full ZIP hash/inventory,
compatible XAMPP version/architecture and destination state, then extracts
create-new only to Companion staging with rollback. There is no overwrite,
automatic placement, XAMPP import, or MariaDB import.
