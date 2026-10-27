use crate::config::database::{Database, DatabaseTrait};
use crate::entity::user::User;
use crate::error::db_error::DbError;
use async_trait::async_trait;
use bson::{doc, oid::ObjectId};
use futures_util::TryStreamExt;
use mongodb::options::{FindOneAndUpdateOptions, ReturnDocument};
use mongodb::Collection;
use std::sync::Arc;

#[derive(Clone)]
pub struct UserRepository {
    collection: Collection<User>,
}

#[async_trait]
pub trait UserRepositoryTrait {
    fn new(db_conn: &Arc<Database>) -> Self;
    async fn find_by_username(&self, username: &str) -> Result<Option<User>, DbError>;
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, DbError>;
    #[allow(dead_code)]
    async fn find_by_id(&self, id: ObjectId) -> Result<Option<User>, DbError>;
    async fn find_all_except(&self, except_id: ObjectId) -> Result<Vec<User>, DbError>;
    async fn create(&self, user: User) -> Result<User, DbError>;
    async fn update_avatar(
        &self,
        id: ObjectId,
        avatar_image: &str,
    ) -> Result<Option<User>, DbError>;
}

#[async_trait]
impl UserRepositoryTrait for UserRepository {
    fn new(db_conn: &Arc<Database>) -> Self {
        let collection = db_conn.get_database().collection::<User>("users");
        Self { collection }
    }

    async fn find_by_username(&self, username: &str) -> Result<Option<User>, DbError> {
        let user = self
            .collection
            .find_one(doc! { "username": username })
            .await?;
        Ok(user)
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<User>, DbError> {
        let user = self.collection.find_one(doc! { "email": email }).await?;
        Ok(user)
    }

    async fn find_by_id(&self, id: ObjectId) -> Result<Option<User>, DbError> {
        let user = self.collection.find_one(doc! { "_id": id }).await?;
        Ok(user)
    }

    async fn find_all_except(&self, except_id: ObjectId) -> Result<Vec<User>, DbError> {
        let cursor = self
            .collection
            .find(doc! { "_id": { "$ne": except_id } })
            .await?;
        let users: Vec<User> = cursor.try_collect().await?;
        Ok(users)
    }

    async fn create(&self, mut user: User) -> Result<User, DbError> {
        let now = bson::DateTime::now();
        user.created_at = Some(now);
        user.updated_at = Some(now);
        let insert_result = self.collection.insert_one(&user).await?;
        user.id = insert_result.inserted_id.as_object_id();
        Ok(user)
    }

    async fn update_avatar(
        &self,
        id: ObjectId,
        avatar_image: &str,
    ) -> Result<Option<User>, DbError> {
        let options = FindOneAndUpdateOptions::builder()
            .return_document(ReturnDocument::After)
            .build();

        let updated_user = self
            .collection
            .find_one_and_update(
                doc! { "_id": id },
                doc! {
                    "$set": {
                        "isAvatarImageSet": true,
                        "avatarImage": avatar_image,
                        "updatedAt": bson::DateTime::now()
                    }
                },
            )
            .with_options(options)
            .await?;

        Ok(updated_user)
    }
}
