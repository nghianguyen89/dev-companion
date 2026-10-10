# Current State

Concise repository snapshot. Update at meaningful milestones.

## Baseline

```text
Branch: main
Baseline commit: dc504ad
Release/version: 0.2.0
```

## Working Features

- Windows-only local migration workflows for Codex environment files, Beyond
  Compare packages, SourceTree personal configuration and selected XAMPP
  projects/config.
- Native commands perform blocking filesystem, process and dialog work away
  from the window thread while retaining serialized operations.
- Conversations table reuses its per-language date formatter, keeping filters
  and Refresh responsive for large local session inventories.
- Codex Environment Manager: custom Windows `CODEX_HOME` entries, CLI status,
  login/logout/launch handoff, safe `.cmd` launchers, User PATH opt-in, and
  marker-preserving global `AGENTS.md` management. It never persists or reads
  authentication credentials; removing metadata never deletes `CODEX_HOME`.
- Codex Migration: scans the current user's direct `.codex` and `.codex-*`
  accounts, creates one strict SHA-256 ZIP with selected account,
  `.chatgpt-projects` project-local and other durable-state content,
  and overwrites archived files into matching account folders after
  `REPLACE CODEX` and a verified target safety ZIP. This includes project/global
  state; the chat snapshot is replaced completely to remove stale WAL/SHM.
  Destination-only non-chat files are kept. It never merges SQLite or
  compares file timestamps. Chat/local databases, safe settings, skills and
  pets, local ChatGPT project data, other durable state, worktrees, plugins and
  visualizations are selected by default. Local project files do not guarantee
  cloud-project state restoration.
  Authentication, machine identity, runtime caches and sandboxes are always
  excluded. Component and selected backup-total estimates are calculated from
  current source metadata. The module lists its own ZIPs with timestamps and
  sizes, can reveal one in Explorer, and deletes only a confirmed, direct
  regular migration ZIP.
- File Transfer: direct Windows Robocopy Simple/Fast/Project Migration/Mirror
  workflows with navigable Source/Destination direct-child views, safe source
  selection, hidden/system icon/color legend, preview, confirmations, dry-run,
  streamed output, cancellation, logs/history, profiles and optional
  non-destructive destination verification. Start analyzes first; determinate
  progress requires both the dry-run byte total and guarded English completed-file output.
- File Compression: independent 7-Zip CLI `.7z` creation with a recursive,
  read-only source tree, path and regex exclusions, Fast (`-mx=1`) / Strong
  (`-mx=9`) modes, source-parent output by default, direct output streaming,
  truthful 7-Zip percentage progress, bounded/filterable tree rendering,
  cancellation, reparse-point exclusion, and no-overwrite publication.
- Local Codex Skills and Pets: direct supported metadata lists; skills import
  and export into new folders only; pet v2-manifest/sprite validation,
  create-new installation, and `REMOVE`-confirmed deletion limited to the
  selected direct pet folder. These workflows never read authentication or
  display skill bodies.
- SourceTree personal configuration: a sensitive closed-app AES-256 bundle
  includes detected `accounts.json`, `bookmarks.xml`, `customactions.xml`,
  `hostedaccounts.xml`, `opentabs.xml`, `passwd`, `userhosts`, and the current
  `user.config`; direct restore needs `RESTORE`, takes a safety copy, verifies
  every replaced target against its bundled SHA-256, and exposes a post-restore
  `DELETE` action. Version-3 source-profile metadata is provenance only;
  primary files overwrite both Local/Roaming configuration roots, with separate
  safety copies and hash checks. `user.config` maps independently to the current
  installation. Version-1/2 bundles remain readable. Its managed bundle list shows only
  matching configuration ZIPs and allows confirmed deletion. Windows Vault, OAuth/DPAPI-bound
  secrets and SSH keys remain excluded, so target-machine sign-in may still be
  required.
- Portable personal bundles and Codex migration ZIPs share `backups/` beside
  `dev-companion.exe` when the adjacent `portable-mode` marker is present;
  installed builds retain the existing AppData location. SourceTree moves only
  validated legacy configuration ZIPs from `backup/` without overwriting.
- Dashboard is the first navigation group and shows current local feature
  counts plus read-only direct-file inventories for session backups and
  personal application bundles.
- Settings also includes an in-app Guide covering each visible workflow and an
  About page that displays the current version and release notes from
  `src/app/releaseNotes.ts`.

### Phase 0–1 addition

- Windows-only Beyond Compare package bundle/recovery: one opaque user-selected
  `.bcpkg`, strict `personal-bundle-v1` inspection, SHA-256, create-new staging
  recovery, rollback, and manual native import.
- Existing Codex session-v1, delete-v1 and environment-v2 workflows remain
  separate and unchanged.

### Phase 2 addition

- Windows-only SourceTree `bookmarks.xml` bundle/recovery: strict fixture-only
  XML parsing, closed-process gate, source/version/hash revalidation,
  namespaced `personal-bundle-v1` ZIP inventory, create-new staging and manual
  destination placement only.
- SourceTree repositories, tabs, custom actions, `user.config`, hosted
  accounts, credentials, licenses and secrets remain excluded.

### Phase 3 addition

- Windows-only XAMPP personal bundle: explicitly selected direct `htdocs`
  projects plus four reviewed text configuration files, SHA-256 inventory,
  strict size/count/namespace validation, stopped-process gate, and
  create-new rollback-safe staging recovery.
- XAMPP binaries, MariaDB data/logical dumps, credentials, keys, logs, caches,
  repository metadata, reparse points, overwrite, automatic placement and
  MariaDB import remain excluded/manual-only.

### Dev Companion branding and Beyond Compare credential selection

- Visible product, installer and portable-executable branding is Dev Companion;
  the legacy `codex-companion` application-data directory and established
  archive formats remain unchanged for compatibility.
- Beyond Compare accepts an explicit user declaration that its opaque `.bcpkg`
  includes saved passwords or FTP/SSH credentials, records that declaration,
  and remains manual-import/staging-only. The ZIP is not password-protected.

## In Progress

- XAMPP domain manager implementation/build completed; packaged UI, LAN-client and installer acceptance remain pending. Read `docs/XAMPP_HANDOFF.md` before continuing from another account.

## Known Issues

- Desktop chat/database merging remains manual and unverified; archive files do
  not establish a usable migrated Codex Desktop environment.

- A `.bcpkg` cannot prove whether its native export includes saved passwords or
  FTP/SSH credentials; the user's recorded choice is not verification and every
  personal bundle remains sensitive. No authenticated encryption, cloud
  transport, license migration or automatic Beyond Compare import is implemented.

## Technical Debt Worth Remembering

- Native operations use one process-wide gate. Split it only if measured
  independent operations need concurrent execution.

## Validation Status

```text
Lint: pnpm lint (pass, 2026-09-09)
Type-check: pnpm check (pass, 2026-09-09)
Tests: pnpm test (18 pass); cargo test --lib (60 pass, 2026-09-09)
Build: pnpm build (pass); Windows x64 NSIS bundle and portable executable (pass, 2026-09-09); MSI not run
```

2026-10-03 File Compression: `pnpm check`, `pnpm lint`, `pnpm test` (21),
`cargo check`, and 8 focused compression tests (including a local 7-Zip smoke
archive) passed. The full Rust library run had 80 passing tests and one
pre-existing environment-dependent failure resolving the current user home.

2026-10-03 Skills/Pets: `pnpm check`, `pnpm lint`, `pnpm test` (27), `pnpm
build`, `cargo check`, and 4 focused Rust contract tests passed. The full Rust
library run had 87 passing tests and the same pre-existing
`codex_environment::tests::metadata_crud_and_removal_leave_codex_home_in_place`
failure because the current user home could not be resolved.

Never mark validation as passing unless it was actually run.

2026-10-06 Migration restore: check/lint/build and 32 frontend tests passed;
native library run had 91 passing tests, one opt-in test ignored, and the same
existing environment-manager home-resolution failure. The opt-in real Codex
ZIP restore passed separately, verifying all 618 files in an isolated target.
SourceTree overwrite/rollback tests and 847px/600px browser layout fixtures
passed. A separate optimized portable test EXE was built; new-machine Desktop
and packaged-app interaction remain NOT RUN. See `docs/VALIDATION.md`.

## Important Recent Decisions

2026-10-06 SQLite copy progress: the target database is stationary at
6,685,187/14,446,592 bytes; its UI remains at 563 while native logs reach 608.
Replace the leading-edge notification throttle with an independent latest-state
100ms UI observer. Channel transport cannot hold native file work. A shared
64KiB copy reports read/write boundaries and confirmed written bytes while
retaining integrity and rollback. The frontend memoizes the static inventory,
ignores late callbacks and derives completion from the native result. Target
I/O cause and new-machine completion remain unverified.
Local check/lint, 34 frontend tests, 13 migration/four command tests, independent
review and optimized separate copy-progress EXE passed. Actual supplied ZIP:
618/618 independent output hashes matched on both fresh and populated isolated
targets through the real asynchronous trace (371.94s total debug test).

2026-10-06 Diagnostic logger isolation: the user confirms their raw log ends
mid-JSON with the true callback flag on `verify-read`. The callback returned;
the after-callback diagnostic write held this run before verification resumed.
Logging now uses a bounded `try_send` queue and a worker owning only the log,
with complete record encoding and no production join. Restore retains all
path, source/safety, actual-byte SHA and rollback checks. Full/disconnected
diagnostics are discarded/counted rather than holding data operations.
Twelve focused migration tests passed, including a deliberately blocked writer
with a full queue while a real isolated restore and independent SHA complete.
Actual new-machine completion/Projects menu remains unverified.
The supplied real ZIP now runs through the actual asynchronous trace too:
618/618 files restored and independently SHA-checked on fresh/populated isolated
targets. Independent review and optimized log-fix EXE build passed.

2026-10-06 Target preparation diagnostics: the handle-fix user run completes
entry 595, then stops preparing entry 596. Earlier log records precede progress
callbacks, so no exact blocked filesystem call was established. New false/true
callback-return pairs plus check/mkdir/recheck and ancestor metadata records
resolve that ambiguity. UI notification volume is bounded to ten per second
within a stage, with stage changes immediate; local trace keeps every event.
Safety/path/hash/rollback behavior remains unchanged. Target root cause and
completion remain unconfirmed.
Ten migration/four command tests, frontend check/lint and 33 tests, independent
review and separate optimized path-trace EXE build passed. Actual IPC behavior
on the new machine remains unverified.

2026-10-06 Output-handle verification: the flush-fix target run closed file 595
then stopped at `verify-file`; the supplied screenshot agrees with the trace.
Restore and rollback now verify size and SHA using the original read/write
handle before closing, with Windows sharing denying competing writes/deletes.
No destination reopen, skipped file or SHA bypass is used. Nine focused native
tests (including size/digest rejection, sharing and complete hash-failure
rollback), frontend check/lint and 33 tests passed. Actual new-machine
completion/Projects menu remains unverified; local validation cannot prove it.
The actual supplied ZIP passed fresh/populated isolated restore with all
618 files independently SHA-checked. Separate optimized handle-fix EXE built.

2026-10-06 Destination flush removal: earlier user trace ended at `flush-file`
after copying file 595 (exact blocked call was not established; see above).
That build made restore/rollback close
and SHA-check outputs using normal Windows writeback; both ZIP syncs and
pre-delete safety verification remain. Focused 7 native tests and the actual
618-file fresh/populated isolated round trips passed after this change.
Optimized flush-fix EXE built. Destination power-loss durability differs from
forced per-file sync; keep source/safety ZIPs. Actual new-machine completion
and Projects menu still need verification.

2026-10-06 Per-file diagnostics: target-machine restore remains stuck at
594/618, with full-size entry 595 and entry 615 absent. A separate diagnostic
EXE adds current filename/I/O operation and a local metadata trace in
`%APPDATA%/codex-companion/logs/`. Native 7 focused tests, frontend 33 tests,
check/lint and optimized build passed. Remote root cause remains unconfirmed;
the earlier successful isolated round trips do not prove this stall resolved.

2026-10-06 Restore progress follow-up: frontend check/lint/build and 33 tests,
6 focused Codex migration tests, and the real-ZIP opt-in check passed. The
618-file archive restored and hash-verified on both fresh and fully populated
isolated targets. Replacement now reports actual phase/file counts, prunes
excluded safety trees, uses verified Stored safety ZIPs, and reuses the strict
manifest only after checking the unchanged ZIP digest under a read lock.
The separate optimized `dev-companion-migration-progress-20261006.exe` was
built; the canonical EXE was not overwritten. Target-machine app behavior
and packaged restore interaction remain unverified.

- Product branding is Dev Companion; existing `codex-companion` storage and
  archive identifiers remain for compatibility.
- The 2026-09-09 portable smoke test exercised 2,000 synthetic conversations:
  sort rendered in about 122 ms and Refresh completed with the window responsive.

## Next Recommended Work

- Run the new NSIS installer on a target machine before release; do not claim
  Desktop chat migration until its manual workflow is verified.

2026-10-08 XAMPP domains: separate GUI/native CRUD with automatic Apache
start/restart, local HTTPS CA bootstrap/reuse and machine trust, optional www,
HTTP redirect, directory listing and LAN. Portable domain state is under
configs/xampp; ordinary backup contract remains unchanged. Copied C:/xampp was
initialized with its existing CA; localhost and three cus-*.local domains were
imported, Windows-trusted HTTPS returned HTTP 200 for all four, and identified
legacy custom files were archived before removal. Recovery directory:
C:/xampp/backup/dev-companion-1791426653180055000. LAN client validation remains
NOT RUN. See ARCHITECTURE, USER_GUIDE and VALIDATION for scope and checks.

2026-10-08 continuation: real portable XAMPP overview inspected through Windows
UI Automation; all 16 domain/www HTTP/HTTPS HEAD checks passed with current
revision and Windows trust (curl revocation best-effort for the local CA).
Current cus-projects.local redirectHttps=true was preserved. Portable CRUD,
restart/listing/LAN option acceptance remains NOT RUN: the UTF-8/BOM harness
issue was corrected, but the next Windows UAC request was canceled. No domain
mutation, initialization, cleanup, product-code fix, rebuild, commit or push.
See VALIDATION.md for probe FAIL versus application-test NOT RUN boundaries.

2026-10-08 browser warning follow-up: user reports all local HTTPS URLs show
security warnings. Public certificate SAN/signature/dates and both Windows
root stores passed inspection; isolated installed Edge loaded all four HTTPS
sites with TLS 1.3 and secure state (no certificate bypass). The user's active
browser/profile warning remains unresolved pending its exact error code.
No CA, Apache, browser profile or trust setting was changed.

Screenshot follow-up: _screenshot_/bug-broswer.png identifies a malformed
https://https//cus-projects.local/ URL causing DNS lookup of host "https"
(Firefox Server Not Found; Edge DNS_PROBE_FINISHED_NXDOMAIN). The captured
failure is explained by that URL; use https://cus-projects.local/. Correct
HTTPS already passed isolated Edge validation; active-browser confirmation
is pending. No certificate/configuration fix was needed for this screenshot.

2026-10-08 domain-list layout implemented: SSL/www status icons, primary full-URL
open/copy (user chose full URL, no www), clickable source folder, right-side
listing/edit/delete icons with tooltips. Saved-domain-only native open commands
preserve management gates and existing contracts. Frontend check/lint/40 tests,
11 focused XAMPP Rust tests, responsive mocked-bridge browser checks and optimized
EXE build passed. New portable: release/portable/dev-companion-domain-layout.exe;
canonical old EXE/installer retained. Packaged click smoke remains NOT RUN after
UI Automation exposed no descendants. No domain/CA/Apache changes or commit/push.

2026-10-08 action follow-up: separated row interactions/draft editing from
Administrator-only writes, added edit scroll/focus and contextual elevation
guidance for non-admin listing/delete. Copy now uses local in-button feedback
and never toggles page-wide busy. Type-check/lint/41 frontend tests and delayed
clipboard/non-admin browser regressions passed; packaged UAC action acceptance
still pending. Native permission checks and live XAMPP state are unchanged.
Elevation now requests a visible window (SW_SHOWNORMAL). Focused Rust tests
(11 passed, 2 opt-in ignored) and optimized build passed. Current review build:
release/portable/dev-companion-domain-actions-fix.exe; previous executables
remain intact. No commit/push.

2026-10-08 SSL indicator follow-up: user expects the icon to follow the saved
HTTPS redirect checkbox. Row icon and open/copy URL scheme now require both
CA readiness and that domain's redirectHttps value. Tooltip explicitly refers
to redirect. Backend TLS certificates/vhosts stay intact when redirect is off.
42 frontend tests, type-check/lint and mocked React edit-save-toggle browser
regression passed; live settings were not modified.
Optimized portable build passed: release/portable/dev-companion-domain-ssl-fix.exe.
Previous executables retained; packaged live-write acceptance remains NOT RUN.

2026-10-08 config/configs unified: shared app settings and transfer history/logs
now use configs in portable/installed mode; XAMPP remains configs/xampp.
Legacy config entries migrate on settings load/save after rejecting links and
collisions; old transfer log paths are validated/remapped. The local portable's
single settings file was moved unchanged, with a retained safety copy outside
portable, and empty config removed. Current executable:
release/portable/dev-companion-configs.exe. Full Rust suite outside sandbox:
111 PASS, 3 opt-in ignored; optimized build PASS. No live Apache/CA mutation.

2026-10-08 XAMPP safety-backup retention completed: default 10 recent completed
archives (1–100), protected initial/oldest, failed/pending and legacy outcomes,
validated inventory plus manual cleanup UI, automatic pruning after verified
live success. No real backup/domain/CA changes. Check/lint/42 frontend tests,
113 Rust tests (3 opt-in ignored), independent review and optimized portable
build passed. Current executable: release/portable/dev-companion-xampp-retention.exe.
Previous EXEs preserved; close them before using this version. Packaged policy/
cleanup and live automatic prune acceptance remain NOT RUN; see VALIDATION.md.
