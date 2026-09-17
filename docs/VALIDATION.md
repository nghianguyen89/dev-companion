# Validation — 2026-09-07

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
