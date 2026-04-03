use crate::state::EntityStore;
use domain::{OrderFull, OrderPatch};
use serde_json::Value;
use socketioxide::extract::{Data, SocketRef, State};
use sync_core::message::{MessageKind, OutboundMessage};
use sync_core::Diffable;
use uuid::Uuid;

/// Handle the sync handshake: client sends its current state, server
/// responds with the minimal update needed to synchronize.
///
/// ## Protocol
///
/// The client emits a `"handshake"` event with:
/// ```json
/// {
///   "entityTag": "Order",
///   "entityId": "<uuid>",
///   "kind": "patch",
///   "payload": { /* client's current state as patch */ }
/// }
/// ```
///
/// The server:
/// 1. Looks up the entity by tag and ID.
/// 2. If the client payload is empty, responds with a `Full` message
///    (complete entity state).
/// 3. If the client has state, reconstructs the client's view by merging
///    the payload into a default, diffs it against the server's canonical
///    state, and responds with a `Patch` message (only changed fields).
/// 4. If no diff is needed (client is up to date), sends nothing.
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
///
/// ## Supported commands
///
/// ### `PlaceOrder`
///
/// Creates a new order with the given symbol and side. Broadcasts a
/// `Full` message to all clients in the `"orders"` room.
///
/// Payload:
/// ```json
/// {
///   "requestId": "<uuid>",
///   "command": "PlaceOrder",
///   "payload": {
///     "symbol": "BTC",
///     "side": "buy",
///     "entityId": "<optional uuid>"
///   }
/// }
/// ```
///
/// ### `UpdateOrder`
///
/// Applies a patch to an existing order. Broadcasts the computed diff
/// (not the raw input patch) to all clients, ensuring only actual
/// changes are transmitted.
///
/// Payload:
/// ```json
/// {
///   "requestId": "<uuid>",
///   "command": "UpdateOrder",
///   "payload": {
///     "entityId": "<uuid>",
///     "patch": { "status": "filled", "filledQuantity": "1000" }
///   }
/// }
/// ```
///
/// ## Acknowledgment
///
/// All commands receive an `"ack"` event with:
/// ```json
/// { "requestId": "<uuid>", "status": "ok" }
/// ```
///
/// Unknown commands get `"status": "error"` with a message.
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
