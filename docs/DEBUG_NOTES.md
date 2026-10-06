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
- A managed archive is not visible on the Dashboard: use **Làm mới** and verify
  it is a direct file in `backups/`, not in a nested folder.

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

## Migration restore regressions — 2026-10-06

- Latest target trace reaches 608/618, copying `.codex/state_5.sqlite`, while
  the newly captured UI still shows 563. Its old leading-edge throttle could
  discard the final update and had no timer to flush during an I/O wait. Native
  progress beyond 563 proves the displayed count was stale. A UI-only observer
  now sends the latest state every 100ms without holding the data worker.
- User measured the destination database at 6,685,187 bytes twice thirty seconds
  apart; the source entry is 14,446,592 bytes. Copying is incomplete/stationary,
  but the old `copy-file` event does not distinguish source reads from writes.
  A bounded 64KiB copy now reports those boundaries and confirmed written bytes.
  Size, ZIP CRC, actual-byte SHA, path/sharing checks and rollback remain required.
  No source/cloud/device/filter cause or target completion is established.

- Latest path-trace attachment ends mid-record:
  `{"notificationCallbackReturned":true,"progress":{"completed":594,`.
  The user confirms this is the original log's EOF, not a truncated copy.
  It follows the full false `verify-read` record for `validate_atlas.py`.
  This proves callback return and entry into the after-callback log write;
  the actual destination read has not resumed. Screen counter 588 is a sampled
  older UI notification; native log count is 594. No driver/filter attributed.
- Root defect in this diagnostic path: `serde_json::to_writer(File)` performs
  many tiny synchronous writes on the restore thread. Ignoring errors does
  not prevent a stalled write from blocking restore. Encode each JSON/newline
  record to bytes, then `try_send` through a bounded queue to a log-only worker.
  Never join that worker from production data work. Preserve all integrity and
  rollback; full/disconnected logging drops/counts records instead of waiting.
- Producer timestamps describe native notification boundaries. Queued logs can
  lag/drop/stall independently; their incomplete tail is no longer proof of a
  native restore stop. `droppedRecords` counts gaps before an accepted record.
  A stuck writer can remain detached until app exit; it never owns data targets
  or safety archives. Thread/log creation remains before destination mutation.
- A test deliberately blocks the writer and fills a one-record queue, then
  verifies actual isolated restore + all SHA complete before releasing it.
  Tests release/join their logger for cleanup; production never waits for it.

- Earlier handle-fix target run verifies/drops entry 595 successfully and records
  completed=595, then stops at `prepare-target` for
  `validate_direction_blind_verdicts.py`. User PowerShell `Get-Item` reads the
  parent `scripts` directory; supplied output omitted TotalSeconds.
- Correction: earlier traces were written before the progress callback, so
  `flush-file`/`verify-file` alone did not prove those filesystem calls had
  begun or blocked. The exact native blocked call remains unconfirmed.
- New trace pairs retain every event with `notificationCallbackReturned=false`
  before callback entry and `true` after it returns. Target preparation marks
  check/mkdir/recheck and each ancestor immediately before metadata inspection
  (`check-path`, absolute path in `file`). Ancestor records stay local; IPC
  forwards stage changes and at most ten same-stage notifications per second.
  `true` proves callback return, not UI delivery. Missing `true` cannot prove
  a callback block if writing the second trace itself fails/stalls.
- That diagnostic version kept trace writes best-effort and synchronous;
  and all original path/reparse/integrity/rollback checks remain. Tauri's
  installed small-payload send uses direct eval with no JS acknowledgement;
  no specific IPC deadlock mechanism or device/filter cause was established.

- Earlier target log and screenshot reach `close-file`, then stop at
  `verify-file` on the same entry 595. The Dropbox flush-fix EXE hash matches
  the build. Removing forced sync exposed a later stalled verification path;
  it did not resolve target completion. The exact reopen/path-check/read
  subcall and underlying device/filter remain unknown.
- Both restore and safety rollback now create read/write outputs and verify
  length, seek/read of every expected byte and SHA using that same handle,
  before closing. Windows sharing denies competing writes/deletes. Traces
  mark `verify-size`, `verify-seek`, `verify-read`, `verify-digest`, then close.
  Tests exercise mismatch rejection, output sharing and full rollback after
  an injected digest mismatch. Source/safety locks and strict validation remain.
  No file-specific exception, detached I/O timer or skipped integrity check.
  See [Windows file sharing](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew).

- Earlier follow-up: target logs and UI stop at `flush-file` for file 595,
  directly before `output.sync_all()`, with no close/verify event. Copy completed
  in milliseconds and the user independently read its expected hash in 0.134s.
  At that time it was attributed to disk sync; the callback-bracketing gap
  above means the exact blocked call was not established.
- Remove destination per-file forced disk sync from both normal restore and
  safety rollback. Retain close/read/hash, both ZIP syncs and strict safety
  validation before removal. Normal Windows writeback means a completed
  destination SHA check is not a power-loss durability guarantee. Keep source
  and safety ZIPs for recovery. `sync_data`/a final sync can hit the same call;
  a detached timeout would leave a live writer and break safe rollback.
- References: [Rust file synchronization](https://doc.rust-lang.org/std/fs/struct.File.html#method.sync_all)
  and [Microsoft FlushFileBuffers](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-flushfilebuffers).

- Follow-up screenshot: restoration stopped at 594/618 on the new machine.
  Entry 595 is `skills/hatch-pet/scripts/validate_atlas.py`, 10,983 bytes.
  The user confirms that full-size target exists and entry 615 is absent.
  The same ZIP member extracted, flushed and hash-verified locally in 131 ms.
  File existence/full size does not prove sync/close/SHA verification returned.
  A user-run PowerShell SHA check on the target returned the expected hash
  `523528e77f58cbc4b71500249988092ba750d54657f37fe21c0a0a00a248f50a`
  in 0.13405 seconds. This proves current target bytes match and that this read
  was fast; it does not establish which native call or UI delivery is blocked.
- Per-file diagnostics now mark target preparation/open, ZIP entry open,
  copy, disk sync, close and output SHA verification before each operation.
  `read-archive` marks entry opening; `copy-file` includes reads and writes.
  Best-effort metadata-only JSONL records in Companion's local AppData `logs/`
  distinguish blocked native phases from missed UI/channel messages. Trace
  creation happens before mutations; tracing cannot abort rollback.
  No antivirus/cloud cause is established, and no detached I/O timeout or
  weakened integrity check was introduced.

- Follow-up: the target-machine operation appeared busy for about 30 minutes
  without progress. The synced safety ZIP was complete (619 members protecting
  1,097,164,735 bytes); this does not establish the remote operation's final
  phase. The development machine has no matching live process to inspect.
- Avoid repeated decompression of an unchanged, already validated input ZIP;
  reuse its validated manifest only after verifying the streaming ZIP digest
  under a held read lock. Avoid walking unrelated destination cache/plugin
  trees and recompressing the destination safety copy. Safety validation,
  destination hashes and rollback remain required.
- Show restore phase/counts immediately below the apply button, before the
  bounded file table. Skip stale runtime files before collecting safety entries,
  including `sessions/*.tmp`; a trailing slash defeats filename exclusions.

- Freshly installed Codex still creates existing project/global state files.
  Create-only `RESTORE` silently kept those conflicts. The migration page now
  offers only safety-backed overwrite with `REPLACE CODEX`; project registry,
  project metadata and databases are restored together.
- The user's actual 367 MiB migration ZIP contained 618 files including
  `.codex-global-state.json` and project metadata. An isolated restore with
  an existing empty registry verified all 618 output SHA-256 hashes. This is
  file-level evidence, not a new-machine Desktop sidebar verification.
- SourceTree source-profile metadata and timestamps of unrelated configuration
  files cannot establish the destination's primary profile. Restore primary
  files into both standard Local/Roaming roots with unique safety copies;
  select installed `user.config` independently. Source discovery prefers Local
  per primary file with Roaming fallback when absent.
- `.migration-archives li > div:last-child` selected the appended recovery
  panel and turned it into a flex row. Target action `div:nth-child(2)` and
  direct archive list children so nested recovery lists stay normal.
- Full library checks: 91 passed, 1 ignored real-archive opt-in test (run
  separately and passed), 1 existing environment-manager test failed because
  the current user home was unavailable. All Codex migration and SourceTree
  configuration tests passed in that run. Frontend: check/lint/build and
  32 tests passed. Browser fixtures passed at 847px and 600px.
