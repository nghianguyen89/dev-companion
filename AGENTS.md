# Codex Project Instructions

These instructions apply to this repository and are shared by the team.

## Read First

Before substantial work, read the relevant documentation:

1. `README.md`
2. `docs/MODEL_STRATEGY.md`
3. `docs/MULTI_AGENT_STRATEGY.md`
4. `docs/ARCHITECTURE.md`
5. `docs/CURRENT_STATE.md`
6. task-specific documentation when applicable

Do not repeatedly scan unrelated repository areas after the affected area is known.

## Project Profile

```text
Project: Dev Companion (Windows-first local developer-tool utility)
Primary stack: Tauri 2 / Rust / React 19 / TypeScript
Package/build tool: pnpm / Vite / Cargo
Main application entry: src/app/App.tsx; src-tauri/src/lib.rs
Important modules: commands, session_storage, backup, environment, personal_bundle, sourcetree, xampp
CI: .github/workflows/ci.yml (frontend, Rust, Windows NSIS)
Deployment: build-publish.ps1; Windows NSIS installer and portable executable
```

## Implementation Rules

- Follow existing architecture and coding conventions.
- Prefer focused, minimal changes.
- Do not perform unrelated cleanup.
- Do not introduce new frameworks, dependencies, abstractions, or architectural
  patterns unless clearly justified.
- Preserve backward compatibility unless the task explicitly requires a break.
- Reuse existing utilities and patterns before creating new ones.
- Keep public APIs and contracts stable unless the requested change requires otherwise.

## Model Strategy

Follow:

```text
docs/MODEL_STRATEGY.md
```

Default routing:

```text
Luna  -> exploration and routine work
Terra -> normal implementation
Sol   -> planning, reasoning, orchestration and review
Astra -> exceptional complexity / escalation
```

## Model Recommendation Gate

Before substantial work, evaluate complexity, uncertainty, and risk.

If the currently selected main model is materially unsuitable, recommend the
preferred model before substantial implementation:

```text
Recommended model: <Luna | Terra | Sol | Astra>
Reason: <one concise sentence>
```

Do not interrupt trivial work merely to recommend another model.

If the user explicitly asks to continue with the current model, continue.

## Multi-Agent Strategy

Follow:

```text
docs/MULTI_AGENT_STRATEGY.md
```

When delegation is useful and supported, use these role specifications:

```text
docs/agents/EXPLORER.md
docs/agents/ARCHITECT.md
docs/agents/WORKER.md
docs/agents/REVIEWER.md
```

These files describe roles and task boundaries. Do not assume they are
automatically loaded by Codex as custom agent configuration.

Preferred flow for complex work:

```text
Explorer / Luna
      ↓
Architect / Sol
      ↓
Worker / Terra
      ↓
Reviewer / Sol
```

Skip unnecessary stages.

## Validation

```text
Install: pnpm install --frozen-lockfile
Lint: pnpm lint
Type-check: pnpm check
Tests: pnpm test; cargo test --manifest-path src-tauri/Cargo.toml --lib
Build: pnpm build; ./build-publish.ps1
Format: rustfmt --edition 2021 <changed Rust files>; preserve existing frontend style
```

Run validation appropriate to the change.

Never claim a check passed unless it was actually executed.

## Architecture-Sensitive Work

For high-risk or architecture-sensitive changes:

1. analyze first;
2. create/update `docs/IMPLEMENTATION_PLAN.md`;
3. identify affected modules, risks, assumptions, and tests;
4. implement only after the path is sufficiently clear;
5. validate;
6. review significant changes.

Preferred routing:

```text
Sol   -> Plan
Terra -> Implement
Sol   -> Review
```

Escalate to Astra only when genuinely warranted.

## Repository Knowledge

When investigation produces stable knowledge useful to future work, update:

```text
docs/CURRENT_STATE.md
docs/ARCHITECTURE.md
docs/IMPLEMENTATION_PLAN.md
docs/DEBUG_NOTES.md
```

Keep documentation concise and factual.
