# Implementation Plan — Personal application migration

## 2026-10-06: SQLite copy boundary and stale sampled UI

- Latest raw log reaches completed=608, copy-file for .codex/state_5.sqlite,
  with complete callback-return pairs and zero rejected enqueue records. The
  ZIP entry is 14,446,592 bytes (3,243,973 compressed); read vs write blocking
  is not established. User measured 6,685,187 bytes twice, thirty seconds apart:
  the destination copy itself is incomplete and stationary.
- The new screenshot's UI still shows563: native progressed beyond that, but
  the leading-edge100ms gate discarded the last event and has no timer to
  flush it during a long I/O wait. Replace it with a latest-value observer
  that samples on its own timer; never hold its mutex during Channel.send,
  and never join that UI-only worker from data work.
- Split shared restore/rollback copy into64KiB read/write phases with confirmed
  output byte counts, retry Interrupted reads and preserve read-to-EOF/CRC,
  final size/SHA, source locks and rollback. Do not skip the database or weaken
  verification; no underlying device/cloud/filter cause is yet attributed.
- Frontend ignores late callbacks after invoke settles and derives completion
  from the native result. Tests cover trailing UI delivery without another
  native event, blocked UI transport, chunking/Interrupted/short-write errors,
  IPC cleanup, native integrity/rollback and the real ZIP before separate build.
- Repeated attempts intentionally retain safety snapshots. User can remove
  intermediate snapshots after closing Companion, preserving the original
  migration ZIP, first safety snapshot and latest safety until verification.

Completed locally: 13 focused migration tests, four command tests, frontend
check/lint and 34 tests, independent review, and optimized separate copy-progress
EXE passed. The actual 618-file ZIP passed fresh/populated isolated restoration
with every output independently SHA-checked and actual asynchronous tracing.
Target-machine I/O cause, packaged interaction and completion remain unverified.

## 2026-10-06: Confirmed partial diagnostic write holds restore

- User confirms the original log ends mid-JSON, after writing the true callback
  flag during `verify-read` on entry 595. That proves callback return and entry
  into the after-callback diagnostic write; file verification has not resumed.
- The logger currently serializes JSON directly into a File on the restore
  thread. Ignoring write errors does not prevent a blocking write from holding
  restore. Move only diagnostic writes to a worker with a bounded std channel
  and non-waiting `try_send`; serialize complete records on the producer.
- Create the log and worker before mutations. Never join that worker during
  production restore/rollback/completion. It owns only the Companion log,
  never a destination or safety file. Drop excess/disconnected diagnostics,
  report gaps on the next accepted record, and retain all integrity checks.
- Test a deliberately blocked writer with a full queue while a real isolated
  restore completes; release/join only for test cleanup. Test disconnected
  logging, complete record encoding, overwrite/verification/rollback, then
  independently review and build a distinct portable EXE.

Completed: 12 focused migration tests plus real supplied ZIP with actual async
trace passed on fresh/populated isolated targets (618/618 independently SHA
checked each). Independent review, optimized separate log-fix EXE and diff
check passed. Actual new-machine completion and Projects menu remain unverified.

## 2026-10-06: Entry 595 finishes; next target preparation stops

- User trace confirms size/read/SHA and close completed for `validate_atlas.py`;
  the next file stops at `prepare-target`. Existing records precede the progress
  callback as well as filesystem operations, so earlier attribution to a
  particular blocked filesystem call was too strong.
- Record callback return explicitly, split target check/mkdir/recheck, and log
  the exact ancestor before each metadata inspection. Keep path/reparse checks,
  actual-byte hashes and rollback intact. Ancestor details stay in local logs.
- Limit UI notifications to stage changes and at most ten updates per second;
  keep every native operation in the diagnostic trace. This bounds UI traffic,
  without claiming that IPC is the established cause.
- Verify callback bracketing, ancestor coverage/reparse rejection and restore
  ordering. Independent review and separately named optimized EXE required.
  Target-machine completion still needs evidence; no integrity bypass or
  file-specific exception is permitted.

Completed locally: 10 migration and four command tests, frontend check/lint and
33 tests, independent review, optimized separately named path-trace EXE and
diff check passed. Data-copy/verifier remains unchanged; no repeat of the prior
618-file real-ZIP check in this diagnostic delta. Target-machine root cause,
actual IPC under load and completion remain unverified.

## 2026-10-06: Verification after close still stalls on target

- Synced flush-fix EXE matches the built SHA. Latest target logs reach
  `close-file`, then stop at `verify-file` for the same entry 595. The older
  independent PowerShell check of those bytes returned the correct SHA quickly.
  The destination reopen/check/read path is now the unconfirmed substep.
- Verify the actual destination through its original read/write `create_new`
  handle before closing. On Windows deny competing writes/deletes while that
  handle is held. Check file length, seek to zero, stream exactly the expected
  byte count and compare SHA. Preserve source/safety locks and all rollback.
- Apply the same output verification to safety rollback. Trace size/seek/read/
  digest boundaries, without recording content. No hash bypass, skipped files,
  detached timeouts or hard-coded exception for `validate_atlas.py`.
- Test same-handle verification, size/SHA mismatch rollback and Windows handle
  sharing. Validate source/target checks and build a separately named EXE.
  Independent review approved the mechanism; remote completion still requires
  target-machine evidence and is not inferred from local round trips.

Completed locally: nine focused native tests, frontend check/lint and 33 tests,
independent review, optimized EXE build and the real supplied ZIP restored and
independently SHA-checked 618/618 on fresh/populated isolated targets. Separate
`dev-companion-migration-handle-fix-20261006.exe` built; target-machine completion
and Projects menu remain unverified. See `docs/VALIDATION.md` for evidence.

## 2026-10-06: Confirmed destination disk-sync stall

- New target logs end at `flush-file` for entry 595, immediately before
  `output.sync_all()`, without reaching close/SHA check. Copy returned in
  milliseconds; the independently read target hash already matches.
- Remove the per-destination-file forced disk sync in both restoration and
  safety rollback. Close outputs and read/hash them as normal file copies.
  Keep both backup/safety ZIP syncs and strict safety verification before any
  deletion. Do not substitute `sync_data`, a no-op `File::flush`, a final
  forced sync, or detached timeouts that leave an active writer behind.
- Destination completion confirms readable matching bytes using Windows
  writeback, not physical power-loss durability for every output file. Retain
  source/safety ZIPs for recovery if an interruption leaves a partial target.
- Validate phase order without the destination flush, overwrite/rollback,
  and real supplied ZIP on fresh/fully populated isolated targets. Build a
  separately named optimized EXE. Independent review approved this scope;
  actual target-machine completion remains a manual check.

Completed: 7 focused native tests; actual supplied ZIP verified 618/618 outputs
in both fresh and populated isolated targets; independent review; optimized
separate flush-fix EXE built. No destination `sync_all()` remains in this module;
the two archive syncs remain. Test details/hashes are in `docs/VALIDATION.md`.

## 2026-10-06: Restore counter stopped at 594/618

- On the new machine, restore stays at 594. Next entry is the 10,983-byte
  `skills/hatch-pet/scripts/validate_atlas.py`; the target file exists with the
  full 10,983 bytes, while entry 615 `transcription-history.jsonl` is absent.
  This confirms native completion cannot be assumed. Isolated extraction,
  durable flush and SHA verification of that exact ZIP member took 131 ms.
- Existing progress is emitted only after copy, disk sync, close and SHA check.
  Add current relative filename and substep before those operations. Preserve
  all checks and synchronous rollback; never detach timed-out file operations.
- Write metadata-only phase records to a Companion-owned local diagnostic log
  so a lost channel message cannot hide actual native progress. Do not log file
  contents or credential paths. No cause is attributed to antivirus/cloud I/O
  without target-machine evidence. Validate phase ordering and trace metadata.

## 2026-10-06: Long-running replacement without feedback

- The target-machine report shows replacement busy for about 30 minutes, with
  no progress. No matching running process is available on this development
  machine, so the remote operation phase is not confirmed.
- Safety discovery previously walked every account directory (including excluded
  caches/runtimes) before filtering. Collect archived targets directly and
  recurse only chat directories needed for snapshot rollback; deduplicate
  Windows paths and retain reparse-point checks. Do not inspect authentication.
- Use uncompressed verified safety ZIPs to avoid recompressing the destination.
  Keep all safety/rollback and destination SHA-256 guarantees.
- Report queued/process-check/archive-check/destination/safety/restore/rollback
  stages through a per-invocation Tauri channel. Show truthful stage/file counts,
  with an indeterminate state when the total is unknown.
- Validate both a fresh and fully populated target with the supplied archive,
  plus progress and excluded-directory regression tests. Build a new separately
  named portable executable; preserve the previous artifacts.

Completed: frontend check/lint/build and 33 tests; 6 focused native tests;
real supplied ZIP restored 618/618 SHA-256-matching files on both fresh and
fully populated isolated targets. Independent review found no integrity or
data-loss blockers. Separate optimized progress EXE built. No remote process
or new-machine Desktop behavior was inspected; the reported 30-minute phase
is still unconfirmed. Stored safety copies trade compression time for disk
space and can be about 1 GiB for this archive.

## 2026-10-06: Restore overwrites and recovery layout regression

- The supplied Codex ZIP contains `.codex-global-state.json`, local project
  metadata, and databases. Its global-state keys include `local-projects` and
  `project-order`; create-only recovery leaves a freshly installed target's
  conflicting registry untouched. Default migration recovery to the explicit
  safety-backed replacement path, requiring `REPLACE CODEX`; the migration
  page no longer offers the keep-existing mode.
- Replace every archived supported file, including project/global state. Keep
  destination components absent from the archive; only chat data uses complete
  snapshot replacement to avoid stale SQLite WAL/SHM companions. Track removed
  files for rollback so an interrupted deletion cannot prevent recovery of
  files already removed. Verify populated-target project replacement and rollback.
- SourceTree restores primary configuration files to both standard Local and
  Roaming roots; the backup's source profile is not destination evidence.
  Discover primary source files from Local first, falling back per file to
  Roaming. Detect the installed `user.config` independently.
  Preserve old bundle compatibility, safety copies, and destination hash checks.
- Narrow archive action-row CSS so appending the recovery panel cannot turn it
  into a horizontal flex row. Visually verify both recovery panels.
- Run frontend/native checks and produce a separately named portable test build;
  preserve the existing portable executable. Real new-machine Desktop behavior
  remains a manual verification, distinct from file round-trip checks.

## 2026-10-05: Deterministic SourceTree profile restore and Codex project evidence

- Store the SourceTree configuration profile (`LOCALAPPDATA` or `APPDATA`) in
  new encrypted bundles. The original same-profile restore plan was superseded
  on 2026-10-06: source provenance cannot identify the new machine's active
  profile, so restore now covers both standard primary roots.
- Keep version-1/2 bundles readable. New bundles use format version 3 with the
  same AES-256 entry encryption, allowlist, safety copy, rollback and hash
  verification.
- Surface the component totals from an inspected Codex migration ZIP so the
  restore screen explicitly shows whether the selected archive contains local
  project files. This is evidence of the file-level restore only; it must not
  claim that the Desktop Project sidebar or cloud project state was recreated.

## 2026-10-05: ChatGPT project-local data in Codex migration

- Detect `.chatgpt-projects` beneath supported `.codex` accounts. Its files
  form one selected-by-default `projects` component and restore only to the
  same nested folder below the matching destination account.
- Keep the existing strict traversal, reparse-point exclusion, SHA-256 ZIP
  validation, create-new restore and rollback. This includes local project
  metadata, instructions and sources only; it cannot promise restoration of
  cloud project state or transfer official CLI authentication.
- Add focused native round-trip/estimate coverage and update the bilingual UI
  scope text so a zero Worktrees row is not mistaken for ChatGPT Projects.
- Preserve all other durable, non-credential account state in a separate
  selected-by-default component. Exclude authentication, machine identity,
  caches, temporary files, sandboxes, links, recovery artifacts and Git data.

## 2026-10-05: Unified portable backup folder

- Write new personal application bundles to the existing portable `backups/`
  folder, shared with Codex migration ZIPs; do not introduce another archive
  root.
- When the SourceTree configuration archive list loads, move only legacy direct
  regular `sourcetree-config-<timestamp>.zip` files from `backup/` to
  `backups/`, after reparse-point checks. A name collision leaves the legacy
  file untouched; there is no overwrite or arbitrary-file migration.
- Treat a shared Dashboard directory as one inventory so file counts and sizes
  are not doubled. Update portable documentation and release notes.

## 2026-10-05: SourceTree configuration archive list

- List only direct regular `sourcetree-config-<timestamp>.zip` files in the
  Companion personal-bundle directory, newest first with total size. Do not
  expose other personal bundles or arbitrary filesystem paths.
- Allow Explorer reveal and deletion only after native filename, regular-file
  and reparse-point validation; the UI asks for confirmation before delete and
  refreshes the list afterwards.
- Reuse the existing archive-list UI pattern and add a focused native scope
  test. Update the guide and release notes.

## 2026-10-04: SourceTree configuration recovery clarity

- Keep the existing AES-256 bundle format, closed-SourceTree gate, strict
  inspection, safety copy and rollback unchanged. Reduce the new-bundle
  password policy from twelve to six characters at the native validation and
  UI boundary, without saving the password anywhere.
- Make the existing recovery stages visible: tell the user when the selected
  ZIP is being read, place the preview action above the inspected file list,
  then expose the explicit `RESTORE` action after preview. An old unencrypted
  bundle may still use an empty password.
- Add focused checks for the six-character boundary and the recovery controls;
  run the focused Rust test, frontend checks/tests, lint, build and whitespace
  check. A real SourceTree restore remains a target-machine manual check.

## 2026-10-05: SourceTree restore evidence

- Keep the existing direct allowlist and safety-copy/rollback flow. After each
  replacement, compare the destination SHA-256 and byte count to the inspected
  bundle manifest; any mismatch rolls back the already-written targets.
- Return the verified destination of every restored file so the UI can show
  the exact files applied, including `bookmarks.xml`, `opentabs.xml`, `passwd`,
  and `userhosts`. Do not expose file contents.

## 2026-10-04: Replace Codex chat snapshot and restore feedback

- Keep the existing create-only restore as the default. Add one explicit
  replace-chat mode; it never attempts to compare timestamps or merge SQLite.
- When selected, require Codex to be closed, snapshot only the target's
  allowlisted chat files into a verified safety ZIP (never auth or machine
  identity), then replace the complete selected chat snapshot including SQLite
  WAL/SHM and sessions. A failed replacement removes new files and restores the
  safety ZIP.
- Show an in-progress state while a migration ZIP is chosen/validated and
  while the restore preview is built. Keep restore controls above a bounded,
  scrollable file list so a large archive cannot hide the action.
- Add focused native replacement/rollback coverage and extend the page test;
  run TypeScript, lint, frontend tests, focused Rust tests, and whitespace
  checks. A target-machine Desktop history check remains required before this
  mode can be claimed as a verified Desktop migration.

## 2026-10-04: Multi-account Codex migration

- Add one concrete `codex_migration.rs` workflow under the Codex navigation
  group. Keep `environment.rs` and its environment-v2 reader unchanged so
  existing archives remain compatible.
- At page load, detect the current user's direct `.codex` and `.codex-*`
  directories, deduplicate them with the active `CODEX_HOME`, and offer each
  existing account as a selected-by-default backup source.
- Offer recommended checkboxes for chat history/databases, safe configuration
  and instructions, skills, and pets. Offer worktrees, plugins, and
  visualizations as opt-in space-heavy components. Runtime caches, sandboxes,
  vendor imports, and official CLI authentication or machine-identity files
  remain excluded regardless of selection.
- Produce a strict versioned ZIP with per-account relative paths and SHA-256
  validation. Restore only after inspection and `RESTORE`; create files only
  beneath the current user's matching `.codex*` account folders, skip every
  existing conflict, and roll back files created by a failed operation.
- Require Codex to be closed before preview, archive creation, or restore.
  Add focused Rust round-trip/rejection coverage and a React rendering test;
  run the relevant TypeScript, lint, test, Rust, and whitespace checks.

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
