use domain::{OrderFull, OrderLineFull, OrderPatch};
use rust_decimal::Decimal;
use std::collections::BTreeMap;
use sync_core::Diffable;

fn sample_order() -> OrderFull {
    let mut fills = BTreeMap::new();
    fills.insert(
        "fill-1".into(),
        OrderLineFull {
            quantity: Decimal::new(100, 0),
            price: Decimal::new(5050, 2),
        },
    );

    OrderFull {
        symbol: "BTC".into(),
        side: "buy".into(),
        status: "open".into(),
        total_quantity: Decimal::new(1000, 0),
        filled_quantity: Decimal::new(100, 0),
        average_price: Decimal::new(5050, 2),
        fills,
        notes: BTreeMap::from([("note1".into(), "initial".into())]),
    }
}

#[test]
fn diff_identical_is_none() {
    let a = sample_order();
    let b = a.clone();
    assert!(a.diff(&b).is_none());
}

#[test]
fn diff_changed_field() {
    let a = sample_order();
    let mut b = a.clone();
    b.status = "filled".into();
    b.filled_quantity = Decimal::new(1000, 0);

    let patch = a.diff(&b).expect("should have changes");
    assert_eq!(patch.status, Some("filled".into()));
    assert_eq!(patch.filled_quantity, Some(Decimal::new(1000, 0)));
    // Unchanged fields should be None/empty
    assert_eq!(patch.symbol, None);
    assert_eq!(patch.side, None);
    assert!(patch.fills.is_empty());
    assert!(patch.notes.is_empty());
}

#[test]
fn merge_empty_patch_is_identity() {
    let original = sample_order();
    let empty_patch = OrderPatch::default();
    let result = original.clone().merge(empty_patch);
    assert_eq!(original, result);
}

#[test]
fn diff_merge_roundtrip() {
    let original = sample_order();
    let mut updated = original.clone();
    updated.status = "partially_filled".into();
    updated.filled_quantity = Decimal::new(500, 0);
    updated.notes.insert("note2".into(), "update".into());

    let patch = original.diff(&updated).unwrap();
    let result = original.merge(patch);
    assert_eq!(result, updated);
}

#[test]
fn leaf_map_tombstone() {
    let original = sample_order();
    let mut updated = original.clone();
    updated.notes.remove("note1");

    let patch = original.diff(&updated).unwrap();
    assert_eq!(patch.notes.get("note1"), Some(&None)); // tombstone
    let result = original.merge(patch);
    assert_eq!(result, updated);
}

/// RFC 7396 cross-check: verify that our typed diff/merge matches json-patch's merge behavior.
#[test]
fn rfc7396_cross_check() {
    let original = sample_order();
    let mut updated = original.clone();
    updated.status = "closed".into();
    updated.filled_quantity = Decimal::new(1000, 0);
    updated.notes.remove("note1");
    updated.notes.insert("note2".into(), "final".into());

    // Typed diff/merge
    let patch = original.diff(&updated).unwrap();
    let typed_result = original.clone().merge(patch.clone());
    assert_eq!(typed_result, updated);

    // JSON-based merge using json-patch crate
    let mut json_original = serde_json::to_value(&original).unwrap();
    let json_patch = serde_json::to_value(&patch).unwrap();
    json_patch::merge(&mut json_original, &json_patch);

    let json_result: OrderFull = serde_json::from_value(json_original).unwrap();
    assert_eq!(json_result, updated);
}
