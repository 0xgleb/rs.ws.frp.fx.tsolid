use domain::{OrderFull, OrderPatch};
use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};
use sync_core::Diffable;
use uuid::Uuid;

/// Server-side entity store holding the canonical state of all entities.
///
/// Uses `Arc<RwLock<...>>` for thread-safe concurrent access from
/// multiple Socket.IO handler tasks. All operations are atomic at the
/// entity level -- reads get a consistent snapshot, writes compute
/// diffs under the write lock.
///
/// # Example
///
/// ```rust,no_run
/// use sync_server::state::EntityStore;
/// use domain::{OrderFull, OrderPatch};
/// use uuid::Uuid;
///
/// let store = EntityStore::new();
/// let id = Uuid::now_v7();
///
/// let order = OrderFull {
///     symbol: "BTC".into(),
///     side: "buy".into(),
///     status: "open".into(),
///     ..Default::default()
/// };
/// store.set_order(id, order.clone());
/// assert_eq!(store.get_order(&id), Some(order));
///
/// let patch = OrderPatch { status: Some("filled".into()), ..Default::default() };
/// let diff = store.patch_order(&id, patch);
/// assert!(diff.is_some());
/// assert_eq!(store.get_order(&id).unwrap().status, "filled");
/// ```
#[derive(Debug, Default, Clone)]
pub struct EntityStore {
    pub orders: Arc<RwLock<BTreeMap<Uuid, OrderFull>>>,
}

/// A request ID paired with a session ID for deduplication.
///
/// Can be used to prevent duplicate command processing when clients
/// retry on network failures.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RequestKey {
    pub session_id: String,
    pub request_id: String,
}

impl EntityStore {
    /// Create an empty entity store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get a full snapshot of an order by its UUID.
    ///
    /// Returns `None` if the order doesn't exist. The returned value
    /// is a clone, so the caller can use it without holding any lock.
    pub fn get_order(&self, id: &Uuid) -> Option<OrderFull> {
        self.orders.read().unwrap().get(id).cloned()
    }

    /// Insert or replace an order, returning the old value if any.
    ///
    /// Used by the `PlaceOrder` command handler to create new orders.
    pub fn set_order(&self, id: Uuid, order: OrderFull) -> Option<OrderFull> {
        self.orders.write().unwrap().insert(id, order)
    }

    /// Apply a patch to an existing order atomically.
    ///
    /// Computes the actual diff between the old state and the merged
    /// result, then returns only the fields that actually changed.
    /// This ensures clients receive the minimal update, even if the
    /// incoming patch contained redundant fields.
    ///
    /// Returns `None` if:
    /// - The order doesn't exist
    /// - The patch produced no actual changes
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

    /// Get a snapshot of all orders.
    ///
    /// Returns a cloned `BTreeMap`, so iteration is deterministic
    /// (ordered by UUID).
    pub fn all_orders(&self) -> BTreeMap<Uuid, OrderFull> {
        self.orders.read().unwrap().clone()
    }
}
