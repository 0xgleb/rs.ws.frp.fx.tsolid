use serde::{Deserialize, Serialize};

/// The kind of entity message: full state snapshot or incremental patch.
///
/// Used in both [`OutboundMessage`] (server-to-client) and
/// [`InboundHandshake`] (client-to-server) to indicate whether the
/// payload contains a complete entity representation or a sparse diff.
///
/// # Serialization
///
/// Serializes to lowercase JSON strings:
///
/// ```rust
/// use sync_core::MessageKind;
///
/// let json = serde_json::to_value(MessageKind::Full).unwrap();
/// assert_eq!(json, "full");
///
/// let json = serde_json::to_value(MessageKind::Patch).unwrap();
/// assert_eq!(json, "patch");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageKind {
    Full,
    Patch,
}

/// Server-to-client message envelope.
///
/// Each message identifies the entity type (`entity_tag`), the specific
/// entity instance (`entity_id`), whether this is a full snapshot or
/// incremental patch (`kind`), and the serialized payload.
///
/// # Wire format
///
/// Fields are serialized as camelCase JSON:
///
/// ```rust
/// use sync_core::message::{OutboundMessage, MessageKind};
///
/// let msg = OutboundMessage {
///     entity_tag: "Order".into(),
///     entity_id: "01912345-6789-7abc-8000-000000000001".into(),
///     kind: MessageKind::Full,
///     payload: serde_json::json!({
///         "symbol": "BTC",
///         "side": "buy",
///         "status": "open"
///     }),
/// };
///
/// let json = serde_json::to_value(&msg).unwrap();
/// assert_eq!(json["entityTag"], "Order");
/// assert_eq!(json["kind"], "full");
/// assert_eq!(json["payload"]["symbol"], "BTC");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutboundMessage {
    pub entity_tag: String,
    pub entity_id: String,
    pub kind: MessageKind,
    pub payload: serde_json::Value,
}

/// Client-to-server command message.
///
/// Commands are application-level actions (e.g., "PlaceOrder",
/// "UpdateOrder") sent by the client. The `request_id` enables
/// the server to send an acknowledgment back.
///
/// # Wire format
///
/// ```rust
/// use sync_core::message::InboundCommand;
///
/// let json = serde_json::json!({
///     "requestId": "550e8400-e29b-41d4-a716-446655440000",
///     "command": "PlaceOrder",
///     "payload": { "symbol": "ETH", "side": "buy" }
/// });
///
/// let cmd: InboundCommand = serde_json::from_value(json).unwrap();
/// assert_eq!(cmd.command, "PlaceOrder");
/// assert_eq!(cmd.payload["symbol"], "ETH");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InboundCommand {
    pub request_id: String,
    pub command: String,
    pub payload: serde_json::Value,
}

/// Client-to-server handshake message declaring the client's current state
/// for a specific entity.
///
/// On connect (or reconnect), the client sends one handshake per entity it
/// currently holds. The server compares this against its canonical state
/// and responds with either a Full (if client state is empty) or a Patch
/// (minimal diff to bring the client up to date).
///
/// # Wire format
///
/// ```rust
/// use sync_core::message::{InboundHandshake, MessageKind};
///
/// let json = serde_json::json!({
///     "entityTag": "Order",
///     "entityId": "550e8400-e29b-41d4-a716-446655440000",
///     "kind": "patch",
///     "payload": { "status": "open" }
/// });
///
/// let hs: InboundHandshake = serde_json::from_value(json).unwrap();
/// assert_eq!(hs.entity_tag, "Order");
/// assert_eq!(hs.kind, MessageKind::Patch);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InboundHandshake {
    pub entity_tag: String,
    pub entity_id: String,
    pub kind: MessageKind,
    pub payload: serde_json::Value,
}

/// Client-to-server message: either a [`InboundCommand`] or an
/// [`InboundHandshake`].
///
/// Uses `#[serde(untagged)]` for deserialization -- the parser tries
/// each variant in order. Commands have `requestId` + `command` fields,
/// handshakes have `entityTag` + `entityId`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum InboundMessage {
    Command(InboundCommand),
    Handshake(InboundHandshake),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outbound_message_serialization() {
        let msg = OutboundMessage {
            entity_tag: "Order".into(),
            entity_id: "01912345-6789-7abc-8000-000000000001".into(),
            kind: MessageKind::Full,
            payload: serde_json::json!({"status": "open"}),
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["entityTag"], "Order");
        assert_eq!(json["kind"], "full");
    }

    #[test]
    fn inbound_command_deserialization() {
        let json = serde_json::json!({
            "requestId": "01912345-6789-7abc-8000-000000000001",
            "command": "PlaceOrder",
            "payload": {"symbol": "BTC"}
        });
        let msg: InboundCommand = serde_json::from_value(json).unwrap();
        assert_eq!(msg.command, "PlaceOrder");
    }
}
