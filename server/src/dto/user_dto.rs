use crate::entity::user::User;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserRegisterDto {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserLoginDto {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserDto {
    #[serde(rename = "_id")]
    pub id: String,
    pub username: String,
    pub email: String,
    #[serde(rename = "isAvatarImageSet")]
    pub is_avatar_image_set: bool,
    #[serde(rename = "avatarImage")]
    pub avatar_image: String,
}

impl From<User> for UserDto {
    fn from(user: User) -> Self {
        Self {
            id: user.id.map(|oid| oid.to_hex()).unwrap_or_default(),
            username: user.username,
            email: user.email,
            is_avatar_image_set: user.is_avatar_image_set,
            avatar_image: user.avatar_image,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthResponseDto {
    pub status: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub msg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<UserDto>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserListItemDto {
    #[serde(rename = "_id")]
    pub id: String,
    pub email: String,
    pub username: String,
    #[serde(rename = "avatarImage")]
    pub avatar_image: String,
}

impl From<User> for UserListItemDto {
    fn from(user: User) -> Self {
        Self {
            id: user.id.map(|oid| oid.to_hex()).unwrap_or_default(),
            email: user.email,
            username: user.username,
            avatar_image: user.avatar_image,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SetAvatarDto {
    pub image: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SetAvatarResponseDto {
    #[serde(rename = "isSet")]
    pub is_set: bool,
    pub image: String,
}
