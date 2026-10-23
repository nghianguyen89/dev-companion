# Validation — 2026-09-07

## 2026-10-17: Sensitive Codex settings opt-in

- PASS: `pnpm check`, 47 frontend tests, `pnpm exec eslint src`, `pnpm build`, changed-Rust rustfmt check and `git diff --check`.
- PASS: 14 migration tests (13 in the initial focused run plus the existing junction test rerun successfully outside the sandbox), 1 real-archive opt-in ignored; 4 command tests passed. The new fixture covers default exclusion, opt-in estimates/selection, credential/runtime exclusion and exact-byte replacement of an empty destination AGENTS.md using four synthetic settings files. Initial test compilation needed an explicit sum type; the corrected test passed.
- PASS: optimized Windows Tauri `build --no-bundle`; separate portable EXE `release/portable/dev-companion-sensitive-settings-20261017.exe` (16,913,408 bytes; SHA-256 `BFF54FA5D854193911008FC103971FA7733744C932ACC3078EDEBC7879993A11`). Copy hash matches the build. Existing portable EXEs preserved. Final focused frontend rerun: 9/9; source ESLint passed again.
- FAIL (existing generated artifacts): full `pnpm lint` reports 105 errors under release validation scripts/WebView data. Source-only ESLint passed. Initial junction test was blocked by sandbox `Access is denied`; an authorized isolated-fixture rerun passed.
- NOT RUN: live-profile backup/restore, packaged UI interaction, installer, full Rust library suite. No live authentication files were accessed. ZIPs remain unencrypted; the opt-in covers selected settings, not Codex authentication or runtime data.

## 2026-10-14: Codex environment CLI and alias repair

- PASS: TypeScript, scoped ESLint for the environment page/fixtures, bridge/DTOs and translations, 43 frontend tests, frontend production build and `git diff --check`.
- PASS: full Rust library suite: 122 passed, 0 failed, 3 opt-in tests ignored. Environment fixtures cover npm shell-shim exclusion, PATH/PATHEXT precedence, version-probe failure, real CMD execution with spaces and ampersands, child CODEX_HOME, argument/exit forwarding, working alias, shadowed alias and changed launcher rejection.
- PASS: explicit installed CLI `--version` smoke in isolated temporary CODEX_HOME: resolved `C:\nvm4w\nodejs\codex.cmd`, returned `codex-cli 0.162.1`. No registry write, credential-file access or authentication action was invoked.
- PASS: independent source review; partial-save refresh and executable suffix precedence findings resolved. Optimized Tauri Windows build (`pnpm tauri build --no-bundle`) completed. Separate test EXE: `release/portable/dev-companion-codex-environments-20261014.exe`; existing portable EXEs preserved.
- NOT RUN: live User PATH mutation/Explorer propagation, real login/logout, packaged UI clicks, installer. Initial sandbox pnpm EPERM and Rust OUT_DIR restrictions were resolved by authorized unsandboxed validation.

## 2026-10-14: Modernizr dependency selection fix

- Cause confirmed with live filenames/metadata: ___resources___/modernizr/node_modules contained .cache plus fast-glob/out and @nodelib/fs.scandir/out, fs.stat/out, fs.walk/out. Those generated dependency directories were incorrectly classified as protected project output.
- Fix: output/cache names are eligible inside already recognized generated candidates. Project discovery/root pruning remains unchanged; sensitive content and links still block selection, now with the exact relative path in the reason.
- PASS: complete Rust library suite, 120 passed / 0 failed / 2 opt-in ignored. Regression fixtures allow selected dependency output/cache removal, preserve sibling project out/node_modules, and continue blocking nested environment/database/backup content. Independent focused review found no blocker.
- PASS: explicit native scan only against D:/projects/cus-svn: incomplete=false, skipped=77, ___resources___/modernizr/node_modules files=17687, bytes=77933944, blocked=None. No preview/execute/delete was invoked against the real source. The Modernizr subfolder scan also passed independently.
- PASS: frontend production build and optimized Windows x64 native build. Fixed artifact: release/portable/dev-companion-project-cleanup-fix-20261014.exe, SHA-256 9BED41B2C5E82776E8BDB87E7F9EFF43F1F0908CF3D446D4AE6AC29A93FB5D1C. Existing EXEs preserved.
- NOT RUN: live project deletion or packaged-app clicks. No live user project files were removed.

## 2026-10-14: Project Cleanup

- PASS: TypeScript, source lint (`pnpm exec eslint src`), 41 frontend tests, frontend production build and `git diff --check`.
- PASS: full Rust library suite: 118 passed, 0 failed, 1 opt-in migration test ignored. Ten Project Cleanup tests cover selection/protected source and lockfiles, changed/new files or markers before mutation, Python source-bound bytecode, IDs/tokens/unsafe roots, queued cancellation, protected/nested managed trees, .next deployment preservation, bounded candidate enumeration, truthful partial deletion and a real Windows junction/outside sentinel.
- PASS: independent focused review resolved managed-root traversal, protected nested data, unbounded enqueue, partial reporting and cancellation publication issues; no remaining blocker reported.
- PASS (synthetic browser fixture only): production ProjectCleanupPage + shared styles, choose/scan, initially empty selection, eligible-only Select all, blocked pnpm-style row, Preview, wrong confirmation disabled, exact confirmation enabled, selection change invalidates preview/confirmation, and mock execute receives only preview token plus confirmation. Fixture invoke does not touch filesystem. Screenshot saved in the chat visualization directory.
- PASS: optimized Windows x64 separate portable test executable: release/portable/dev-companion-project-cleanup-20261014.exe, SHA-256 4C60C7F81693D6CFF8BE06460C3659599C09106A3E5B88AC70CA888AB1F9D006. Includes Codex Cleanup navigation/inventory changes. Existing canonical portable EXE and running Dropbox copy preserved.
- NOT RUN: native picker, Explorer, live-project deletion, packaged-app click/scan/delete interaction, NSIS and second-machine behavior. No live project or Codex data was deleted. Previous native launch validation timed out on app approval; not retried for this task.
- Full-repository lint remains affected by ignored release validation artifacts; source lint passes. Scan limits are 100,000 entries / 20,000 directories / 64 levels / 2,048 candidates; incomplete scans cannot authorize deletion. Hash checks cover files up to 4 MiB and required project/source markers; larger generated files use size/mtime snapshots. This uses the existing trusted-local-folder boundary, not protection against a hostile process racing filesystem checks.

## 2026-10-14: Codex cleanup navigation / size inventory

- PASS: TypeScript check, source lint (`pnpm exec eslint src`), all 39 frontend tests, frontend production build, and three focused Rust cleanup tests.
- PASS: read-only inventory fixture preserves synthetic chat/auth/plugin files, limits traversal, reports partial sizes, and skips a real Windows junction. Existing unknown/changed/outside-file cleanup tests still pass.
- PASS: independent review found no deletion-safety blocker. Root files / omitted entries are explicitly distinguished when traversal is partial.
- PASS: optimized Windows x64 EXE built using Tauri `build --no-bundle`; separate `release/portable/dev-companion-cleanup-20261014.exe`. Canonical portable EXE and running Dropbox copy preserved.
- FAIL (existing artifact scope): `pnpm lint` reports 104 errors in ignored `release/source-tree-ui-check` and `release/xampp-ui-validation` artifacts. Source lint passes; no unrelated cleanup performed.
- NOT RUN: native EXE menu/scan/cleanup interaction. Computer Use launch timed out waiting for app approval. No live Codex files were deleted, no processes were stopped, and no NSIS build/installation or second-machine validation was performed.
- Live metadata-only profile inventory: sessions ~589 MiB, installed plugins ~519 MiB, sandbox binaries ~327 MiB, temporary staging ~145 MiB (including a ~76 MiB plugin checkout). Only the verified remote catalog remains eligible; age and temporary-directory names alone do not prove these checkouts are unused. Counts can change while Codex runs.

## Diagnostic writer isolation — 2026-10-06

- User confirms the raw path-trace log ends mid-JSON, with the true callback
  field during `verify-read` on entry 595. The callback returned; synchronous
  after-callback logging held this run before destination reading resumed.
  No underlying device/filter cause attributed.
- PASS: 12 focused migration tests. A deliberately blocked writer with a full
  one-record queue does not hold actual isolated 8-file restore and independent
  SHA verification. Test releases/joins its worker only for cleanup; production
  never joins it. Includes disconnected logging, complete JSON/newline encoding,
  dropped enqueue counts, trace ordering, actual-byte SHA, sharing and rollback.
- PASS: real supplied ZIP with the same asynchronous trace used in production:
  fresh and populated isolated targets restored 618/618 and independently
  rechecked every SHA. All owned test data removed afterward. Fresh restore
  completed in 77.99s (108.14s including independent checks/test logger drain);
  populated in 165.31s (192.37s including checks/drain). Whole opt-in test 493.73s.
  These are local debug measurements, not target/optimized timing promises.
- PASS: optimized `tauri build --no-bundle` including fresh frontend typecheck
  and build; independent review; diff check. Frontend logic is unchanged in
  this delta; previous lint/33 tests and four command tests were not repeated.
- Separate EXE: `release/portable/dev-companion-migration-log-fix-20261006.exe`,
  16,330,752 bytes, SHA-256
  `CE8740C25BFC0E0EBBC5BE7A62B9E293BFCB8D869ABDD39C2016D945F7FF35D7`.
- Log and worker creation precede mutations. Only diagnostic writes move off
  the data thread; source/safety locks and syncs, destination actual-byte SHA,
  path/reparse checks, rollback, completion and gate behavior remain intact.
  Queue capacity 1024; droppedRecords counts rejected enqueues, not every disk
  loss. Incomplete/delayed logging no longer proves a native restore stopped.
- NOT RUN: actual new-machine completion/Projects menu, packaged restore UI,
  full native suite/NSIS. Restore intentionally does not wait for its logger;
  trailing records may be lost and a stalled log worker can remain until exit.

## Target preparation/callback boundaries — 2026-10-06

- User target run completes size/SHA/close of entry 595, then stops preparing
  entry 596. PowerShell reads the parent directory; no usable timing supplied.
  The earlier pre-callback records did not establish an exact blocked I/O call.
- PASS: 10 focused Codex migration tests (one real-ZIP opt-in ignored) and four
  command tests; includes actual callback-entry/return trace bracketing,
  check/mkdir/recheck order, all first-target ancestor records, reparse rejection,
  overwrite/SHA/rollback and bounded notifications with immediate stage changes.
  A test's wrong first-account assumption was corrected by selecting account ID;
  notification suppression uses a controlled future timestamp to avoid timing
  sensitivity. Failed-test temporary fixture removed after the passing rerun.
- PASS: frontend check/lint, 33 tests, fresh frontend production build and
  optimized `tauri build --no-bundle`; independent review; `git diff --check`.
- Separate EXE: `release/portable/dev-companion-migration-path-trace-20261006.exe`,
  16,375,808 bytes, SHA-256
  `84B4590EC4DF218AEC8784EB1A8B5377933306C1DCD5206A098E182279C08731`.
- No destination integrity, path/reparse or rollback check was removed. This
  delta adds diagnostics/UI sampling; the previous 618-file real-ZIP round trip
  was not repeated because the data-copy/verifier algorithm is unchanged.
- NOT RUN: actual target-machine completion, real WebView2 IPC under restore
  load, Projects/history, full native suite/NSIS. No IPC or filter root cause
  established. A missing after-callback trace is not proof if that log write
  failed/stalled; `true` proves native callback return, not browser delivery.

## Output-handle verification — 2026-10-06

- Latest target log/UI reached close then stopped at `verify-file` on file 595.
  The prior flush removal did not resolve target completion. Exact blocked
  subcall/device/filter is unconfirmed; destination reopen is now eliminated.
- PASS: nine focused native tests, including same-handle actual-byte SHA,
  size/digest rejection, Windows write/delete sharing denial, full rollback
  after digest failure, and verify-before-close metadata trace ordering.
- PASS: real supplied ZIP, 618/618 files restored and independently SHA-checked
  on both fresh and populated isolated targets; temporary test data removed.
  Fresh restore completed in 61.69s (81.39s with independent checks); populated
  in 130.84s (149.80s with checks). Whole opt-in test: 318.33s. These are local
  debug timings, not new-machine/optimized measurements.
- PASS: frontend typecheck/lint and 33 tests; optimized `tauri build --no-bundle`
  including fresh frontend build; independent native review; `git diff --check`.
- Separate EXE: `release/portable/dev-companion-migration-handle-fix-20261006.exe`,
  16,395,264 bytes, SHA-256
  `C1DB82C915C6095625A9CB3B08E2E274F2FF3403E2D511AAA8D348A57BC3428C`.
- Source/safety ZIP synchronization, strict validation and rollback remain.
  Destination uses normal OS writeback, with length/SHA verified through the
  pinned read/write handle before close. Retain recovery ZIPs until verified.
- NOT RUN: actual new-machine completion/Projects menu, packaged restore,
  NSIS installation, full native suite (prior unrelated home-resolution failure).

## Destination flush removal (later target verification still stalled) — 2026-10-06

- Earlier user log/UI ended at the `flush-file` notification. The native
  callback ran between this record and `output.sync_all()`, so this did not
  establish that exact blocked call. Copy finished; independent target
  SHA was correct and readable in 0.134s. No specific driver/filter attributed.
- PASS: 7 focused migration tests, including overwrite/rollback and metadata
  trace phase ordering (copy -> close -> SHA), plus separately enabled real-ZIP
  test on fresh and fully populated isolated targets. Both restored 618/618
  files and independently rechecked every SHA; temporary content was removed.
- Debug measurements: fresh completion 91.77s (115.76s including independent
  output check); populated completion 164.76s (181.75s including check). Whole
  opt-in test 380.17s, including initial strict archive inspection and previews.
  These are development-machine/debug timings, not new-machine/optimized timings.
- PASS: optimized `tauri build --no-bundle`, including fresh frontend typecheck
  and build; `git diff --check`. Frontend logic was unchanged in this delta;
  its previous lint and 33 tests passed and were not repeated. Independent
  review approved removal of only the two destination-file sync calls.
- Both archive ZIP syncs and pre-delete safety verification remain. Targets
  use normal OS writeback plus close/read/SHA; sudden-power-loss physical
  persistence of every destination is not guaranteed. Retain recovery ZIPs.
- Separate EXE: `release/portable/dev-companion-migration-flush-fix-20261006.exe`,
  16,374,784 bytes, SHA-256
  `4664E42D8659E7755B5649C0CC7D74D6B61E424DBEE05ABB76DF963D38697F38`.
- NOT RUN: actual new-machine completion/Projects menu, packaged restore,
  NSIS installation, full native suite (previous unrelated home test failure).

## Per-file stall diagnostics — 2026-10-06

- PASS: 7 focused native migration tests, including ordered per-file operations
  and metadata trace round trip; existing overwrite/rollback tests; frontend
  check/lint and 33 tests; typecheck/frontend production build as part of
  optimized `tauri build --no-bundle`; `git diff --check`.
- PASS: isolated read/extraction, durable flush and matching SHA of actual
  ZIP member 595 (10,983 bytes) in 131 ms on this machine. The target machine
  reports that file full-size but entry 615 absent. No remote I/O cause proven.
- Independent review: no safety blocker. All output hashes, synchronous disk
  sync and rollback remain. Trace writes are metadata-only and best-effort
  after successful creation before mutation. Copy still combines read/write.
- Separate diagnostic artifact:
  `release/portable/dev-companion-migration-diagnostic-20261006.exe`,
  16,392,192 bytes, SHA-256
  `A3167B3B32FD86C918DA8EEEE8F6E56D9360A1C6893E67683B48CB9A60313F56`.
- NOT RUN: target-machine substep logs, packaged diagnostic interaction,
  repeated actual 618-file round trips after adding diagnostic metadata,
  full native suite (previous unrelated home-resolution failure persists).
  The diagnostic build is not evidence that the remote stall has been fixed.

## Migration replacement progress — 2026-10-06

- PASS: frontend check/lint/build and 33 tests; 6 focused Codex migration
  native tests (the separate real-ZIP test is opt-in), 3 command-worker tests;
  `git diff --check`.
- PASS: actual supplied ZIP in isolated targets, first with existing empty
  global state, then with all 618 files already present. Both replacements
  restored 618 files and independently rechecked every output SHA-256.
  Temporary restored data/safety copies were removed after success.
- Debug-build measurements on this development machine: fresh replacement
  completed at 102.76 seconds (137.84 including independent output recheck);
  populated replacement completed at 173.63 seconds (194.31 including recheck).
  Total opt-in test: 411.96 seconds, including initial strict inspection and
  preview hashing. These are not target-machine/optimized-EXE timings.
- PASS: stale chat `.tmp` and cache pruning, fully validated Stored safety ZIP,
  native stage/count reporting and frontend invocation-channel forwarding.
  Independent review found no remaining data-loss/integrity blocker.
- PASS: optimized `tauri build --no-bundle`. Separate artifact:
  `release/portable/dev-companion-migration-progress-20261006.exe`, 16,274,432 bytes,
  SHA-256 `7EA29F162C029E6C380A2F7A5D9AC1B6AA3AFAEA569B087E3BF25F22B7FD410A`.
  Canonical EXE hash remains
  `17E403AE9CEB8C015FAD085FAC13F4FF6885F38DFFA754572C86A6812FBBCD3B`.
- NOT RUN: remote process diagnosis, actual new-machine Codex sidebar/history,
  SourceTree launch/sign-in, packaged progress/restore interaction, and NSIS.
  The previous full Rust run's unrelated home-resolution failure remains;
  that whole suite was not rerun for this follow-up.

## Current working-tree validation — 2026-10-04

The current feature set passed `pnpm lint`, `pnpm check`, `pnpm test` (28
tests), `pnpm build`, `cargo check --manifest-path src-tauri/Cargo.toml`, the
focused backup-inventory Rust test, and `git diff --check`. The full Rust
library suite had 92 passing tests and one pre-existing environment-dependent
failure in `codex_environment::tests::metadata_crud_and_removal_leave_codex_home_in_place` because the test process could not resolve the current Windows home directory. The native portable EXE/NSIS build and a packaged-app smoke test have not been rerun after the latest SourceTree, portable-storage, theme, and Dashboard changes.

## Automated checks

All commands ran successfully in `D:\projects\my-github\tools\codex-companion` after the final source changes:

| Check | Result |
|---|---|
| `node node_modules/typescript/bin/tsc -b` | Pass |
| `node node_modules/eslint/bin/eslint.js .` | Pass |
| `node node_modules/vitest/vitest.mjs run` | 13 tests pass |
| `cargo test --manifest-path src-tauri/Cargo.toml --lib --offline` | 39 tests pass |
| `./build-publish.ps1` | Pass: Windows x64 NSIS bundle and portable executable |

Rust fixtures cover legacy metadata string/object sources, ZIP traversal, archive corruption, v1 session archive compatibility, delete safety archive recovery, conflict/no-overwrite restore, rollback and rollback failure, empty destination, source locks, Windows junction rejection, cleanup allowlist/change detection, and synthetic environment archive round trips.

Frontend fixtures cover typed translations, preview/selection equality and gated environment/cleanup actions. A browser UI fixture exercised the Vietnamese environment preview, excluded credential warning, restore statuses (`new`, `manual`, `conflict`), and cleanup scan/selection/confirmation gate. It deliberately did not execute a destructive restore or cleanup operation.

## Windows deliverables

The final build command must be re-run after a source change:

```powershell
./build-publish.ps1
```

It produces:

- `release/portable/codex-companion.exe` and `portable-mode`.
- `src-tauri/target/release/bundle/nsis/Codex Companion_0.2.0_x64-setup.exe`.

Final portable executable SHA-256:

```text
63E47854E5424D7C54BF617E78C2671B392E296B37A30950A952955BE3606131
```

## Not validated

No target-machine copy or Codex Desktop reindex/reopen test was possible. The application therefore does not claim full desktop-chat migration. See [STORAGE_AUDIT.md](STORAGE_AUDIT.md) and [USER_GUIDE.md](USER_GUIDE.md).

## Phase 0–1 personal bundle delta — 2026-09-07

After the Beyond Compare source changes, `pnpm lint`, `pnpm check`, `pnpm test`
(14 tests), and `pnpm build` passed.
`cargo check --manifest-path src-tauri/Cargo.toml` passed and
`cargo test --manifest-path src-tauri/Cargo.toml --lib` passed 46 tests,
including the existing session-v1, delete-v1, and environment-v2 readers plus
new personal-bundle acknowledgement, malicious-inventory, source-revalidation,
create-new, and rollback checks. `git diff --check` passed.

`pnpm tauri build --bundles nsis` passed and produced the Windows x64 NSIS
installer. The default `pnpm tauri build` also built the release executable but
the MSI step failed because local Windows Installer access prevented WiX ICE
validation; no MSI artifact is claimed for this delta.

## Phase 2 SourceTree delta — 2026-09-07

Completed checks: `pnpm lint`, `pnpm check`, `pnpm test` (15 tests), `pnpm build`,
`cargo check --manifest-path src-tauri/Cargo.toml`, and
`cargo test --manifest-path src-tauri/Cargo.toml --lib` (52 tests) all passed.
The Rust suite includes the session-v1, delete-v1, environment-v2 and Beyond
Compare regressions plus SourceTree valid/malformed/unknown XML, URL userinfo,
traversal/ADS/reserved paths, duplicate inventory, hash/size rejection,
process-running recognition, source-change, create-new conflict and rollback
coverage. The frontend fixture verifies the SourceTree exclusions, sensitive
transport warning and manual-only copy. `git diff --check` passed. The fresh
Windows x64 NSIS bundle passed at
`src-tauri/target/release/bundle/nsis/Codex Companion_0.2.0_x64-setup.exe`
(SHA-256 `1C2A1D96340AEDFA2262197544B0F197EC33D8B77F91487C755483FD16ACD445`).
MSI was not run for this delta.

## Phase 3 XAMPP files delta — 2026-09-07

Completed checks: `pnpm lint`, `pnpm check`, `pnpm test` (16 tests), `pnpm build`,
`cargo check --manifest-path src-tauri/Cargo.toml`,
`cargo test --manifest-path src-tauri/Cargo.toml --lib` (57 tests), and
`git diff --check` all passed. `cargo check` emitted a non-fatal Windows
incremental-cache access warning; compilation completed successfully.

Rust regression coverage includes all prior session-v1, delete-v1,
environment-v2, Beyond Compare and SourceTree cases plus XAMPP selected-project
boundary, reviewed-config credential rejection from a hash-valid crafted ZIP,
credential/key filename exclusion, process recognition, source-change, strict
bundle inventory/hash and staging rollback. The frontend fixture confirms the stopped-process requirement,
exclusions, sensitive transport warning, and manual-only staging workflow.

After normal checks passed, `./build-publish.ps1` produced the Windows x64 NSIS
installer and portable executable. The fresh hashes are:

```text
release/portable/codex-companion.exe
03CB334BAEBA9AA2D3061368763B00AF3BF44BCD6FF67E3BE5721A495C17D5F2

src-tauri/target/release/bundle/nsis/Codex Companion_0.2.0_x64-setup.exe
B6521680F8C72DED9A474C2C13854FCB1EA26F84A75A98428EE828D4A05486E4
```

MSI was not run.

## Dev Companion branding and Beyond Compare credential-selection delta — 2026-09-08

Completed checks: `pnpm lint`, `pnpm check`, `pnpm test` (16 tests), `pnpm build`,
`cargo check --manifest-path src-tauri/Cargo.toml`,
`cargo test --manifest-path src-tauri/Cargo.toml --lib` (57 tests), and
`git diff --check` all passed. `cargo check` emitted a non-fatal Windows
incremental-cache access warning; compilation completed successfully.

The Beyond Compare regression verifies that an explicit credential-included
choice is recorded and remains inspectable by the strict personal-bundle-v1
reader. Existing SourceTree, XAMPP, session-v1, delete-v1 and environment-v2
coverage remains in the same test run. After the normal checks passed,
`./build-publish.ps1` produced:

```text
release/portable/dev-companion.exe
8BCD6559EFFFCC4D9AF579A130DB1C66A6A896C2745AA8464CA1454BE4C89282

src-tauri/target/release/bundle/nsis/Dev Companion_0.2.0_x64-setup.exe
D3C4AB4B592F960854BD896FA45A087B36DBA5C91FA117C0816586F23E19F55C
```

MSI was not run. No target-machine Beyond Compare import or passphrase-encrypted
bundle test has been performed; native import remains manual and the bundle is
not password-protected.

Independent Sol review found no blockers: existing Codex archive readers and
the legacy application-data path remain unchanged; personal-bundle-v1 accepts
both the established `true` and newly recorded `false` credential flag; the UI
keeps opaque-data, unencrypted-transport and manual-import warnings; and no
generic provider framework was introduced.

## Repository rename and Conversations responsiveness delta — 2026-09-09

Completed checks: `pnpm lint`, `pnpm check`, `pnpm test` (18 tests), `pnpm build`,
`cargo test --manifest-path src-tauri/Cargo.toml --lib --offline` (60 tests),
and `git diff --check` all passed.

The Rust tests include worker-thread execution, serialized blocking operations,
and recovery after a blocking-worker panic. The Conversations test renders
2,000 rows in English and Vietnamese and verifies a single date formatter per
language. A fresh portable-app smoke test with 2,000 synthetic local sessions
rendered a sort change in about 122 ms; Refresh immediately entered its loading
state, completed, and left the application window responsive.

`./build-publish.ps1` produced the Windows x64 NSIS installer and portable
executable:

```text
release/portable/dev-companion.exe
D2C494B3F3225F9318EDD085D3CF7220D1046C61E937D0737537E736529429E3

src-tauri/target/release/bundle/nsis/Dev Companion_0.2.0_x64-setup.exe
3396E6200FB59C35412631BA7490114B38F1DAE1FEF76318708469E1564DEE90
```

MSI was not run. The portable smoke test is not an installer installation test,
and no claim is made for Desktop chat/database merging.

## Migration overwrite and recovery layout — 2026-10-06

Latest copy-progress delta:

- PASS: `pnpm check`, `pnpm lint`, all 34 frontend tests, 13 focused migration
  tests and four command tests, independent review, `git diff --check`, and
  optimized `tauri build --no-bundle` (including fresh frontend build).
- PASS: blocked UI channel does not hold the native producer; the observer
  delivers the last state without another native event. Chunked copy retries
  interrupted reads, handles partial writes, rejects write failure and retains
  output SHA verification/rollback. Late frontend callbacks are ignored.
- PASS: supplied 618-file ZIP with actual asynchronous trace, isolated fresh
  and populated targets, and every output independently SHA-checked. Fresh
  restore completed at 71.72s (including independent checks: 94.77s); populated
  at 164.28s (including checks: 187.01s). Whole debug test: 371.94s.
- NOT RUN: new-machine packaged restoration, actual source/destination I/O
  stall diagnosis and Codex Projects menu. Full native suite was not repeated
  for this delta; the earlier unrelated home-resolution failure remains below.

New artifact: `release/portable/dev-companion-migration-copy-progress-20261006.exe`
(16,409,088 bytes), SHA-256
`7B8FFE7A8FA897D12F78980B4F573A6F44D2486BE321865B72EFF7A80E7D0F6D`.
The canonical EXE was not created/overwritten; retain the portable marker and
existing config/backups beside this separately named file.

- PASS: `pnpm check`, `pnpm lint`, `pnpm test` (32), `pnpm build`,
  `git diff --check`, and optimized `tauri build --no-bundle`.
- Native library run: 91 passed, 1 ignored, 1 failed. All migration and
  SourceTree configuration tests passed. The existing environment-manager
  metadata test failed resolving the current user home; no unrelated fix was
  made. The ignored real-archive test was run separately and passed.
- PASS: isolated restore of the user's actual 367 MiB Codex migration ZIP
  into a fresh-install fixture with existing empty global state. All 618
  restored files matched the archive SHA-256 inventory. No live Codex data
  was changed; temporary restored content was removed after validation.
- PASS: four synthetic SourceTree files overwrite and hash-verify in both
  Local/Roaming destinations, independent safety copies, forced restore/hash
  failure rollback, incomplete rollback reporting, and owned-temp cleanup.
  The user's encrypted SourceTree ZIP inventory contains all four primary
  files and `user.config`; actual decryption/restore of that ZIP was NOT RUN.
- PASS: browser fixtures using the shared production styles and both recovery
  panels at 847px and 600px; buttons/panels/nested lists remain readable.
- Independent review found no remaining blockers. Real new-machine Codex
  Desktop sidebar/history, SourceTree launch/sign-in, packaged-app interaction,
  and NSIS installation are NOT RUN.

Portable test artifact: `release/portable/dev-companion-migration-fix-20261006.exe`.
SHA-256: `F55215749187A881D66AFA8C124D11BC3EBC3FF4C3D80C334A2C449C8AAA7B3F`.
The canonical `release/portable/dev-companion.exe` was preserved. The test file
uses the same adjacent `portable-mode`, `config/`, and `backups/` conventions.

To repeat the optional real-archive check without touching the live profile:

```powershell
$env:DEV_COMPANION_VALIDATION_ZIP = '<path to a Codex migration ZIP>'
cargo test --manifest-path src-tauri/Cargo.toml --lib codex_migration::tests::real_archive_overwrites_fresh_install_and_verifies_every_file -- --ignored --nocapture --test-threads=1
```

## 2026-10-14 — Project Cleanup recycle choice and archive button

- PASS: local TypeScript type-check, source ESLint, all 46 frontend tests (including mode/default IPC and retained-output handoff), full Rust library suite (132 passed, 0 failed, 5 ignored), focused Project Cleanup tests (22 passed, 0 failed, 3 ignored), and scoped independent review.
- PASS: actual Windows Recycle Bin opt-in tests using only uniquely named owned fixtures under src-tauri/target: one cache file and one directory with two nested tiny files. Shell confirmed a Recycle Bin item, source/stage paths disappeared, and the tiny fixture items remain in the bin. No global bin cleanup or live project deletion occurred.
- PASS: synthetic browser interaction with production ProjectCleanupPage/CompressionPage: default recycle; selecting candidates; changing mode invalidates preview and typed confirmation; both mode-specific confirmations execute with the preview token; result archive action loads the completed Unicode source; pending readiness disables source changes; Everything filters retain dist/package.json; archive still requires explicit preview/start. No filesystem operation or 7-Zip creation was performed in the browser fixture.
- PASS: deterministic staging recovery tests for null/full-delete restoration, partial source retention, replacement collision, source disappearance during validation, confirmed recycling with stage-cleanup warning and truthful counters, and recovery-stage scan/root/candidate guards.
- NOT RUN: interactive Explorer restore, real user-project cleanup/archival, packaged-app interactions, NSIS installation, unsupported-volume/no-space Windows recovery behavior. Stop builds/watchers before cleanup; recycle requires verifiable same-volume hardlinks and does not free disk space until the bin is emptied.

- PASS: optimized `tauri build --no-bundle` (fresh frontend production build), final `git diff --check`.
- New separate portable artifact: `release/portable/dev-companion-cleanup-recycle-archive-20261014.exe` (16,864,768 bytes), SHA-256 `9E34FFC3E150B970AE2433371D72985ED2D891A83C6067DDF109D7C8E8DF50E5`. Canonical portable EXE was not overwritten; no commit/push.

## 2026-10-14 — pnpm linked node_modules cleanup

- PASS: TypeScript check, source-only ESLint, 46 frontend tests; full Rust library suite 135 passed / 0 failed / 6 ignored; focused Project Cleanup 25 passed / 0 failed / 4 ignored; scoped independent review and git diff whitespace check.
- PASS: explicit read-only scan of D:/projects/cus-gitlab/shinshin: incomplete=false, node_modules files=13,714 / bytes=82,051,560 / blocked=None. No real project deletion or installation was performed.
- PASS: owned permanent fixture with internal junction and external hardlink sentinel preserved; ancestor cycle and external junction rejection; junction retargeted from internal A to B after preview aborts before deleting either target.
- PASS: actual opt-in Windows Recycle Bin smoke for a tiny owned pnpm-like directory. Raw junction recovery staging succeeded, candidate/staging paths were gone after confirmation, outside hardlink sentinel stayed unchanged. One tiny test directory remains in the bin; no global bin cleanup. No owned fixture/recovery directories remain under target.
- NOT RUN: interactive Explorer restoration or cleanup/archival of the user's real project in the packaged app; file-symlink privilege behavior on other machines, non-NTFS/unsupported reparse destinations. External/workspace links and linked roots/caches remain blocked. Logical content bytes are displayed without claiming physical disk-space reclamation; shared pnpm store remains out of scope.

- PASS: optimized Tauri no-bundle build with fresh frontend production build. Separate artifact: release/portable/dev-companion-pnpm-cleanup-fix-20261014.exe (16879616 bytes), SHA-256 F2D5392391958BE57DE9D82D4547DB3391F6BAA7062C9998C593CB9835FF926E. Canonical portable EXE preserved; no commit/push.

## 2026-10-14 — Canonical path Recycle Bin fix

- REPRODUCED before fix: owned pnpm fixture passed its canonical verbatim path into Windows Shell and failed with the user's exact E_INVALIDARG (0x80070057), before mutation. Earlier smoke had only passed ordinary paths.
- PASS after fix: same actual Windows Bin fixture with canonical path, spaces/Vietnamese parent, internal junction and preserved outside hardlink sentinel. No temporary stage/fixture directory remains; one tiny owned test item remains in the bin. No live project was deleted.
- PASS: full native suite 135 passed / 0 failed / 6 ignored, rustfmt on changed native file. Shell input strips only the verbatim-drive prefix in UTF-16; original canonical paths and safety/recovery checks are unchanged.
- NOT RUN: deletion/recycling of the user's real project, interactive Explorer restore or packaged-app interaction. No frontend behavior was changed for this fix.

- PASS: optimized Tauri no-bundle build with fresh frontend production build. Separate artifact: release/portable/dev-companion-recycle-shell-fix-20261014.exe (16886272 bytes), SHA-256 73B8C46A00E3F6367ADB0F31F26FCD8E84B050B72D8C12D71B605E74FDDD529F. Canonical portable EXE preserved; no commit/push.
