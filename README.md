# Dev Companion

Dev Companion is a Windows-only local migration assistant for selected developer-tool data: Codex environment files, Beyond Compare settings packages, SourceTree bookmarks, and XAMPP projects/configuration. It is an independent community utility, not an official OpenAI product.

It does not replace Codex Desktop, Beyond Compare, SourceTree, XAMPP, or alter their binaries. Local data changes are explicit: create-only recovery, selected local session deletion with safety archives, and allowlisted cache cleanup.

## Version 0.2.0

Codex environment archives cover selected local files with SHA-256 verification, offline source locks, and create-only restore. **Desktop chat/database merging remains unfinished and unverified.** Database/index/settings are manual migration components; copying files does not establish a usable migrated Desktop environment.

Temporary cleanup is on-demand and limited to the verified remote plugin catalog cache. Installed plugin resources are protected. Session v1 and existing delete safety archives remain readable.

Personal bundles are concrete Windows-only workflows: an opaque user-exported Beyond Compare `.bcpkg`, fixture-proven non-secret SourceTree `bookmarks.xml`, and explicitly selected XAMPP `htdocs` projects plus reviewed text configuration. Recovery stages files for manual placement or import and never overwrites existing data.

See [User guide](docs/USER_GUIDE.md), [storage inventory and limitations](docs/STORAGE_AUDIT.md), [build instructions](BUILD.md), and [validation](docs/VALIDATION.md).

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
