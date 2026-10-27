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
