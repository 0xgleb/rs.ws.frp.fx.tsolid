//! # sync-server
//!
//! Real-time entity synchronization server built on Axum and Socket.IO.
//!
//! Provides the server-side implementation of the sync protocol:
//!
//! - **[`state::EntityStore`]** -- Thread-safe in-memory store for canonical
//!   entity state, with atomic diff-and-update operations.
//! - **[`handler`]** -- Socket.IO event handlers for handshake and command
//!   processing, with room-based broadcasting.
//! - **[`server::build_app`]** -- Assembles the Axum router with Socket.IO
//!   layer, CORS, and health check endpoint.
//!
//! ## Quick start
//!
//! ```rust,no_run
//! use sync_server::build_app;
//! use sync_server::state::EntityStore;
//!
//! #[tokio::main]
//! async fn main() {
//!     let store = EntityStore::new();
//!     let (app, _io) = build_app(store);
//!
//!     let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
//!     axum::serve(listener, app).await.unwrap();
//! }
//! ```
//!
//! ## Protocol flow
//!
//! 1. Client connects via Socket.IO WebSocket and joins the `"orders"` room.
//! 2. Client emits `"handshake"` events for each entity it holds. Server
//!    responds with `"sync"` events containing Full or Patch payloads.
//! 3. Client emits `"command"` events (e.g., `PlaceOrder`, `UpdateOrder`).
//!    Server applies the change, computes the diff, broadcasts the Patch
//!    to all clients in the room, and sends an `"ack"` back to the sender.

pub mod state;
pub mod handler;
pub mod server;

pub use server::build_app;
