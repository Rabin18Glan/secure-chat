use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WsIncomingMessage {
    #[serde(default)]
    pub event: Option<String>,
    #[serde(rename = "type", default)]
    pub message_type: Option<String>,
    #[serde(rename = "userId", default)]
    pub user_id: Option<String>,
    #[serde(default)]
    pub to: Option<String>,
    #[serde(default)]
    pub msg: Option<String>,
    #[serde(default)]
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WsOutgoingMessage {
    pub event: String,
    #[serde(rename = "type")]
    pub message_type: String,
    pub msg: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
}
