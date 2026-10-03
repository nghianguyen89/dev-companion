# Debug Notes

## Quick diagnostics

Run these from the repository root before changing code:

```powershell
pnpm check
pnpm lint
pnpm test
pnpm build
cargo check --manifest-path src-tauri/Cargo.toml
```

For native/UI reproduction, use
`$env:CODEX_COMPANION_LOG = 'debug'; pnpm tauri dev`. Record only the command,
error text, Windows version, and whether the run is portable or installed.
Never copy session contents, bundle passwords, token/OAuth values, or credential
files into logs, commits, screenshots, or bug reports.

### Common local setup failures

- Missing Node, pnpm, Cargo, Visual Studio C++ Build Tools, or WebView2: install
  the prerequisite named by the failed command, then rerun the same command.
- `pnpm` cannot open its per-user AppData config: retry from a normal user
  PowerShell after checking for a lock from security software. Do not delete
  the configuration as the first fix.
- Portable build behaves like an installed build: confirm `portable-mode` is
  adjacent to `dev-companion.exe`; keep the full portable folder together.
- A personal bundle is not visible on the Dashboard: use **Làm mới** and verify
  it is a direct file in `backup/` (personal bundle) or `backups/` (session
  backup), not in a nested folder.

## Conversations filter/refresh stalls — 2026-09-09

- Trigger: selecting a Conversations filter or Refresh made the installed app
  appear frozen.
- Frontend cause: every table rerender created two `Intl.DateTimeFormat`
  objects per visible row. A packaged WebView test with 2,000 sessions created
  4,000 formatters and took 624 ms for a sort change.
- Fix: memoize one formatter per language and parse each date once. The focused
  regression renders 2,000 rows in English and Vietnamese, including invalid
  and missing dates.
- Native cause: all Tauri commands were synchronous. Startup diagnostics,
  conversation discovery, archive work and blocking dialogs could occupy the
  window thread.
- Fix: async command wrappers serialize their blocking work in Tauri's blocking
  pool. The commands tests cover worker execution, serialization and recovery
  after a worker panic.

## Checkout rename and generated paths

- The Git remote and product names already point to `dev-companion`.
- pnpm junctions and generated Tauri permission metadata kept absolute paths to
  the former checkout; reinstall dependencies and rebuild the affected cache.
- Keep `codex-companion` storage and archive-kind strings where documented as
  compatibility identifiers.
