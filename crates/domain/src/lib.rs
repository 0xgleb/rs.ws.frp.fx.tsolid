//! # domain
//!
//! Domain entities for the sync protocol proof of concept.
//!
//! This crate defines the business objects that are synchronized between
//! server and client. Currently models a simple **trading order** system
//! with two entities:
//!
//! - [`Order`] -- A trading order with symbol, side, status, quantities,
//!   a nested map of fills, and a leaf map of notes.
//! - [`OrderLine`] -- A single fill/execution with quantity and price.
//!
//! Both use `#[derive(SyncEntity)]` which generates `OrderFull`,
//! `OrderPatch`, `OrderLineFull`, `OrderLinePatch`, along with `Diffable`
//! and `FullToPatch` implementations.
//!
//! ## Example: creating and diffing orders
//!
//! ```rust
//! use domain::{OrderFull, OrderLineFull, OrderPatch};
//! use rust_decimal::Decimal;
//! use std::collections::BTreeMap;
//! use sync_core::Diffable;
//!
//! // Create an order
//! let order = OrderFull {
//!     symbol: "BTC".into(),
//!     side: "buy".into(),
//!     status: "open".into(),
//!     total_quantity: Decimal::new(1000, 0),
//!     filled_quantity: Decimal::ZERO,
//!     average_price: Decimal::ZERO,
//!     fills: BTreeMap::new(),
//!     notes: BTreeMap::from([("origin".into(), "manual".into())]),
//! };
//!
//! // Modify the order
//! let mut updated = order.clone();
//! updated.status = "partially_filled".into();
//! updated.filled_quantity = Decimal::new(500, 0);
//!
//! // Compute the diff -- only changed fields are present
//! let patch = order.diff(&updated).expect("should have changes");
//! assert_eq!(patch.status, Some("partially_filled".into()));
//! assert_eq!(patch.filled_quantity, Some(Decimal::new(500, 0)));
//! assert_eq!(patch.symbol, None); // unchanged, omitted
//!
//! // Apply the patch to get back to the updated state
//! let result = order.merge(patch);
//! assert_eq!(result, updated);
//! ```
//!
//! ## Example: nested map diffing (fills)
//!
//! ```rust
//! use domain::{OrderFull, OrderLineFull};
//! use rust_decimal::Decimal;
//! use std::collections::BTreeMap;
//! use sync_core::Diffable;
//!
//! let mut order = OrderFull {
//!     symbol: "ETH".into(),
//!     side: "sell".into(),
//!     status: "open".into(),
//!     total_quantity: Decimal::new(100, 0),
//!     filled_quantity: Decimal::ZERO,
//!     average_price: Decimal::ZERO,
//!     fills: BTreeMap::from([
//!         ("fill-1".into(), OrderLineFull {
//!             quantity: Decimal::new(50, 0),
//!             price: Decimal::new(3000, 0),
//!         }),
//!     ]),
//!     notes: BTreeMap::new(),
//! };
//!
//! // Remove a fill (tombstone)
//! let mut updated = order.clone();
//! updated.fills.remove("fill-1");
//!
//! let patch = order.diff(&updated).unwrap();
//! assert!(matches!(patch.fills.get("fill-1"), Some(None))); // tombstone
//!
//! let result = order.merge(patch);
//! assert!(result.fills.is_empty());
//! ```
//!
//! ## Example: leaf map diffing (notes)
//!
//! ```rust
//! use domain::OrderFull;
//! use std::collections::BTreeMap;
//! use sync_core::Diffable;
//!
//! let order = OrderFull {
//!     notes: BTreeMap::from([("k1".into(), "v1".into())]),
//!     ..Default::default()
//! };
//!
//! let mut updated = order.clone();
//! updated.notes.insert("k2".into(), "v2".into());
//! updated.notes.remove("k1");
//!
//! let patch = order.diff(&updated).unwrap();
//! assert_eq!(patch.notes.get("k1"), Some(&None));          // tombstone
//! assert_eq!(patch.notes.get("k2"), Some(&Some("v2".into()))); // added
//! ```

use rust_decimal::Decimal;
use std::collections::BTreeMap;
use sync_macro::SyncEntity;

/// Force the linker to include this crate's `inventory` registrations.
///
/// Must be called before [`sync_core::ts::generate_ts_file()`] to ensure
/// entity TypeScript interfaces are registered. Without this call, the
/// linker may optimize away the `inventory::submit!` blocks and codegen
/// will produce empty output.
///
/// # Example
///
/// ```rust
/// domain::register_entities();
/// // Now sync_core::ts::generate_ts_file() will include Order and OrderLine
/// ```
pub fn register_entities() {
    // This function exists solely to ensure the domain crate is linked
    // and its inventory::submit! registrations are included.
}

/// Marker type for `Id<OrderTag>` phantom parameter.
///
/// Used with [`sync_core::Id`] to create type-safe order identifiers
/// that cannot be accidentally mixed with other entity IDs.
///
/// ```rust
/// use sync_core::Id;
/// use domain::OrderTag;
///
/// let order_id: Id<OrderTag> = Id::new_v7();
/// ```
pub struct OrderTag;

/// Marker type for `Id<FillTag>` phantom parameter.
///
/// Used with [`sync_core::Id`] to create type-safe fill identifiers.
///
/// ```rust
/// use sync_core::Id;
/// use domain::FillTag;
///
/// let fill_id: Id<FillTag> = Id::new_v7();
/// ```
pub struct FillTag;

/// A single fill (execution) in a trading order.
///
/// Generates `OrderLineFull` and `OrderLinePatch` via `#[derive(SyncEntity)]`.
/// Used as nested values in `Order.fills` to demonstrate recursive diffing.
///
/// ## Generated types
///
/// - `OrderLineFull { quantity: Decimal, price: Decimal }`
/// - `OrderLinePatch { quantity: Option<Decimal>, price: Option<Decimal> }`
#[derive(SyncEntity)]
pub struct OrderLine {
    /// Quantity filled in this execution.
    pub quantity: Decimal,
    /// Price per unit for this execution.
    pub price: Decimal,
}

/// A trading order with nested fills and leaf notes.
///
/// Generates `OrderFull` and `OrderPatch` via `#[derive(SyncEntity)]`.
/// Demonstrates all three field categories:
///
/// - **Scalar fields** (`symbol`, `side`, `status`, quantities, price)
/// - **Nested map** (`fills`) -- values are `OrderLineFull` which are
///   themselves `Diffable`, enabling recursive patching
/// - **Leaf map** (`notes`) -- values are plain strings, replaced wholesale
///
/// ## Generated types
///
/// - `OrderFull` -- All fields required, derives `PartialEq` for testing
/// - `OrderPatch` -- All fields optional, maps use tombstone semantics
#[derive(SyncEntity)]
pub struct Order {
    /// Trading symbol (e.g., "BTC", "ETH").
    pub symbol: String,
    /// Order side: "buy" or "sell".
    pub side: String,
    /// Order status: "open", "partially_filled", "filled", "closed".
    pub status: String,
    /// Total quantity requested.
    pub total_quantity: Decimal,
    /// Quantity filled so far.
    pub filled_quantity: Decimal,
    /// Volume-weighted average fill price.
    pub average_price: Decimal,
    /// Individual fill executions, keyed by fill ID.
    /// Marked `#[sync(nested)]` for recursive diffing.
    #[sync(nested)]
    pub fills: BTreeMap<String, OrderLineFull>,
    /// Arbitrary key-value notes attached to the order.
    /// Plain leaf map -- values are replaced wholesale, not diffed.
    pub notes: BTreeMap<String, String>,
}
