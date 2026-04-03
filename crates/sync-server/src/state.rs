use domain::{OrderFull, OrderPatch};
use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};
use sync_core::Diffable;
use uuid::Uuid;

/// Server-side entity store. Holds the canonical state of all entities.
#[derive(Debug, Default, Clone)]
pub struct EntityStore {
    pub orders: Arc<RwLock<BTreeMap<Uuid, OrderFull>>>,
}

/// A request ID paired with a session ID for deduplication.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RequestKey {
    pub session_id: String,
    pub request_id: String,
}

impl EntityStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get a full snapshot of an order.
    pub fn get_order(&self, id: &Uuid) -> Option<OrderFull> {
        self.orders.read().unwrap().get(id).cloned()
    }

    /// Insert or replace an order, returning the old value if any.
    pub fn set_order(&self, id: Uuid, order: OrderFull) -> Option<OrderFull> {
        self.orders.write().unwrap().insert(id, order)
    }

    /// Apply a patch to an existing order. Returns the computed patch that was
    /// actually applied (the diff between old and new state), or None if no changes.
    pub fn patch_order(&self, id: &Uuid, patch: OrderPatch) -> Option<OrderPatch> {
        let mut orders = self.orders.write().unwrap();
        if let Some(current) = orders.remove(id) {
            let updated = current.clone().merge(patch);
            let diff = current.diff(&updated);
            orders.insert(*id, updated);
            diff
        } else {
            None
        }
    }

    /// Get all orders.
    pub fn all_orders(&self) -> BTreeMap<Uuid, OrderFull> {
        self.orders.read().unwrap().clone()
    }
}
