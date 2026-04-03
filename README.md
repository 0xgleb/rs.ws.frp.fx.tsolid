# Sync Protocol PoC

A real-time entity synchronization framework for **Rust** and **TypeScript**.
Maintains consistent state between an Axum/Socket.IO server and SolidJS clients
using [RFC 7396 JSON Merge Patch](https://datatracker.ietf.org/doc/html/rfc7396)
semantics with typed diffs, tombstone deletions, and automatic TypeScript codegen.

## How it works

```
┌────────────────────┐       Socket.IO       ┌────────────────────┐
│   SolidJS Client   │◄────────────────────►│   Axum Server      │
│                    │   Full / Patch msgs   │                    │
│  store.ts          │                       │  EntityStore       │
│  socket.ts         │                       │  handler.rs        │
│  generated/sync.ts │                       │  state.rs          │
└────────────────────┘                       └────────────────────┘
         ▲                                            ▲
         │                                            │
    Effect Schema                              #[derive(SyncEntity)]
    validation                                 macro generates:
                                                 - Full struct
                                                 - Patch struct
                                                 - Diffable impl
                                                 - TS interfaces
```

### Sync protocol

1. **Connect** -- Client opens a WebSocket via Socket.IO.
2. **Handshake** -- Client sends its current state for each known entity. Server
   responds with a `Full` snapshot (if client has nothing) or a `Patch` (minimal
   diff to bring client up to date).
3. **Commands** -- Client sends `PlaceOrder`, `UpdateOrder`, etc. Server applies
   the change, computes the diff, and broadcasts a `Patch` to all clients in the
   `orders` room.
4. **Reconnect** -- On reconnect, the handshake replays, so clients only receive
   the delta they missed.

### Diff / merge model

```
old state ──┐
            ├── diff() ──► Option<Patch>
new state ──┘

full state ──┐
             ├── merge() ──► updated Full
patch ───────┘
```

- **Leaf fields**: `diff` returns `Some(new_value)` if changed, `None` if same.
  `merge` picks the patch value when present.
- **Leaf maps** (`BTreeMap<K, V>`): Entries present in old but not new become
  tombstones (`None`). New/changed entries become `Some(value)`.
- **Nested maps** (`#[sync(nested)]`): Values are themselves `Diffable`, so
  patches are recursive -- only changed sub-fields travel over the wire.

## Quick start

### Prerequisites

- Rust stable (1.75+)
- Node.js 22+
- npm

### Run everything

```bash
# Build and test Rust
cargo test --workspace

# Generate TypeScript types from Rust entities
cargo run --bin codegen

# Start the server
cargo run --bin server &

# Install client dependencies and start dev server
cd packages/client
npm ci
npx vite dev
# Open http://localhost:5173
```

### Run E2E tests

```bash
cargo build --bin server
cd packages/client
npm ci
npx playwright install --with-deps chromium
npx playwright test
```

## Project structure

```
crates/
  sync-core/        Core library: Id<E>, Diffable trait, message types, TS codegen
  sync-macro/       Proc macro: #[derive(SyncEntity)]
  domain/           Domain models: Order, OrderLine (the PoC entities)
  sync-server/      Server: Axum + Socket.IO, EntityStore, command handlers
packages/
  client/           SolidJS frontend with Effect schema validation
    src/
      socket.ts     WebSocket connection + message decoding
      store.ts      Reactive store with merge-patch logic
      App.tsx       Order grid UI
    e2e/
      sync.spec.ts  Playwright E2E tests
    generated/
      sync.ts       Auto-generated TS types (gitignored)
```

## Defining entities

Use the `SyncEntity` derive macro to define a syncable entity:

```rust
use sync_macro::SyncEntity;
use rust_decimal::Decimal;
use std::collections::BTreeMap;

#[derive(SyncEntity)]
pub struct Order {
    pub symbol: String,
    pub side: String,
    pub status: String,
    pub total_quantity: Decimal,
    pub filled_quantity: Decimal,
    pub average_price: Decimal,
    #[sync(nested)]
    pub fills: BTreeMap<String, OrderLineFull>,
    pub notes: BTreeMap<String, String>,
}

#[derive(SyncEntity)]
pub struct OrderLine {
    pub quantity: Decimal,
    pub price: Decimal,
}
```

This generates `OrderFull`, `OrderPatch`, `OrderLineFull`, `OrderLinePatch`,
along with `Diffable` and `FullToPatch` implementations, and registers
TypeScript interface definitions for codegen.

## CI

GitHub Actions pipeline with three required status checks:

| Job   | What it does                                       |
|-------|----------------------------------------------------|
| `rs`  | `cargo check` + `cargo test` + codegen artifact    |
| `ts`  | `tsc --noEmit` + `vite build`                      |
| `e2e` | Builds server + Playwright tests                   |

## License

MIT -- see [LICENSE](LICENSE).
