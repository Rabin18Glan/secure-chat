use crate::config::database::{Database, DatabaseTrait};
use crate::entity::session::Session;
use crate::error::db_error::DbError;
use async_trait::async_trait;
use bson::{doc, oid::ObjectId};
use mongodb::Collection;
use std::sync::Arc;

#[derive(Clone)]
pub struct SessionRepository {
    collection: Collection<Session>,
}

#[async_trait]
pub trait SessionRepositoryTrait {
    fn new(db_conn: &Arc<Database>) -> Self;
    async fn create(&self, session: Session) -> Result<Session, DbError>;
    async fn find_by_session_id(&self, session_id: &str) -> Result<Option<Session>, DbError>;
    async fn add_admin(&self, session_id: &str, user_id: ObjectId) -> Result<(), DbError>;
    async fn remove_admin(&self, session_id: &str, user_id: ObjectId) -> Result<(), DbError>;
    #[allow(dead_code)]
    async fn add_member(&self, session_id: &str, user_id: ObjectId) -> Result<(), DbError>;
    async fn remove_member(&self, session_id: &str, user_id: ObjectId) -> Result<(), DbError>;
    async fn delete_session(&self, session_id: &str) -> Result<(), DbError>;
}

#[async_trait]
impl SessionRepositoryTrait for SessionRepository {
    fn new(db_conn: &Arc<Database>) -> Self {
        let collection = db_conn.get_database().collection::<Session>("sessions");
        Self { collection }
    }

    async fn create(&self, mut session: Session) -> Result<Session, DbError> {
        let now = bson::DateTime::now();
        session.created_at = Some(now);
        session.updated_at = Some(now);
        let result = self.collection.insert_one(&session).await?;
        session.id = result.inserted_id.as_object_id();
        Ok(session)
    }

    async fn find_by_session_id(&self, session_id: &str) -> Result<Option<Session>, DbError> {
        let session = self
            .collection
            .find_one(doc! { "sessionId": session_id })
            .await?;
        Ok(session)
    }

    async fn add_admin(&self, session_id: &str, user_id: ObjectId) -> Result<(), DbError> {
        self.collection
            .update_one(
                doc! { "sessionId": session_id },
                doc! {
                    "$addToSet": { "admins": user_id },
                    "$set": { "updatedAt": bson::DateTime::now() }
                },
            )
            .await?;
        Ok(())
    }

    async fn remove_admin(&self, session_id: &str, user_id: ObjectId) -> Result<(), DbError> {
        self.collection
            .update_one(
                doc! { "sessionId": session_id },
                doc! {
                    "$pull": { "admins": user_id },
                    "$set": { "updatedAt": bson::DateTime::now() }
                },
            )
            .await?;
        Ok(())
    }

    async fn add_member(&self, session_id: &str, user_id: ObjectId) -> Result<(), DbError> {
        self.collection
            .update_one(
                doc! { "sessionId": session_id },
                doc! {
                    "$addToSet": { "members": user_id },
                    "$set": { "updatedAt": bson::DateTime::now() }
                },
            )
            .await?;
        Ok(())
    }

    async fn remove_member(&self, session_id: &str, user_id: ObjectId) -> Result<(), DbError> {
        self.collection
            .update_one(
                doc! { "sessionId": session_id },
                doc! {
                    "$pull": { "members": user_id, "admins": user_id },
                    "$set": { "updatedAt": bson::DateTime::now() }
                },
            )
            .await?;
        Ok(())
    }

    async fn delete_session(&self, session_id: &str) -> Result<(), DbError> {
        self.collection
            .delete_one(doc! { "sessionId": session_id })
            .await?;
        Ok(())
    }
}
