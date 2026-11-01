use crate::entity::session::Session;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateSessionDto {
    pub name: Option<String>,
    #[serde(rename = "ownerId", alias = "owner_id")]
    pub owner_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SessionInfoDto {
    #[serde(rename = "_id")]
    pub id: String,
    #[serde(rename = "sessionId")]
    pub session_id: String,
    pub name: String,
    #[serde(rename = "ownerId", alias = "owner_id")]
    pub owner_id: String,
    pub admins: Vec<String>,
    pub members: Vec<String>,
}

impl From<Session> for SessionInfoDto {
    fn from(s: Session) -> Self {
        Self {
            id: s.id.map(|oid| oid.to_hex()).unwrap_or_default(),
            session_id: s.session_id,
            name: s.name,
            owner_id: s.owner_id.to_hex(),
            admins: s.admins.into_iter().map(|a| a.to_hex()).collect(),
            members: s.members.into_iter().map(|m| m.to_hex()).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn create_session_accepts_browser_and_legacy_fields() {
        for field in ["ownerId", "owner_id"] {
            let value = serde_json::json!({field: "507f1f77bcf86cd799439011", "name": "Review"});
            let dto: CreateSessionDto = serde_json::from_value(value).unwrap();
            assert_eq!(dto.owner_id, "507f1f77bcf86cd799439011");
        }
    }
}
