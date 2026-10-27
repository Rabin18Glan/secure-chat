use crate::config::database::{Database, DatabaseTrait};
use crate::entity::message::Message;
use crate::error::db_error::DbError;
use async_trait::async_trait;
use bson::doc;
use futures_util::TryStreamExt;
use mongodb::Collection;
use std::sync::Arc;

#[derive(Clone)]
pub struct MessageRepository {
    collection: Collection<Message>,
}

#[async_trait]
pub trait MessageRepositoryTrait {
    fn new(db_conn: &Arc<Database>) -> Self;
    async fn create(&self, message: Message) -> Result<Message, DbError>;
    async fn find_between_users(&self, from: &str, to: &str) -> Result<Vec<Message>, DbError>;
}

#[async_trait]
impl MessageRepositoryTrait for MessageRepository {
    fn new(db_conn: &Arc<Database>) -> Self {
        let collection = db_conn.get_database().collection::<Message>("messages");
        Self { collection }
    }

    async fn create(&self, mut message: Message) -> Result<Message, DbError> {
        let now = bson::DateTime::now();
        message.created_at = Some(now);
        message.updated_at = Some(now);
        let result = self.collection.insert_one(&message).await?;
        message.id = result.inserted_id.as_object_id();
        Ok(message)
    }

    async fn find_between_users(&self, from: &str, to: &str) -> Result<Vec<Message>, DbError> {
        let filter = doc! {
            "users": {
                "$all": [from, to]
            }
        };
        let cursor = self
            .collection
            .find(filter)
            .sort(doc! { "updatedAt": 1, "_id": 1 })
            .await?;
        let messages: Vec<Message> = cursor.try_collect().await?;
        Ok(messages)
    }
}
