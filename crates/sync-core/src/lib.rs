//! # sync-core
//!
//! Core library for the entity synchronization framework. Provides the
//! foundational types and traits used by all other crates:
//!
//! - [`Id<E>`] -- Type-safe entity ID wrapping UUID v7 with a phantom type
//!   parameter to prevent mixing IDs from different entity types.
//! - [`Diffable`] -- Trait for computing minimal diffs between entity states
//!   and merging patches back, following RFC 7396 JSON Merge Patch semantics.
//! - [`OutboundMessage`] / [`InboundMessage`] -- Wire-format message envelopes
//!   for the Socket.IO sync protocol.
//! - [`TsTypeRegistration`] -- Registry for auto-generating TypeScript
//!   interfaces from Rust entity definitions.
//!
//! ## Example: diffing and merging
//!
//! ```rust
//! use sync_core::diffable::{diff_leaf, merge_leaf, diff_leaf_map, merge_leaf_map};
//! use std::collections::BTreeMap;
//!
//! // Leaf scalar diff
//! assert_eq!(diff_leaf(&42, &42), None);        // no change
//! assert_eq!(diff_leaf(&42, &99), Some(99));     // changed
//!
//! // Leaf scalar merge
//! assert_eq!(merge_leaf(42, None), 42);          // keep original
//! assert_eq!(merge_leaf(42, Some(99)), 99);      // apply patch
//!
//! // Map diff with tombstones
//! let old: BTreeMap<String, i32> = [("a".into(), 1), ("b".into(), 2)].into();
//! let new: BTreeMap<String, i32> = [("a".into(), 1), ("c".into(), 3)].into();
//! let patch = diff_leaf_map(&old, &new);
//! assert_eq!(patch.get("a"), None);              // unchanged, omitted
//! assert_eq!(patch.get("b"), Some(&None));        // tombstone (deleted)
//! assert_eq!(patch.get("c"), Some(&Some(3)));     // added
//! ```

pub mod id;
pub mod diffable;
pub mod message;
pub mod ts;

pub use id::Id;
pub use diffable::Diffable;
pub use message::{InboundMessage, OutboundMessage, MessageKind};
pub use ts::TsTypeRegistration;
