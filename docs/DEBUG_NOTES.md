# Debug Notes

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
