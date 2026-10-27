use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AddMessageDto {
    pub from: String,
    pub to: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetMessagesDto {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProjectedMessageDto {
    #[serde(rename = "fromSelf")]
    pub from_self: bool,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MessageStatusDto {
    pub msg: String,
}
