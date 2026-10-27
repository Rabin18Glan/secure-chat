use crate::config::database::Database;
use crate::dto::user_dto::{
    AuthResponseDto, SetAvatarDto, SetAvatarResponseDto, UserDto, UserListItemDto,
    UserLoginDto, UserRegisterDto,
};
use crate::entity::user::User;
use crate::error::api_error::ApiError;
use crate::error::user_error::UserError;
use crate::repository::user_repository::{UserRepository, UserRepositoryTrait};
use bson::oid::ObjectId;
use std::sync::Arc;

#[derive(Clone)]
pub struct UserService {
    user_repo: UserRepository,
}

impl UserService {
    pub fn new(db_conn: &Arc<Database>) -> Self {
        Self {
            user_repo: UserRepository::new(db_conn),
        }
    }

    pub async fn register(&self, payload: UserRegisterDto) -> Result<AuthResponseDto, ApiError> {
        if self
            .user_repo
            .find_by_username(&payload.username)
            .await?
            .is_some()
        {
            return Ok(AuthResponseDto {
                status: false,
                msg: Some("Username already used".to_string()),
                user: None,
            });
        }

        if self.user_repo.find_by_email(&payload.email).await?.is_some() {
            return Ok(AuthResponseDto {
                status: false,
                msg: Some("Email already used".to_string()),
                user: None,
            });
        }

        let hashed_password = bcrypt::hash(&payload.password, 10)
            .map_err(|e| UserError::BcryptError(e))?;

        let new_user = User {
            id: None,
            username: payload.username,
            email: payload.email,
            password: hashed_password,
            is_avatar_image_set: false,
            avatar_image: String::new(),
            created_at: None,
            updated_at: None,
        };

        let created_user = self.user_repo.create(new_user).await?;

        Ok(AuthResponseDto {
            status: true,
            msg: None,
            user: Some(UserDto::from(created_user)),
        })
    }

    pub async fn login(&self, payload: UserLoginDto) -> Result<AuthResponseDto, ApiError> {
        let user = match self.user_repo.find_by_username(&payload.username).await? {
            Some(u) => u,
            None => {
                return Ok(AuthResponseDto {
                    status: false,
                    msg: Some("Incorrect Username or Password".to_string()),
                    user: None,
                });
            }
        };

        let is_valid = bcrypt::verify(&payload.password, &user.password).unwrap_or(false);
        if !is_valid {
            return Ok(AuthResponseDto {
                status: false,
                msg: Some("Incorrect Username or Password".to_string()),
                user: None,
            });
        }

        Ok(AuthResponseDto {
            status: true,
            msg: None,
            user: Some(UserDto::from(user)),
        })
    }

    pub async fn get_all_users(&self, user_id_str: &str) -> Result<Vec<UserListItemDto>, ApiError> {
        let object_id =
            ObjectId::parse_str(user_id_str).map_err(|_| UserError::InvalidUserId)?;
        let users = self.user_repo.find_all_except(object_id).await?;
        let user_dtos = users.into_iter().map(UserListItemDto::from).collect();
        Ok(user_dtos)
    }

    pub async fn set_avatar(
        &self,
        user_id_str: &str,
        payload: SetAvatarDto,
    ) -> Result<SetAvatarResponseDto, ApiError> {
        let object_id =
            ObjectId::parse_str(user_id_str).map_err(|_| UserError::InvalidUserId)?;
        let updated_user = self
            .user_repo
            .update_avatar(object_id, &payload.image)
            .await?;

        match updated_user {
            Some(u) => Ok(SetAvatarResponseDto {
                is_set: u.is_avatar_image_set,
                image: u.avatar_image,
            }),
            None => Err(UserError::UserNotFound.into()),
        }
    }
}
