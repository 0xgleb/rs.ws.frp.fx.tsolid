use rust_decimal::Decimal;
use std::collections::BTreeMap;
use sync_macro::SyncEntity;

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
