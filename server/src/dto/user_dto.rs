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

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct GoogleAuthDto {
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default, alias = "id_token", alias = "credential", alias = "token")]
    pub credential: Option<String>,
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default, alias = "redirect_uri")]
    pub redirect_uri: Option<String>,
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
    #[serde(rename = "authProvider")]
    pub auth_provider: String,
}

impl From<User> for UserDto {
    fn from(user: User) -> Self {
        Self {
            id: user.id.map(|oid| oid.to_hex()).unwrap_or_default(),
            username: user.username,
            email: user.email,
            is_avatar_image_set: user.is_avatar_image_set,
            avatar_image: user.avatar_image,
            auth_provider: user.auth_provider,
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
    pub username: String,
    pub email: String,
    #[serde(rename = "isAvatarImageSet")]
    pub is_avatar_image_set: bool,
    #[serde(rename = "avatarImage")]
    pub avatar_image: String,
}

impl From<User> for UserListItemDto {
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
pub struct SetAvatarDto {
    pub image: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SetAvatarResponseDto {
    #[serde(rename = "isSet")]
    pub is_set: bool,
    pub image: String,
}
