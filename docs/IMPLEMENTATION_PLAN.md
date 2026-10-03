# Implementation Plan — Personal application migration

## 2026-10-03: Local Codex skills and pets

- Replace the two Milestone 4 placeholders with concrete local workflows; do
  not add a generic extension/package framework or network catalog.
- Skills: list only direct, valid `CODEX_HOME/skills/<name>/SKILL.md` metadata;
  import a user-selected skill folder and export a selected installed skill to
  a user-selected folder. Both operations create a new directory only.
- Pets: list only validated v2 `CODEX_HOME/pets/<id>` manifests; install the
  contract's `pet.json` and PNG/WebP sprite as a new directory, and remove a
  selected direct child only after the literal `REMOVE` confirmation.
- All filesystem work remains in Rust behind narrow commands. Reject unsafe
  names, symbolic links/junctions and non-regular files; do not read Codex
  auth/configuration, display skill bodies, overwrite a target, or delete
  anything outside the selected direct pet directory.
- Copy into a sibling temporary directory and rename only after success. Add
  focused Rust contract tests plus frontend rendering tests; validate with
  TypeScript checks, tests, Rust tests and diff whitespace checks.

## 2026-10-03: File Compression

- Add a dedicated `compression.rs` adapter and File Compression page. Do not
  extend `file_transfer.rs`: Robocopy copies files but cannot create an archive.
- Use direct hidden `7z.exe` execution only. Detect the executable from the
  standard 7-Zip installation path and `PATH`; when absent, return a clear
  install-required readiness state without attempting a download.
- Load the selected source as a recursive, read-only tree. Users can exclude
  individual relative files/folders and provide Rust-regex patterns evaluated
  against slash-normalized source-relative paths. Reparse points are skipped.
- Produce a `.7z` archive in the selected output folder, defaulting to the
  source folder's parent. Fast archival uses `-mx=1`; maximum compression uses
  `-mx=9`. Refuse to overwrite an existing archive.
- Keep the process, selection validation, temporary list file, output events,
  cancellation, command preview and tests inside the new module. No generic
  archive framework, Robocopy staging phase, profiles, or transfer-history
  integration is needed for this slice.
- Render canonical Windows paths without the internal `\\?\` prefix. For large
  source trees, show an honest scanning/preparation state and render a filtered,
  bounded result list. Surface a percentage only when 7-Zip emits one through
  `-bsp1`; otherwise retain indeterminate progress.

## 2026-10-03: SourceTree personal configuration bundle

- Replace the bookmark-only SourceTree flow with one concrete, sensitive bundle
  of `accounts.json`, `bookmarks.xml`, `customactions.xml`, and the current
  SourceTree `user.config` when each is present as a regular file. Do not read
  their contents into the UI or logs.
- Keep Windows Credential Manager, OAuth tokens, SSH keys and SourceTree/Git
  passwords outside the bundle: they are managed by Windows and may be DPAPI
  bound to the previous machine. The UI must state that accounts may still need
  a sign-in after restoration.
- Require SourceTree to be closed. Preview hashes and counts the exact detected
  files; create revalidates before making a create-new ZIP. Inspect rejects
  unknown, missing, duplicate, linked or hash-mismatched entries.
- Restore only after an explicit `RESTORE` confirmation. Save an internal
  safety copy of an existing target before replacing it, then rollback newly
  restored files if a later write fails. Add a separate explicit `DELETE`
  action for an inspected bundle after a successful restore.

## 2026-10-03: SourceTree full personal migration

- Extend the concrete SourceTree allowlist to include detected `opentabs.xml`,
  `hostedaccounts.xml`, `userhosts`, and `passwd` alongside the existing
  configuration files. Treat every file as opaque: return only its name and
  byte count to the UI and never log its contents.
- New bundles use a versioned AES-256 encrypted ZIP. Require a user-entered
  password for create, inspect, recovery preview, and restore; keep it only in
  the active command call, never in the manifest, plan token, settings, logs,
  or history. Continue to accept existing version-1 unencrypted bundles so
  earlier backups remain recoverable.
- Maintain the closed-SourceTree gate, revalidation, strict inventory/hash
  checks, direct restore with safety copy/rollback, and explicit `RESTORE` /
  `DELETE` confirmations. This copies legacy local credential files but cannot
  migrate Windows Credential Manager, OAuth/DPAPI-bound secrets, or SSH keys.

## 2026-09-25: File Transfer

- Add one concrete `file_transfer.rs` adapter: pure command construction,
  validation, output-summary parsing, Robocopy exit interpretation, portable
  logs/history, and one managed background process. Do not add a generic
  process or transfer framework.
- The React page owns the editable form and profiles in the existing settings
  file. Narrow Tauri commands own folder selection, Robocopy availability,
  execution, cancellation, logs, and portable history.
- Use direct `robocopy.exe` arguments, never a command shell. Mirror and
  system-location warnings require confirmation in both UI and native code.
- Browse source/destination with direct-child Explorer lists, breadcrumb/Up
  navigation and explicit hidden/system markers. Source begins with no
  selection; the native command builder excludes every unselected sibling,
  including hidden/system items, before Robocopy starts.
- Start runs a cancellable `/L` analysis first. Progress is determinate only
  when its byte total and guarded English completed-file output are both
  available; otherwise the UI remains indeterminate.

## 2026-09-17: Codex Environment Manager

- Add one concrete Windows-first `codex_environment.rs` adapter rather than a
  generic service hierarchy. It owns safe environment metadata, Codex CLI
  probes/actions, launcher generation, current-user PATH inspection/update and
  managed `AGENTS.md` block replacement.
- Persist only environment metadata in the existing settings JSON. Never read,
  copy, serialize or log Codex authentication data. The default `.codex`
  environment is an implicit externally-managed entry unless management is
  explicitly enabled.
- Keep process and registry work behind narrow Tauri commands on the existing
  blocking gate. Windows-only actions fail clearly on other platforms.
- Use create/update/delete metadata semantics only: removing an entry never
  removes its `CODEX_HOME`; a launcher removal is an explicit separate action.
- Cover validation, launch-script argument forwarding, User PATH deduplication,
  managed-instruction preservation/backups and metadata CRUD with focused unit
  tests. Defer PowerShell profile migration: it is optional and needs a
  separately scoped AST-backed migration contract.

## 2026-09-09: repository rename and window responsiveness

- Update current repository descriptions for Dev Companion's implemented Windows
  workflows; retain legacy data directories and archive identifiers.
- Startup diagnostics and menu-triggered discovery/history currently execute as
  synchronous Tauri commands on the window thread. Blocking native file dialogs
  use the same path, contrary to the dialog plugin's threading requirement.
- Move command bodies to Tauri's blocking worker pool through async commands.
  Preserve serialized filesystem operations with one async gate, held inside the
  worker until completion (including if its caller goes away). Keep command
  names, arguments, success DTOs and structured backup errors compatible.
- Verify worker-thread execution, serialization, panic/error propagation and
  existing archive regressions; rebuild Windows NSIS and portable artifacts.
- The reported trigger is Conversations filter changes or Refresh. Reuse one
  date formatter per language to remove repeated expensive construction during
  table rerenders; verify with a 2,000-session fixture and packaged WebView.
- Recreate pnpm junctions and affected Tauri build caches after the checkout
  rename; both retained absolute paths to the former repository directory.

## Task

Assess and plan a safe expansion from Codex-only backup into a personal,
Windows-first backup and migration assistant for Codex, Beyond Compare,
SourceTree, and XAMPP.

## Feasibility decision

Feasible as a curated set of application-specific migration adapters. It is not
feasible or safe to promise arbitrary-folder backup, transparent two-way sync,
credential migration, or blind restoration of live databases.

| Application | Feasibility | First safe boundary |
|---|---|---|
| Codex | Existing | Preserve the current strict v1/v2 workflows |
| Beyond Compare | High | Package a user-created `.bcpkg`; native import remains manual |
| SourceTree | Medium | Closed-app backup of allowlisted non-secret settings; repositories handled separately |
| XAMPP | Medium | Selected projects/config plus logical MariaDB dumps; never copy a running data directory |

## Scope

### In scope

- Windows-first, explicit preview and manual execution.
- One versioned personal bundle containing verified, app-labelled artifacts.
- Per-application allowlists, process checks, exclusions, restore policy and
  compatibility metadata.
- SHA-256 verification, strict archive inventory, create-new/no-overwrite
  restore and rollback where the tool writes files.
- Codex compatibility without changing existing archive readers.
- Beyond Compare native settings packages may include saved passwords and
  FTP/SSH credentials only after explicit user selection; the opaque package
  and containing bundle are always treated as sensitive.
- Parsed, fixture-proven SourceTree bookmarks without account credentials.
- XAMPP selected `htdocs` projects, reviewed text configuration and logical SQL dumps.

### Out of scope

- Background service, scheduling, incremental/deduplicated backup or two-way sync.
- Cloud upload, unencrypted transport or automatic home/work conflict resolution.
- Arbitrary directory backup or restore.
- OS credential stores, OAuth token transfer, SSH/private keys, Windows
  Credential Manager, license keys, machine identity or certificate private keys.
- Raw copy/merge of a running MariaDB data directory.
- SourceTree repository contents by default; pushed repositories should be
  re-cloned, while repositories with local-only work require a separate explicit flow.
- Reinstalling application binaries or guaranteeing cross-version compatibility.

## Current understanding

- `environment.rs` already proves the required safety sequence: allowlisted
  inventory, preview token, source revalidation, locked reads, SHA-256, strict
  ZIP inspection, create-new restore and rollback.
- The reusable safety rules exist, but the root, groups, process names, archive
  kind and restore paths are hard-coded to Codex.
- The current machine has XAMPP 8.2.12 and SourceTree data. Apache, MariaDB and
  SourceTree were running during this read-only assessment, confirming that
  process-aware snapshot policy is required.
- XAMPP database migration needs a logical export/import path. A byte copy of
  live database files is not a valid migration strategy.
- Beyond Compare already supplies the best portability contract through its
  native `.bcpkg` export/import workflow.

## Proposed architecture

Keep the existing Codex modules and formats intact. A personal bundle embeds an
unchanged Codex v1/v2 archive and delegates recovery to the existing Codex
inspect, preview and execute commands. Add a small personal-bundle orchestrator
and concrete app modules one at a time. Do not introduce a public plugin SDK or
a trait hierarchy until two adapters genuinely share behavior.

```text
React migration screen
  -> narrow Tauri preview/create/inspect/restore commands
    -> personal_bundle.rs (manifest, hashing, ZIP validation only)
      -> codex existing commands
      -> beyond_compare.rs
      -> sourcetree.rs
      -> xampp.rs
        -> fs_safety.rs + Companion-owned backup/staging locations
```

Personal bundle v1 should contain a strict top-level manifest with
`formatVersion`, fixed `kind`, creation time, platform, and namespaced
app-labelled artifacts. Each item records app ID, adapter format version,
source app version, artifact kind/logical target, relative archive path, file
count, bytes, SHA-256 and restore mode (`automatic` or `manual`). Per-artifact,
entry-count and total uncompressed limits are mandatory. Unknown fields, app
IDs, adapter versions, extra/nested ZIP entries and unsafe paths are rejected.

## Implementation order

### Phase 0 — Freeze the contract

1. Define `personal-bundle-v1` and the credential/exclusion policy.
2. Treat every bundle as sensitive, even when secret export is disabled. Permit
   transport only on trusted encrypted storage; do not add cloud sync while the
   bundle lacks authenticated encryption.
3. Keep established data locations and archive formats when rebranding the
   product; do not introduce a generic migration-platform framework.
4. Defer personal-operation history. If later required, use a separate
   `personal-history-v1.json`; never widen the Codex-specific history schema.

### Phase 1 — Beyond Compare proof of extension

1. Add one explicit Beyond Compare card and readiness check; defer a catalog.
2. Let the user select a native `.bcpkg` created with `Tools > Export Settings`.
3. Hash and package it without interpreting or rewriting its payload.
4. On recovery, extract create-new to a Companion staging folder and direct the
   user to native `Tools > Import Settings`.
5. Record whether the user selected native export of saved passwords and
   FTP/SSH credentials. Because this cannot be proven from an opaque package,
   still classify the artifact as sensitive and require encrypted transport.
   Never include or migrate the license key.

### Phase 2 — SourceTree non-secret settings

1. Require SourceTree to be closed before preview and execution.
2. Parse only fixture-proven fields from `bookmarks.xml`; reject credential-bearing
   URL userinfo and unknown/future XML. Exclude tabs, custom actions, whole-file
   `user.config`, hosted accounts and credential files initially.
3. Record SourceTree version and detect machine-specific repository paths.
4. Restore to staging first. Offer create-new placement only when the exact
   destination is absent; otherwise report a conflict for manual review.
5. Add more files and path remapping only after fixtures from both home and work machines prove
   the XML variants. Do not guess new schemas.

### Phase 3 — XAMPP files

1. Detect installation/version/architecture; do not archive XAMPP binaries.
2. Back up explicitly selected `htdocs` projects. Treat all project content as
   sensitive; heuristics may warn but cannot promise credentials are absent.
3. Archive reviewed Apache/PHP/MariaDB configuration as manual components.
4. Require relevant services stopped for all file-level backup and restore.
5. Restore into a clean compatible XAMPP installation with create-new semantics;
   report conflicts instead of replacing them.

### Phase 4 — XAMPP logical databases

1. Inventory databases without rendering row data or credentials.
2. Run the canonical bundled MariaDB dump client directly without a shell, with
   explicit database selection and consistency options. Never put passwords in
   command arguments, logs or manifests; specify and test a restricted,
   create-new temporary credential channel with guaranteed cleanup before coding.
3. Store SQL dumps as hashed artifacts and validate command exit status plus
   expected dump structure before publishing the bundle. Database content is
   sensitive by definition.
4. Use two explicit states: MariaDB running for the logical dump, then all XAMPP
   services stopped for file snapshot and final source revalidation.
5. Keep SQL recovery manual in the initial format. SHA-256 detects corruption,
   not malicious SQL provenance; never execute SQL from an arbitrary inspected ZIP.
6. Define dump shape, engine/locking policy, routines/events/triggers/views and
   version compatibility before any later automatic import.
7. Defer physical backup until version compatibility, prepare/restore tooling,
   empty-target checks and destructive rollback are specified and tested.

### Phase 5 — Product rename and polish

Completed branding as Dev Companion while preserving existing data locations
and archive contracts. Continue to avoid a speculative migration-platform
framework.

## Likely affected areas

| Area | Expected change |
|---|---|
| `src-tauri/src/environment.rs` | Reuse behavior; extract only proven shared archive mechanics |
| `src-tauri/src/fs_safety.rs` | Reuse path/reparse validation unchanged where possible |
| `src-tauri/src/platform.rs` | Add Companion staging/bundle locations and explicit app root resolvers |
| `src-tauri/src/commands.rs`, `lib.rs` | Add narrow personal bundle and per-app commands |
| New Rust app modules | Concrete inventory, exclusions, process and restore policy |
| `src/features/backup` | Add one app workflow at a time, preview, conflicts and restore status |
| `src/services`, `src/types`, `src/i18n` | Add concrete typed DTOs and bilingual messages; generalize only proven overlap |
| Documentation | Bundle contract, per-app support matrix, limitations and restore guide |

## Risks and controls

- **Secrets in one archive:** treat every bundle as sensitive; explicit exclusions
  reduce exposure but do not prove absence; no content in UI/history/logs; require
  trusted encrypted transport until authenticated bundle encryption exists.
- **Live/inconsistent data:** app/service-specific closed checks; MariaDB logical
  dump instead of live file copy.
- **Schema/version drift:** strict app/adapter format allowlists; unsupported means
  manual recovery, never guessed parsing.
- **Machine-specific paths:** preview and conflict reporting; SourceTree remapping
  only for fixture-proven formats.
- **Overwrite/data loss:** create-new writes, revalidation immediately before
  execution, rollback of newly created files, no replace mode in initial releases.
- **Archive growth:** per-file and total bundle limits before reading into memory.
- **Executable SQL:** integrity is not provenance; initial recovery is manual and
  automatic execution requires a separately designed authenticated trust boundary.
- **False “sync” expectation:** describe the product as backup/migration; no
  background or bidirectional synchronization claim.

## Assumptions requiring confirmation

- Windows is the first supported platform for non-Codex applications.
- The first release produces a local bundle; transport between machines is
  user-managed on trusted encrypted storage.
- Credentials and licenses are intentionally re-entered on the destination machine.
- Existing Codex behavior and archive formats remain backward compatible.

## Validation plan

- [ ] Synthetic strict-manifest, traversal, duplicate-entry, hash and size fixtures.
- [ ] Golden regression fixtures proving existing session-v1, delete-v1 and
      environment-v2 readers/commands remain unchanged.
- [ ] One round-trip fixture per app and adapter format/version.
- [ ] Process-running rejection for SourceTree and XAMPP file snapshots.
- [ ] Secret-exclusion fixtures with synthetic values only.
- [ ] Create-new conflict, source-change, partial-failure and rollback checks.
- [ ] Hash-valid malicious bundle/SQL rejection and ZIP entry-count, total-size,
      compression-ratio, reserved-name, ADS and case-collision limits.
- [ ] SourceTree fixtures with URL userinfo and malformed/future XML.
- [ ] MariaDB dump against disposable mixed-engine databases, including failed
      command, DDL warning, routines/events/triggers/views and version mismatch.
- [ ] Home-to-work and work-to-home manual acceptance test with compatible app versions.
- [ ] `pnpm lint`, `pnpm check`, `pnpm test`, `pnpm build`.
- [ ] `cargo check --manifest-path src-tauri/Cargo.toml` and focused/full Rust tests.
- [ ] `git diff --check`; rebuild portable/installer artifacts only after source changes.

## First implementation slice

Implement Phase 0 plus Phase 1 only. This proves that the Codex-specific product
can host a second safe migration workflow with the smallest risk and without
premature provider abstractions.

## Phase 0–1 implementation status — 2026-09-07

Implemented the frozen `personal-bundle-v1` contract, the concrete Beyond
Compare `.bcpkg` workflow, and Phase 2 SourceTree bookmarks. SourceTree is a
separate concrete strict reader/writer: it accepts only the fixture-proven
`bookmarks.xml` name/path schema, requires SourceTree closed, records the
detected version and machine-specific bookmark paths, validates source/archive/
destination state twice, and recovers create-new only to Companion staging.
SourceTree placement remains manual even when absent; conflicts are previewed.
No generic provider API/catalog, personal history, cloud, scheduling,
incremental mode, overwrite mode, or automatic SourceTree import was added.

Phase 2 validation completed: `pnpm lint`, `pnpm check`, `pnpm test` (15 tests),
`pnpm build`, Cargo check, Cargo library tests (52 tests), `git diff --check`,
and the Windows x64 NSIS bundle passed. MSI was not run.

## Phase 3 implementation status — 2026-09-07

Implemented only XAMPP file migration: direct user-selected `htdocs` projects
and four fixture-reviewed text configuration files. The concrete `xampp.rs`
bundle does not alter Codex archive commands/DTOs, Beyond Compare, or
SourceTree. It requires Apache/MariaDB/XAMPP-related processes stopped without
terminating them, detects compatible XAMPP version/architecture, verifies
source identity before creation, and validates SHA-256, count, size, inventory,
archive hash and destination state before staging-only recovery. No provider
trait, plugin SDK, catalog, cloud sync, scheduler, incremental/overwrite mode,
binaries, MariaDB data, SQL dumps, automatic placement, or import was added.

## Dev Companion branding and credential-selection delta — 2026-09-08

The product is now branded Dev Companion. The Tauri product name, installer,
portable executable and visible UI labels changed, but the legacy
`codex-companion` application-data directory and all archive kinds/formats
remain fixed for compatibility. Beyond Compare now records the explicit user
choice whether a native opaque `.bcpkg` includes saved passwords or FTP/SSH
credentials; it does not inspect the package, move a license, encrypt the ZIP,
upload it, or import it automatically. A future application remains a concrete
adapter/card/command/test slice, not a provider framework.
