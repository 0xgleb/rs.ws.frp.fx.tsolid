use rust_decimal::Decimal;
use std::collections::BTreeMap;
use sync_macro::SyncEntity;

/// Force the linker to include this crate's inventory registrations.
pub fn register_entities() {
    // This function exists solely to ensure the domain crate is linked
    // and its inventory::submit! registrations are included.
}

/// Marker types for Id phantom parameter
pub struct OrderTag;
pub struct FillTag;

#[derive(SyncEntity)]
pub struct OrderLine {
    pub quantity: Decimal,
    pub price: Decimal,
}

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
