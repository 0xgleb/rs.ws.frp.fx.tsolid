use serde::{Deserialize, Serialize};

/// The kind of entity message: full state or incremental patch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageKind {
    Full,
    Patch,
}

/// Server → Client message envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutboundMessage {
    pub entity_tag: String,
    pub entity_id: String,
    pub kind: MessageKind,
    pub payload: serde_json::Value,
}

/// Client → Server command message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InboundCommand {
    pub request_id: String,
    pub command: String,
    pub payload: serde_json::Value,
}

/// Client → Server handshake message (what the client currently holds).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InboundHandshake {
    pub entity_tag: String,
    pub entity_id: String,
    pub kind: MessageKind,
    pub payload: serde_json::Value,
}

/// Client → Server message (either a command or a handshake).
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
