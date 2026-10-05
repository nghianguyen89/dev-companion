# Dev Companion

Dev Companion is a Windows-only local developer-tool utility for selected migration data and Codex environment management: Codex environment files, isolated local Codex environments, Beyond Compare settings packages, SourceTree configuration/bookmarks, and XAMPP projects/configuration. It is an independent community utility, not an official OpenAI product.

File Transfer is a local Robocopy GUI for explicit folder transfers. It supports Simple Copy, Fast Copy, Project Migration, and confirmation-protected Mirror operations with direct process launch, live output, logs, profiles, and local history.

It does not replace Codex Desktop, Beyond Compare, SourceTree, XAMPP, or alter their binaries. Local data changes are explicit: create-only recovery, selected local session deletion with safety archives, and allowlisted cache cleanup.

## Version 0.2.0

Codex environment archives cover selected local files with SHA-256 verification, offline source locks, and create-only restore. **Desktop chat/database merging remains unfinished and unverified.** Database/index/settings are manual migration components; copying files does not establish a usable migrated Desktop environment.

Temporary cleanup is on-demand and limited to the verified remote plugin catalog cache. Installed plugin resources are protected. Session v1 and existing delete safety archives remain readable.

Personal bundles are concrete Windows-only workflows: an opaque user-exported Beyond Compare `.bcpkg`, SourceTree bookmarks and an optional AES-256 encrypted full local-configuration bundle, and explicitly selected XAMPP `htdocs` projects plus reviewed text configuration. Beyond Compare, bookmark and XAMPP recovery stage files for manual placement or import; the separately confirmed SourceTree configuration restore replaces its allowlisted local files only after a safety copy and hash verification.

See [User guide](docs/USER_GUIDE.md), [storage inventory and limitations](docs/STORAGE_AUDIT.md), [build instructions](BUILD.md), [development and debugging](docs/DEVELOPMENT.md), [debug notes](docs/DEBUG_NOTES.md), and [validation](docs/VALIDATION.md).

## Quick start

For a built portable copy, keep the full `release/portable` folder together and
run `dev-companion.exe`. Keep `portable-mode` beside the executable. Codex
migration ZIPs and personal application bundles share its adjacent `backups/`
folder, so they travel with the portable folder. The Dashboard is the first
navigation item and shows one read-only inventory of that folder.

To develop locally on Windows, install Node.js, pnpm, Rust stable, Visual
Studio C++ Build Tools, and WebView2, then run:

```powershell
pnpm install
pnpm tauri dev
```

For a portable executable and NSIS installer, close any running portable copy
and run `./build-publish.ps1`. Full commands and troubleshooting are in
[BUILD.md](BUILD.md) and [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md).

## Supported platforms

The current app-adapter workflows are Windows-only. Native bundles must be produced on their matching operating systems through CI.

## Development

Prerequisites: Node.js, pnpm, Rust stable, and the platform prerequisites required by Tauri 2.

```sh
pnpm install
pnpm dev
pnpm lint
pnpm check
pnpm test
pnpm build
pnpm tauri dev
```

See [Development notes](docs/DEVELOPMENT.md), [architecture](docs/ARCHITECTURE.md), and the [backup format](docs/BACKUP_FORMAT.md).

Companion can delete only supported legacy Codex session files discovered under `CODEX_HOME/sessions`; it never calls a cloud API and never deletes Codex Desktop or cloud chats. The UI requires a metadata-only preview and typing `DELETE`. Rust re-resolves IDs, rejects symlinks/path traversal, writes and validates a create-new versioned ZIP safety archive before removal, and restores with create-new semantics if removal fails. Safety archives are retained in Companion's local quarantine until manually removed.
