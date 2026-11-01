use bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Session {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    #[serde(rename = "sessionId")]
    pub session_id: String,
    pub name: String,
    #[serde(rename = "ownerId")]
    pub owner_id: ObjectId, // Super Admin
    #[serde(default)]
    pub admins: Vec<ObjectId>,
    #[serde(default)]
    pub members: Vec<ObjectId>,
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<bson::DateTime>,
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<bson::DateTime>,
}
