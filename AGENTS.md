# AGENTS.md

Instructions for AI coding agents working on this repository.

## Project overview

This is a **real-time entity synchronization framework** for Rust + TypeScript.
It synchronizes state between an Axum/Socket.IO server and SolidJS clients using
RFC 7396 JSON Merge Patch semantics. The primary demo domain is financial trading
orders.

## Repository layout

```
crates/
  sync-core/      Core traits (Diffable, Id, messages) and TS codegen
  sync-macro/     #[derive(SyncEntity)] proc macro
  domain/         Domain entities (Order, OrderLine)
  sync-server/    Axum + Socket.IO server with state and handlers
packages/
  client/         SolidJS + Vite frontend with Effect schema validation
```

## Development commands

### Rust

```bash
cargo check --workspace        # Type check
cargo test --workspace         # Run all Rust tests (unit + integration)
cargo run --bin codegen        # Generate packages/client/generated/sync.ts
cargo run --bin server         # Start the sync server on :3000
```

### TypeScript (from `packages/client/`)

```bash
npm ci                         # Install dependencies
npx tsc --noEmit               # Type check
npx vite build                 # Production build
npx vite dev                   # Dev server (proxies /socket.io to :3000)
npx playwright test            # E2E tests (needs server + codegen first)
```

### Full local workflow

```bash
cargo test --workspace
cargo run --bin codegen
cd packages/client && npm ci
npx tsc --noEmit
cd ../..
cargo run --bin server &
cd packages/client && npx playwright test
```

## Architecture decisions

1. **Typed IDs** -- `Id<E>` uses a phantom type parameter to prevent mixing
   entity IDs at compile time. IDs use UUID v7 for natural ordering.

2. **Diffable trait** -- Core abstraction: `diff(&old, &new) -> Option<Patch>`
   and `merge(full, patch) -> Full`. Leaf fields use `Option<T>`, maps use
   `BTreeMap<K, Option<V>>` with `None` as tombstone for deletions.

3. **SyncEntity derive macro** -- Single `#[derive(SyncEntity)]` generates:
   - `{Name}Full` struct (all fields public, Serialize/Deserialize)
   - `{Name}Patch` struct (all fields optional, skips empty on serialize)
   - `Diffable` impl with field-by-field diff/merge
   - `FullToPatch` impl for converting full state to patch representation
   - TypeScript interfaces registered via `inventory`

4. **Nested vs leaf maps** -- Fields annotated `#[sync(nested)]` use
   `diff_nested_map` / `merge_nested_map` (values are themselves Diffable).
   Unannotated maps use `diff_leaf_map` / `merge_leaf_map` (values are plain
   scalars).

5. **Handshake protocol** -- On connect, client sends its current state per
   entity. Server responds with either a Full (if client state is empty) or a
   Patch (diff from client state to server state). This minimizes bandwidth on
   reconnects.

6. **TypeScript codegen** -- Rust entities produce TS interfaces at build time
   via `inventory` + `generate_ts_file()`. The generated `sync.ts` is committed
   as a CI artifact, not checked into git. Client uses Effect `Schema` for
   runtime validation of incoming messages.

## CI

GitHub Actions with three required status checks: `rs`, `ts`, `e2e`.
- `rs`: cargo check + test + codegen (uploads `sync.ts` artifact)
- `ts`: tsc + vite build (downloads artifact from `rs`)
- `e2e`: builds server + runs Playwright tests

Job names are required status checks in branch protection. Do not rename them
without updating GitHub branch protection settings.

## Conventions

- Rust: snake_case. TypeScript: camelCase. The macro handles conversion.
- `Decimal` fields serialize as strings in JSON (via `serde-str` feature).
- All map types use `BTreeMap` for deterministic ordering.
- Tests should be runnable via `cargo test` (Rust) or `npx playwright test` (E2E).
- Commit regularly and push to the working branch.
- Always run `cargo test --workspace` before pushing Rust changes.
- Always run `npx tsc --noEmit` before pushing TypeScript changes.

## Testing

- **Unit tests**: Inline `#[cfg(test)]` modules in each Rust source file.
- **Integration tests**: `crates/domain/tests/diff_merge.rs` covers diff/merge
  roundtrips, tombstones, and RFC 7396 cross-checks against `json-patch`.
- **E2E tests**: `packages/client/e2e/sync.spec.ts` with Playwright, covering
  connection, order placement, patching, and multi-order grid.
- **Doc tests**: Rust doc comments include runnable examples tested via
  `cargo test --doc`.

## Common pitfalls

- The `domain` crate's `register_entities()` must be called to force the linker
  to include `inventory` registrations. Without it, codegen produces empty output.
- The generated `sync.ts` lives in `packages/client/generated/` which is
  gitignored. Run `cargo run --bin codegen` to regenerate after changing entities.
- Socket.IO proxy in `vite.config.ts` points to `:3000` -- the Rust server must
  be running for the dev client to connect.
