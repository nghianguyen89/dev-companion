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
  `user.config`; direct restore needs `RESTORE`, takes a safety copy, and
  exposes a post-restore `DELETE` action. Windows Vault, OAuth/DPAPI-bound
  secrets and SSH keys remain excluded, so target-machine sign-in may still be
  required.
- Portable personal bundles are written to `backup/` beside `dev-companion.exe`
  when the adjacent `portable-mode` marker is present; installed builds retain
  the existing AppData location.
- Dashboard is the first navigation group and shows current local feature
  counts plus read-only direct-file inventories for session backups and
  personal application bundles.

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

- No active feature work.

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

## Important Recent Decisions

- Product branding is Dev Companion; existing `codex-companion` storage and
  archive identifiers remain for compatibility.
- The 2026-09-09 portable smoke test exercised 2,000 synthetic conversations:
  sort rendered in about 122 ms and Refresh completed with the window responsive.

## Next Recommended Work

- Run the new NSIS installer on a target machine before release; do not claim
  Desktop chat migration until its manual workflow is verified.
