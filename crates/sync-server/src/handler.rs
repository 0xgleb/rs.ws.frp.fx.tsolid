use crate::state::EntityStore;
use domain::{OrderFull, OrderPatch};
use serde_json::Value;
use socketioxide::extract::{Data, SocketRef, State};
use sync_core::message::{MessageKind, OutboundMessage};
use sync_core::Diffable;
use uuid::Uuid;

/// Handle the sync handshake: client sends what it has, server responds with diffs.
pub async fn on_handshake(
    socket: SocketRef,
    Data(data): Data<Value>,
    State(store): State<EntityStore>,
) {
    let entity_tag = data.get("entityTag").and_then(|v| v.as_str()).unwrap_or("");
    let entity_id_str = data.get("entityId").and_then(|v| v.as_str()).unwrap_or("");
    let payload = data.get("payload").cloned().unwrap_or(Value::Object(Default::default()));

    match entity_tag {
        "Order" => {
            let Ok(entity_id) = Uuid::parse_str(entity_id_str) else {
                return;
            };

            if let Some(current) = store.get_order(&entity_id) {
                let client_state: OrderPatch = match serde_json::from_value(payload) {
                    Ok(p) => p,
                    Err(_) => return,
                };

                let is_empty = serde_json::to_value(&client_state)
                    .map(|v| v.as_object().is_some_and(|o| o.is_empty()))
                    .unwrap_or(true);

                if is_empty {
                    let msg = OutboundMessage {
                        entity_tag: "Order".into(),
                        entity_id: entity_id.to_string(),
                        kind: MessageKind::Full,
                        payload: serde_json::to_value(&current).unwrap(),
                    };
                    let _ = socket.emit("sync", &msg);
                } else {
                    let client_full = OrderFull::default().merge(client_state);
                    if let Some(diff) = client_full.diff(&current) {
                        let msg = OutboundMessage {
                            entity_tag: "Order".into(),
                            entity_id: entity_id.to_string(),
                            kind: MessageKind::Patch,
                            payload: serde_json::to_value(&diff).unwrap(),
                        };
                        let _ = socket.emit("sync", &msg);
                    }
                }
            }
        }
        _ => {}
    }
}

/// Handle incoming commands from clients.
pub async fn on_command(
    socket: SocketRef,
    Data(data): Data<Value>,
    State(store): State<EntityStore>,
) {
    let request_id = data.get("requestId").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let command = data.get("command").and_then(|v| v.as_str()).unwrap_or("");
    let payload = data.get("payload").cloned().unwrap_or(Value::Object(Default::default()));

    match command {
        "PlaceOrder" => {
            let symbol = payload.get("symbol").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let side = payload.get("side").and_then(|v| v.as_str()).unwrap_or("buy").to_string();
            let entity_id_str = payload.get("entityId").and_then(|v| v.as_str()).unwrap_or("");

            let entity_id = if entity_id_str.is_empty() {
                Uuid::now_v7()
            } else {
                Uuid::parse_str(entity_id_str).unwrap_or_else(|_| Uuid::now_v7())
            };

            let order = OrderFull {
                symbol,
                side,
                status: "open".into(),
                total_quantity: Default::default(),
                filled_quantity: Default::default(),
                average_price: Default::default(),
                fills: Default::default(),
                notes: Default::default(),
            };

            store.set_order(entity_id, order.clone());

            let msg = OutboundMessage {
                entity_tag: "Order".into(),
                entity_id: entity_id.to_string(),
                kind: MessageKind::Full,
                payload: serde_json::to_value(&order).unwrap(),
            };
            // Broadcast to room (includes sender)
            let _ = socket.within("orders").emit("sync", &msg).await;

            let ack = serde_json::json!({
                "requestId": request_id,
                "status": "ok",
                "entityId": entity_id.to_string(),
            });
            let _ = socket.emit("ack", &ack);
        }
        "UpdateOrder" => {
            let entity_id_str = payload.get("entityId").and_then(|v| v.as_str()).unwrap_or("");
            let Ok(entity_id) = Uuid::parse_str(entity_id_str) else {
                return;
            };

            let patch_value = payload.get("patch").cloned().unwrap_or(Value::Object(Default::default()));
            let Ok(patch) = serde_json::from_value::<OrderPatch>(patch_value) else {
                return;
            };

            if let Some(applied_diff) = store.patch_order(&entity_id, patch) {
                let msg = OutboundMessage {
                    entity_tag: "Order".into(),
                    entity_id: entity_id.to_string(),
                    kind: MessageKind::Patch,
                    payload: serde_json::to_value(&applied_diff).unwrap(),
                };
                let _ = socket.within("orders").emit("sync", &msg).await;
            }

            let ack = serde_json::json!({
                "requestId": request_id,
                "status": "ok",
            });
            let _ = socket.emit("ack", &ack);
        }
        _ => {
            let ack = serde_json::json!({
                "requestId": request_id,
                "status": "error",
                "message": format!("Unknown command: {}", command),
            });
            let _ = socket.emit("ack", &ack);
        }
    }
}
