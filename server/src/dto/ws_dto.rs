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
    pub username: Option<String>,
    #[serde(default)]
    pub avatar: Option<String>,
    #[serde(rename = "sessionId", default)]
    pub session_id: Option<String>,
    #[serde(rename = "targetUserId", default)]
    pub target_user_id: Option<String>,
    #[serde(default)]
    pub to: Option<String>,
    #[serde(default)]
    pub msg: Option<String>,
    #[serde(default)]
    pub ciphertext: Option<String>,
    #[serde(default)]
    pub iv: Option<String>,
    #[serde(default)]
    pub sdp: Option<serde_json::Value>,
    #[serde(default)]
    pub candidate: Option<serde_json::Value>,
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
