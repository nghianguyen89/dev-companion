# Validation — 2026-09-07

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

## 2026-10-08 XAMPP domain manager

- PASS: pnpm check, pnpm lint, pnpm test (19 files / 38 tests), cargo check,
  focused native XAMPP tests (10 passed; two opt-in tests run separately),
  rustfmt and git diff --check.
- PASS: opt-in isolated real OpenSSL/Apache integration with CA creation,
  domain/www certificates, add/delete, hosts conflict rollback, start,
  revision-confirmed restart, and shutdown cleanup.
- PASS: explicit elevated initialization of the copied C:/xampp installation.
  Existing CA reused and machine-trusted; four existing domains imported.
  Windows-trusted HTTPS requests returned HTTP 200 for localhost,
  cus-projects.local, cus-wordpress.local and cus-gitlab.local.
  Legacy custom files were backed up, then removed; vendor makecert.bat kept.
  Safety directory: C:/xampp/backup/dev-companion-1791426653180055000.
  recovery.json records original targets and their verified safety copies.
- PASS: browser fixture exercising the production React domain component
  and styles with mocked native state: create, edit/rename, listing and delete
  confirmation. This does not validate native packaged-app interaction.
- PASS: build-publish.ps1 production frontend, optimized Tauri executable and
  Windows x64 NSIS bundle; canonical portable executable updated.
- NOT RUN: LAN access from another computer, packaged-app interactive CRUD,
  and installing the NSIS bundle on a target machine.

Portable: release/portable/dev-companion.exe
SHA-256: 87B575055FF3F37E050C7E5F1C5255BD200F0C19DF6521BEAD5A7985D73D9983
Installer: src-tauri/target/release/bundle/nsis/Dev Companion_0.2.0_x64-setup.exe
Portable domain configuration: release/portable/configs/xampp/settings.json.

### 2026-10-08 continuation: portable read-only acceptance

- PASS: canonical portable EXE SHA-256 still matches the value above. Windows
  UI Automation opened the real elevated portable XAMPP page, showing the saved
  C:/xampp installation, four domains, trusted CA and enabled management controls.
  This is a packaged native overview check, not CRUD acceptance.
- PASS: 16 read-only HTTP/HTTPS HEAD checks (four domains, each with www,
  both schemes) reached 200 after following redirects; responses contained
  the current generated revision 1791428482622830600. Current settings have
  redirectHttps enabled for cus-projects.local; that value was preserved.
  localhost redirects to its dashboard; www redirect targets the primary name.
  curl used Windows Schannel with --ssl-revoke-best-effort, retaining CA-chain
  and hostname verification; no --insecure or custom trust bypass was used.
- FAIL (probe/tooling): sandbox Schannel returned SEC_E_NO_CREDENTIALS; outside
  the sandbox strict revocation returned CRYPT_E_NO_REVOCATION_CHECK for the
  local CA without revocation information. The best-effort checks above passed.
  Non-admin Apache -t could not access the restricted certificate directory;
  this does not establish invalid running Apache configuration.
- FAIL (test harness): Windows PowerShell misread Vietnamese UI labels from
  the UTF-8 script without a BOM. No domain mutation occurred. The script now
  has a BOM; its corrected CRUD execution remains unverified because the next
  UAC request was canceled.
- NOT RUN: portable create/edit/delete, listing and LAN toggle/firewall checks,
  automatic restart through that UI, GUI folder picker/public CA export, a
  second LAN client, NSIS installation, and elevated Apache syntax rerun.
  Initial setup/cleanup and a port-binding fixture were intentionally not repeated.
- No application defect was established; no product-code fix, rebuild,
  commit or push was performed. Read-only evidence and prepared disposable UI
  harness are under ignored release/xampp-ui-validation/. Two owned elevated
  validation windows (PIDs 32172 and 36412) were opened; the pre-existing
  portable window and Apache were left running. Do not terminate by process name.

### 2026-10-08 browser security-warning investigation

- User reports security warnings for all existing URLs on the XAMPP machine;
  exact browser error code remains unavailable. Do not mark this report resolved.
- PASS: public TLS peer metadata for all eight domain/www names matches SANs
  and signatures against the exported CA. Leaf certificates are valid for one
  year from 2026-10-08, with serverAuth usage; the CA expires in March 2035.
  Metadata inspection alone is not a browser trust check.
- PASS: the exported CA fingerprint matches LocalMachine/Root and
  CurrentUser/Root; no same-subject CA was found in either Disallowed store.
- PASS: installed Edge 154.0.4258.62, isolated headless context with
  ignoreHTTPSErrors=false, loaded all four primary HTTPS URLs with HTTP 200,
  TLS 1.3 and Chromium visibleSecurityState=secure, with no security issue IDs.
  No certificate bypass or custom CA was supplied to this browser check.
- NOT RUN: reproducing the warning in the user's active browser/profile;
  Chrome is not installed at Playwright's expected path. Firefox is running,
  but no error code was exposed in the visible UI Automation tree, and no
  explicit enterprise-roots override was found in the selected prefs.js.
  Those observations do not establish Firefox as the cause.
- No Apache/CA/trust/browser-profile mutation or product fix was performed.
  Safe public-certificate metadata and isolated browser results are retained
  under ignored release/xampp-ui-validation/. Await the actual warning code
  before changing certificates or browser trust.

- Screenshot clarification: _screenshot_/bug-broswer.png shows the malformed
  URL https://https//cus-projects.local/ in both Firefox and Edge. Firefox
  reports Server Not Found for host "https"; Edge reports
  DNS_PROBE_FINISHED_NXDOMAIN. This captured failure occurs during name
  resolution, before certificate validation. Correct URL:
  https://cus-projects.local/. Correct URL passed the isolated Edge checks
  above; user confirmation in the active browser remains pending. No CA or
  Apache change is required for the malformed URL shown in this screenshot.

### 2026-10-08 domain-list layout

- PASS: pnpm check, pnpm lint, pnpm test (19 files / 40 tests), rustfmt,
  git diff --check and focused cargo test --lib xampp (11 passed, two opt-in
  integration tests ignored). Native tests reject unknown names, www aliases,
  URLs and unsafe strings for the new open actions.
- PASS: real React App/browser fixture with mocked native bridge and clipboard:
  SSL independent of redirect, active/muted www, open/copy primary HTTPS URL,
  HTTP fallback, folder dispatch, listing toggle, edit and exact-name delete
  guards, accessible tooltip labels and non-admin read-only actions. No row
  overflow at 1100/847/600px, including a long domain/path. Screenshots and
  fixture evidence are in ignored release/xampp-ui-validation/.
- PASS: production frontend and optimized Tauri executable (build --no-bundle).
  New executable: release/portable/dev-companion-domain-layout.exe, beside the
  existing portable-mode marker and existing configs. Canonical old EXE and
  NSIS installer were not replaced; no commit/push or live XAMPP mutation.
- FAIL (build/tooling, recovered): inherited Node lookup prevented the first
  build hook; a local build-config override with the bundled Node absolute path
  succeeded. Sandbox linker could not overwrite a prior build DLL; the
  explicitly approved unsandboxed build passed. Initial lint also scanned
  generated scripts/WebView resources under release; ESLint now excludes that
  generated artifact directory, consistent with its existing dist/target ignores.
- FAIL (automation harness): UI Automation returned zero descendants for its
  owned new portable window, so it could not find navigation controls even
  after waiting and accepting both EN/VI labels. Owned test windows were closed.
  This does not establish a render defect, and does not count as native UI PASS.
- NOT RUN: packaged click verification of clipboard/default-browser/Explorer
  behavior, new-row CRUD/listing with live backend, LAN-client and NSIS install.
  The prepared smoke did not reach domain actions; existing domain settings,
  certificates, trust and Apache were left unchanged. Full Rust suite not rerun.

### 2026-10-08 domain action/copy follow-up

- User reports three management icons cannot be used and copying causes a
  flash. Source diagnosis: row controls shared the Administrator-dependent
  canManage gate; Edit changed the off-screen form without scrolling. Copy
  reused the page-wide busy/result wrapper, inserting/removing pending content.
- Changed: row interactions require initialized installation, not Administrator
  merely to open an edit/delete panel. Edit scrolls/focuses the form. Non-admin
  listing/delete reveal contextual elevation guidance; Save and confirmed Delete
  remain permission/setup-gated and native Administrator checks are unchanged.
  Copy uses a dedicated clipboard handler and temporary in-button green check,
  without changing busy or inserting the full result card.
- PASS: frontend type-check, lint, 41 tests (19 files). Browser regression uses
  a delayed clipboard promise: busy remains false, row coordinates and panel
  height remain unchanged, and other buttons stay enabled. Non-admin edit fields
  and focus work; listing shows a notice without calling save; confirmed Delete
  remains disabled. Mocked bridge tests are not packaged native acceptance.
- Source diagnosis also found elevation using ShellExecuteW SW_HIDE. It now
  requests SW_SHOWNORMAL so the user-requested elevated app is visible.
- PASS: focused Rust XAMPP tests (11 passed, 2 opt-in ignored), production
  frontend and optimized Tauri build. Sandbox linker retry required approved
  unsandboxed execution; final build passed. New portable executable:
  release/portable/dev-companion-domain-actions-fix.exe (17185280 bytes), SHA256
  0dabebeb41246b6692acd9cf6226e14e3de8a14bdd97189c5e5b452c94d6e642.
  Canonical portable EXE retained with its original hash; git diff --check passed.
- NOT RUN: actual UAC interaction with these updated controls; no domain
  mutation or CA/Apache changes in this fix. Full Rust suite not rerun.

### 2026-10-08 SSL indicator follows saved domain option

- Diagnosis: SSL and open/copy used installation-wide CA readiness, ignoring
  the individual saved redirectHttps checkbox. This kept SSL green after save.
- Changed: SSL active and HTTPS URL now require CA readiness AND redirectHttps;
  tooltip identifies the redirect state. Existing backend continues to provide
  TLS vhosts/certificates even with HTTP redirect disabled.
- PASS: frontend type-check, lint, 42 tests (19 files); real React mocked-bridge
  browser edit/uncheck/save makes SSL muted and open/copy HTTP, recheck/save
  restores green; www and delayed-copy/layout regressions also passed.
- FAIL (build tooling): sandbox linker LNK1104 writing the existing DLL;
  approved retry outside sandbox recovered it.
- PASS: production frontend and optimized Tauri build; current portable is
  release/portable/dev-companion-domain-ssl-fix.exe. Previous executables retained.
- NOT RUN: updated portable live domain writes, LAN client and NSIS install.
  No live domain/CA/Apache settings changed; no Rust changes or full Rust rerun.

### 2026-10-08 unified configs storage

- PASS: legacy settings/history/logs migrate preserving bytes and existing XAMPP
  state; collisions preserve both folders before moving; junctions rejected;
  old log paths remap within the known root and traversal/outside paths do not.
- PASS: full Rust library suite outside sandbox, 111 passed, 3 opt-in ignored;
  production frontend and optimized Tauri build. No frontend source changed.
- FAIL (recovered): first junction fixture used forward slashes with mklink;
  corrected path construction passed. Sandbox full suite had 110 PASS/1 FAIL
  due to unavailable user home; unsandboxed retry passed all 111 tests.
- FAIL (permissions, recovered): sandbox denied moving the portable settings
  file. Approved retry moved it; SHA256 matches the retained pre-move copy in
  release/xampp-ui-validation/settings-before-configs-merge.json. Legacy config
  directory removed only after empty; configs/xampp unchanged.
- PASS: git diff --check. New portable: release/portable/dev-companion-configs.exe;
  canonical old executables retained. Do not use old versions that write config.
- NOT RUN: packaged interactive migration/elevation, LAN client, NSIS install.
  No XAMPP reinitialization/cleanup, Apache/CA changes, commit or push.

## 2026-10-08 XAMPP safety-backup retention

- PASS: pnpm check, pnpm lint, all 42 frontend tests, cargo check,
  focused XAMPP run (14 passed, 2 opt-in ignored), full Rust library suite
  (113 passed, 3 opt-in ignored), and scoped independent review.
- PASS: isolated retention fixtures prove newest-N pruning, earliest/initial,
  failed/pending/legacy protection, preservation of unexpected files/empty
  directories, rejection of tampered recovery paths, and Windows junction
  rejection without touching the external target. Old settings default to
  10; values outside 1–100 are rejected.
- Native cleanup tests delete fixture-owned backups only. All eight real
  C:/xampp/backup directories remain; no live domain/CA/hosts changes were made.
- NOT RUN: packaged GUI save-policy/clean action or live automatic pruning
  after Apache restart. Existing outcomes cannot be inferred retrospectively;
  old unknown-state backups intentionally remain protected.

- PASS: optimized Tauri no-bundle build; production frontend included.
  New portable: release/portable/dev-companion-xampp-retention.exe.
  SHA-256: 5F913AA1A4380C953264EACBFD4478B9AAE23CE6AD25C522739AD4D552D18AAA.
  Existing portable executables preserved; NSIS was not rebuilt for retention.
- PASS: git diff --check after final code/doc changes.
