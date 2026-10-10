# XAMPP handoff — 2026-10-08

Read this first when continuing from another ChatGPT/Codex account. This file
is a repository handoff, not a request to run every operation listed below.

## Current objective and authorization

The user requested a complete UI for XAMPP domain management alongside backup:
domain + arbitrary source folder, HTTPS, first-use CA identity form, XAMPP
detection/folder selection, automatic Apache start/restart, optional directory
listing and optional LAN access. Create/edit/delete are implemented.

The user authorized changes to the copied C:/xampp installation, archiving old
custom files before cleanup. That cleanup and initialization already happened;
do not repeat them or reset the installation just to recover chat context.
No commit or push was requested. Preserve all current working-tree changes.

## Read order and implementation map

Read AGENTS.md and its required repository documentation first, then this file,
the 2026-10-08 sections of ARCHITECTURE.md, IMPLEMENTATION_PLAN.md,
VALIDATION.md and USER_GUIDE.md. Older phase descriptions are historical;
XAMPP backup and the new live domain manager have different contracts.

- Native implementation: src-tauri/src/xampp_domains.rs.
- Async serialized command bridge: src-tauri/src/commands.rs; registration in
  src-tauri/src/lib.rs. Reuse run_blocking and existing COMMAND_GATE.
- Existing backup root now uses saved installation: src-tauri/src/xampp.rs.
- UI: src/features/backup/XamppDomains.tsx inside XamppPage.tsx.
- UI checks: XamppDomains.test.tsx; DTOs: src/types/codex.ts;
  bridge: src/services/tauri.ts; labels: src/i18n/en.ts and vi.ts;
  scoped styles: src/app/global.css.

## Runtime state and recovery

Portable executable: D:/projects/my-github/tools/dev-companion/release/portable/dev-companion.exe.
Keep portable-mode beside it. Domain state is configs/xampp beside backups;
existing general portable settings still use config (singular).

Current installation: C:/xampp. Domain settings:
D:/projects/my-github/tools/dev-companion/release/portable/configs/xampp/settings.json.
Generated httpd-domains.conf and public ca.crt are in the same directory.
Private CA/leaf material is C:/xampp/apache/conf/dev-companion, with restricted
ACLs. Do not print, persist in docs, export or share private key contents.
Portable configs alone do not transfer CA private keys or install trust on
another machine. Ordinary XAMPP backup remains htdocs/config-only and excludes
private keys; it requires Apache/MariaDB-related processes stopped.

The valid existing CA was adopted and Windows machine root trust installed.
Four imported domains: localhost -> C:/xampp/htdocs;
cus-projects.local -> D:/projects/cus-svn;
cus-wordpress.local -> D:/projects/cms/wordpress;
cus-gitlab.local -> D:/projects/cus-gitlab. Read settings.json for current
values because the user can change these in the UI after this snapshot.
All initially have www enabled, redirect and LAN disabled; directory listing
is disabled for localhost and preserved enabled for the three legacy sites.

Initial safety directory:
C:/xampp/backup/dev-companion-1791426653180055000.
recovery.json lists 47 original/absent target records and safety paths.
Identified legacy custom makecert/CNF/certificate files were archived before
removal; stock makecert.bat and vendor/unrelated files were retained.
Keep the safety archive. Recovery is guided by the manifest, not a blind
whole-folder overwrite. Subsequent UI edits can create additional archives.

Apply requires Administrator, validates inputs/conflicts, writes managed
hosts/config blocks, checks Apache syntax, applies public CA trust and optional
Private-profile LocalSubnet firewall access, then starts/restarts the selected
Apache executable. HTTP revision header confirms reload. Failures restore
files/settings and attempt runtime/trust/firewall recovery. First initialization
without an earlier managed revision cannot prove old-site reload on rollback;
the result explicitly reports that limitation.

LAN clients need hosts/DNS pointing names to server LAN IP and trust of the
exported public CA on each client. Never distribute the CA private key.

## Validation already performed

See VALIDATION.md for exact PASS/NOT RUN boundaries. Frontend check/lint,
38 tests, cargo check, 10 focused XAMPP native tests, isolated real OpenSSL/Apache
CA/CRUD/conflict/restart smoke and explicit elevated live initialization passed.
HTTPS HEAD requests to all four real domains returned 200 with Windows trust.
Browser fixture covered real React components with mocked native state;
this is not proof of packaged-app native UI behavior.

build-publish.ps1 completed production frontend, release Tauri, NSIS and canonical
portable publication. SHA-256 at build time:
87B575055FF3F37E050C7E5F1C5255BD200F0C19DF6521BEAD5A7985D73D9983.
Installer: src-tauri/target/release/bundle/nsis/Dev Companion_0.2.0_x64-setup.exe.
No version bump, commit or push. Temporary browser/UAC scripts and logs cleaned.

## Recommended next work

1. Inspect git status/diff and current settings; do not reimplement the feature.
2. Verify packaged portable UI with Administrator: create a disposable domain
   outside htdocs, open HTTP/HTTPS, edit folder/options, toggle listing, delete;
   source folder must remain. Verify restart, conflicts and error display.
3. If a second LAN machine is available, enable LAN on a test domain, export
   public CA, configure client name resolution/trust, verify HTTPS and scope.
4. NSIS installation on a target machine remains NOT RUN. Full Rust library
   tests were not rerun for this feature; older runs have a known unrelated
   current-user-home resolution failure. Do not describe focused checks as a
   full-suite pass or fix that unrelated issue without an actual need.
5. Fix concrete failures narrowly, validate changed behavior and update docs.
   No release/commit/push unless the user requests it.

## Repeating checks when needed

```powershell
pnpm check
pnpm lint
pnpm test
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml --lib xampp
```

Optional isolated real test requires installed C:/xampp binaries; restart smoke
also needs ports 80/443 available. The live installation is now running, so do
not run a port-binding fixture concurrently with it:

```powershell
$env:DEV_COMPANION_XAMPP_RESTART_SMOKE = 'YES'
cargo test --manifest-path src-tauri/Cargo.toml --lib real_openssl_apache_isolated -- --ignored --nocapture --test-threads=1
```

Do not run initialize_copied_xampp_live to gather context: it mutates live
C:/xampp, trust, hosts and portable settings and needs explicit elevated setup.

Tooling note: default nvm Node was broken during this session. Bundled Node
works by prepending C:/Users/nghia/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin
and its dependencies/bin to PATH for the process. Native UAC validation used
Windows PowerShell; an inherited-output helper remained after the test and was
closed by exact owned PID/command-line verification, leaving Apache running.
Do not broadly kill httpd or unrelated PowerShell processes.

## Continuation check — 2026-10-08

The canonical EXE hash is unchanged. The actual elevated portable XAMPP page
was inspected through Windows UI Automation; it shows the saved installation,
four domains and trusted CA with management enabled. Current settings have
cus-projects.local redirectHttps=true; preserve that current value rather than
restoring the earlier snapshot above.

All 16 HTTP/HTTPS HEAD checks (four names and www, both schemes) passed after
redirects, with revision 1791428482622830600. curl needs an unsandboxed Windows
context and --ssl-revoke-best-effort for this local CA; CA-chain/hostname checks
stay enabled. Strict revocation failed because no revocation information is
available. A non-admin Apache syntax probe cannot read the restricted certs.

Packaged UI CRUD/restart/options remain NOT RUN. The prepared disposable
Windows UI Automation script initially failed before mutation because Windows
PowerShell needs a UTF-8 BOM for Vietnamese labels. Encoding is corrected, but
the next UAC request was canceled; do not retry elevation without the user's
answer to the pending clarification. No new domain was created or removed.
Ignored release/xampp-ui-validation/ holds scripts and read-only evidence;
prepare.cjs adds the BOM before launch.ps1 -Test. Test names are
dc-ui-20261008.test and dc-ui-renamed-20261008.test, with source fixtures outside
htdocs in that test directory. The corrected harness is not yet validated.

Two owned elevated validation windows were opened (32172 and 36412), separate
from the pre-existing portable PID 31328. Check exact executable path/PID before
closing only those validation windows; Apache and the original window must stay.
No product-code changes, rebuild, commit or push were performed in continuation.

## Browser security warning follow-up — 2026-10-08

User reports warnings for all current URLs on this same machine. Public peer
certificates have matching SANs/signatures and valid dates; the exported CA is
present in both Windows machine/user Root stores and absent from Disallowed.
Installed Edge 154 in an isolated headless context loaded all four HTTPS sites
with HTTP 200/TLS 1.3 and visibleSecurityState=secure, with certificate checks
enabled and no bypass. This does not resolve the user's active-profile report.
Firefox is running, but no actual error code was captured; its cause is not
established. Await the requested browser name/error code (or warning screenshot).
Do not regenerate CA, reinitialize XAMPP, import certificates again or change
Firefox preferences speculatively. See VALIDATION.md for exact boundaries.

The subsequently supplied _screenshot_/bug-broswer.png shows
https://https//cus-projects.local/ in Firefox and Edge. Both fail resolving
host "https" (Server Not Found / DNS_PROBE_FINISHED_NXDOMAIN), before TLS.
The correct URL is https://cus-projects.local/, which already passed the
isolated Edge test. Ask for active-browser confirmation only if still needed;
do not alter CA/trust/Apache to address the malformed URL in this screenshot.

## Domain-list layout update — 2026-10-08

Implemented the user's layout reference in XamppDomainEntry.tsx: green SSL
when initialized+caReady (independent of redirect), blue enabled www/muted off,
primary domain open/copy, folder link and tooltip-labelled listing/edit/delete.
User chose copying the complete URL; open/copy never add www. Read-only open
commands validate against persisted primary domains; folder paths are resolved
from settings and safety/existence checked. No added dependency.

check/lint/40 frontend tests, 11 focused Rust XAMPP tests, responsive React
browser fixture (mock native bridge) and optimized build passed. The new EXE
is release/portable/dev-companion-domain-layout.exe; it uses the adjacent
portable-mode/configs. The canonical old EXE/hash and existing installer above
remain unchanged. Use the new EXE to review this layout; close the old app first.
Prepared packaged smoke found no UI Automation descendants, so actual new
copy/open/folder clicks and live row CRUD/listing remain NOT RUN, not PASS.
No existing domains, CA/trust or Apache were mutated; no commit/push.
ESLint ignores generated release artifacts; build used a local absolute-Node
hook override and approved unsandboxed linker retry. Details: VALIDATION.md.

Action follow-up: user reported disabled index/edit/delete and a flash on copy.
Row actions now allow opening draft/edit/delete panels without Administrator;
Edit scrolls to and focuses the form. Non-admin listing/delete display contextual
elevation guidance, while Save/confirmed Delete and all native writes retain
their gates. Copy never toggles page busy or the full result card; a green check
appears inside its existing button for two seconds. Frontend check/lint/41 tests
and delayed-copy/non-admin browser regressions passed. Actual packaged elevation
and live writes remain NOT RUN; no XAMPP state was changed for this fix.
The native elevation request now uses SW_SHOWNORMAL instead of SW_HIDE; the
previous value hid the newly opened Administrator window. Actual UAC acceptance
still requires the user's interactive verification.
Focused Rust tests passed (11, with 2 opt-in ignored) and optimized build passed.
Use release/portable/dev-companion-domain-actions-fix.exe for this follow-up;
close the previous app first. The adjacent portable marker/configs are reused,
and previous executables remain intact. SHA256:
0dabebeb41246b6692acd9cf6226e14e3de8a14bdd97189c5e5b452c94d6e642.

SSL follow-up supersedes the initial CA-only row indicator: saved redirectHttps
now controls SSL color and primary open/copy scheme together with CA readiness.
Tooltips say HTTPS redirect enabled/disabled; disabling redirect retains backend
TLS certificates/vhosts. Type-check/lint/42 tests and real React mocked-bridge
edit-save-off/on regression passed. No live domain mutation or commit/push.
Optimized build passed; use release/portable/dev-companion-domain-ssl-fix.exe
after closing the older app. Previous executables/portable configs preserved.
Live backend acceptance on this build remains NOT RUN.

Storage follow-up: use release/portable/dev-companion-configs.exe now. Shared
app settings/history/logs join XAMPP under configs; legacy config is migrated
on load/save with link/collision guards. The local portable's settings moved
unchanged to configs/settings.json; its emptied config directory was removed.
Safety copy retained under release/xampp-ui-validation. Old EXEs remain but
would write config again; close them and use the new EXE. Full Rust suite
111 PASS/3 ignored and optimized build PASS; native interactive acceptance
still NOT RUN. No Apache/CA/domain settings changes or commit/push.

Retention follow-up: new safety archives use operation.json outcome metadata.
KeepRecent defaults to 10 (configurable 1–100); UI shows validated count/bytes
and manual cleanup. Auto cleanup follows successful live apply. Initial/oldest,
failed/pending and legacy unknown-outcome archives stay protected; existing
C:/xampp/backup archives were not relabeled or removed. Invalid/linked/extra
content is preserved. Total copies can exceed keepRecent because protected
archives do not count toward the regular completed-backup limit.
Current retention executable: release/portable/dev-companion-xampp-retention.exe
(SHA-256 5F913AA1A4380C953264EACBFD4478B9AAE23CE6AD25C522739AD4D552D18AAA).
Close older copies before use. Old EXEs reject the new backupKeepRecent field
once saved, so use the current executable thereafter. Frontend 42/full Rust113
passed (3 opt-in ignored), optimized build/review passed. Packaged retention
interaction and live pruning remain NOT RUN. No actual backups were removed.
