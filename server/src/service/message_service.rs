use crate::config::database::Database;
use crate::dto::message_dto::{
    AddMessageDto, GetMessagesDto, MessageStatusDto, ProjectedMessageDto,
};
use crate::entity::message::{Message, MessageText};
use crate::error::api_error::ApiError;
use crate::error::message_error::MessageError;
use crate::repository::message_repository::{MessageRepository, MessageRepositoryTrait};
use bson::oid::ObjectId;
use std::sync::Arc;

#[derive(Clone)]
pub struct MessageService {
    message_repo: MessageRepository,
}

impl MessageService {
    pub fn new(db_conn: &Arc<Database>) -> Self {
        Self {
            message_repo: MessageRepository::new(db_conn),
        }
    }

    pub async fn add_message(&self, payload: AddMessageDto) -> Result<MessageStatusDto, ApiError> {
        validate_parties(&payload.from, &payload.to)?;
        if payload.message.trim().is_empty() || payload.message.len() > 16_384 {
            return Err(crate::error::user_error::UserError::InvalidInput(
                "Message must contain 1–16384 bytes".into(),
            )
            .into());
        }
        let sender_oid =
            ObjectId::parse_str(&payload.from).map_err(|_| MessageError::InvalidUserId)?;

        let message_entity = Message {
            id: None,
            message: MessageText {
                text: payload.message,
            },
            users: vec![payload.from, payload.to],
            sender: sender_oid,
            created_at: None,
            updated_at: None,
        };

        self.message_repo.create(message_entity).await?;

        Ok(MessageStatusDto {
            msg: "Message added successfully.".to_string(),
        })
    }

    pub async fn get_messages(
        &self,
        payload: GetMessagesDto,
    ) -> Result<Vec<ProjectedMessageDto>, ApiError> {
        validate_parties(&payload.from, &payload.to)?;
        let messages = self
            .message_repo
            .find_between_users(&payload.from, &payload.to)
            .await?;

        let projected = messages
            .into_iter()
            .map(|msg| {
                let from_self = msg.sender.to_hex() == payload.from;
                ProjectedMessageDto {
                    from_self,
                    message: msg.message.text,
                }
            })
            .collect();

        Ok(projected)
    }
}

fn validate_parties(from: &str, to: &str) -> Result<(), ApiError> {
    ObjectId::parse_str(from).map_err(|_| MessageError::InvalidUserId)?;
    ObjectId::parse_str(to).map_err(|_| MessageError::InvalidUserId)?;
    if from == to {
        return Err(crate::error::user_error::UserError::InvalidInput(
            "Choose another user to message".into(),
        )
        .into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn message_parties_must_be_valid_and_distinct() {
        let from = "507f1f77bcf86cd799439011";
        assert!(validate_parties(from, "507f1f77bcf86cd799439012").is_ok());
        assert!(validate_parties(from, "invalid").is_err());
        assert!(validate_parties(from, from).is_err());
    }
}
