use crate::config::database::Database;
use crate::dto::user_dto::{
    AuthResponseDto, GoogleAuthDto, SetAvatarDto, SetAvatarResponseDto, UserDto, UserListItemDto,
    UserLoginDto, UserRegisterDto,
};
use crate::entity::user::User;
use crate::error::api_error::ApiError;
use crate::error::user_error::UserError;
use crate::repository::user_repository::{UserRepository, UserRepositoryTrait};
use bcrypt::{hash, verify, DEFAULT_COST};
use bson::oid::ObjectId;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
struct GoogleTokenPayload {
    sub: Option<String>,
    email: Option<String>,
    name: Option<String>,
    picture: Option<String>,
    aud: Option<String>,
    iss: Option<String>,
    email_verified: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GoogleTokenExchangeResponse {
    #[allow(dead_code)]
    access_token: Option<String>,
    id_token: Option<String>,
}

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

    pub async fn get_user(&self, id: &str) -> Result<UserDto, ApiError> {
        let id = ObjectId::parse_str(id).map_err(|_| UserError::InvalidUserId)?;
        self.user_repo
            .find_by_id(id)
            .await?
            .map(UserDto::from)
            .ok_or_else(|| UserError::UserNotFound.into())
    }

    pub fn get_google_config(&self) -> serde_json::Value {
        let client_id = std::env::var("GOOGLE_CLIENT_ID")
            .map(|v| v.trim_matches('"').to_string())
            .unwrap_or_default();
        let callback_url = std::env::var("GOOGLE_CALLBACK_URL")
            .map(|v| v.trim_matches('"').to_string())
            .unwrap_or_else(|_| "http://localhost:5173/".to_string());
        serde_json::json!({
            "status": true,
            "client_id": client_id,
            "callback_url": callback_url
        })
    }

    pub fn get_google_auth_url(&self) -> serde_json::Value {
        let client_id = std::env::var("GOOGLE_CLIENT_ID")
            .map(|v| v.trim_matches('"').to_string())
            .unwrap_or_default();
        let callback_url = std::env::var("GOOGLE_CALLBACK_URL")
            .map(|v| v.trim_matches('"').to_string())
            .unwrap_or_else(|_| "http://localhost:5173/".to_string());

        if client_id.is_empty() {
            return serde_json::json!({"status": false, "msg": "Google sign-in is not configured"});
        }
        let mut url = reqwest::Url::parse("https://accounts.google.com/o/oauth2/v2/auth")
            .expect("valid Google URL");
        url.query_pairs_mut().extend_pairs([
            ("client_id", client_id.as_str()),
            ("redirect_uri", callback_url.as_str()),
            ("response_type", "code"),
            ("scope", "openid email profile"),
            ("prompt", "select_account"),
        ]);
        serde_json::json!({
            "status": true,
            "url": url.as_str()
        })
    }

    pub async fn register(
        &self,
        mut payload: UserRegisterDto,
    ) -> Result<AuthResponseDto, ApiError> {
        payload.username = payload.username.trim().to_owned();
        payload.email = payload.email.trim().to_lowercase();
        validate_registration(&payload)?;
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

        if self
            .user_repo
            .find_by_email(&payload.email)
            .await?
            .is_some()
        {
            return Ok(AuthResponseDto {
                status: false,
                msg: Some("Email already used".to_string()),
                user: None,
            });
        }

        let password = payload.password;
        let hashed_password = tokio::task::spawn_blocking(move || hash(password, DEFAULT_COST))
            .await
            .map_err(|_| UserError::SomethingWentWrong("Password processing failed".into()))?
            .map_err(UserError::BcryptError)?;

        let new_user = User {
            id: None,
            username: payload.username,
            email: payload.email,
            password: hashed_password,
            is_avatar_image_set: false,
            avatar_image: String::new(),
            google_id: None,
            auth_provider: "local".to_string(),
            created_at: None,
            updated_at: None,
        };

        let created_user = self.user_repo.create(new_user).await?;

        Ok(AuthResponseDto {
            status: true,
            msg: None,
            user: Some(created_user.into()),
        })
    }

    pub async fn login(&self, payload: UserLoginDto) -> Result<AuthResponseDto, ApiError> {
        let user = self
            .user_repo
            .find_by_username(payload.username.trim())
            .await?;

        let user = match user {
            Some(u) => u,
            None => {
                return Ok(AuthResponseDto {
                    status: false,
                    msg: Some("Incorrect Username or Password".to_string()),
                    user: None,
                })
            }
        };

        if user.password.is_empty() || payload.password.len() > 72 {
            return Err(UserError::IncorrectCredentials.into());
        }
        let password = payload.password;
        let hashed = user.password.clone();
        let is_valid = tokio::task::spawn_blocking(move || verify(password, &hashed))
            .await
            .map_err(|_| UserError::SomethingWentWrong("Password processing failed".into()))?
            .map_err(UserError::BcryptError)?;

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

    pub async fn google_auth(&self, payload: GoogleAuthDto) -> Result<AuthResponseDto, ApiError> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|_| UserError::GoogleAuthFailed("OAuth client unavailable".into()))?;
        let mut id_token_to_verify = payload.credential.clone();

        // If authorization code provided, exchange for tokens
        if id_token_to_verify.as_deref().unwrap_or_default().is_empty() {
            if let Some(code) = payload.code {
                let client_id = std::env::var("GOOGLE_CLIENT_ID")
                    .map(|v| v.trim_matches('"').to_string())
                    .unwrap_or_default();
                let client_secret = std::env::var("GOOGLE_CLIENT_SECRET")
                    .map(|v| v.trim_matches('"').to_string())
                    .unwrap_or_default();
                let redirect_uri = payload.redirect_uri.unwrap_or_else(|| {
                    std::env::var("GOOGLE_CALLBACK_URL")
                        .map(|v| v.trim_matches('"').to_string())
                        .unwrap_or_else(|_| "http://localhost:5173/".to_string())
                });

                let params = [
                    ("client_id", client_id),
                    ("client_secret", client_secret),
                    ("code", code),
                    ("grant_type", "authorization_code".to_string()),
                    ("redirect_uri", redirect_uri),
                ];

                let exchange_res = client
                    .post("https://oauth2.googleapis.com/token")
                    .form(&params)
                    .send()
                    .await;

                if let Ok(res) = exchange_res {
                    if let Ok(token_data) = res.json::<GoogleTokenExchangeResponse>().await {
                        id_token_to_verify = token_data.id_token;
                    }
                }
            }
        }

        let token_str = id_token_to_verify.unwrap_or_default();
        if token_str.is_empty() {
            return Ok(AuthResponseDto {
                status: false,
                msg: Some("Missing Google ID token or authorization code".to_string()),
                user: None,
            });
        }

        let resp = client
            .get("https://oauth2.googleapis.com/tokeninfo")
            .query(&[("id_token", token_str)])
            .send()
            .await;
        let token_info = match resp {
            Ok(res) => {
                if !res.status().is_success() {
                    return Ok(AuthResponseDto {
                        status: false,
                        msg: Some("Invalid Google ID token".to_string()),
                        user: None,
                    });
                }
                res.json::<GoogleTokenPayload>()
                    .await
                    .map_err(|e| ApiError::User(UserError::SomethingWentWrong(e.to_string())))?
            }
            Err(e) => {
                return Ok(AuthResponseDto {
                    status: false,
                    msg: Some(format!("Failed to reach Google OAuth: {}", e)),
                    user: None,
                });
            }
        };

        let expected_client = crate::config::parameter::get_or("GOOGLE_CLIENT_ID", "");
        if expected_client.is_empty()
            || token_info.aud.as_deref() != Some(expected_client.as_str())
            || !matches!(
                token_info.iss.as_deref(),
                Some("accounts.google.com" | "https://accounts.google.com")
            )
            || token_info.email_verified.as_deref() != Some("true")
        {
            return Err(UserError::GoogleAuthFailed("Untrusted Google identity".into()).into());
        }
        let google_id = token_info.sub.unwrap_or_default();
        let email = token_info.email.unwrap_or_default().trim().to_lowercase();
        if google_id.is_empty() || email.is_empty() {
            return Ok(AuthResponseDto {
                status: false,
                msg: Some("Incomplete profile returned from Google".to_string()),
                user: None,
            });
        }

        if let Some(existing_user) = self.user_repo.find_by_google_id(&google_id).await? {
            return Ok(AuthResponseDto {
                status: true,
                msg: None,
                user: Some(UserDto::from(existing_user)),
            });
        }

        if let Some(existing_user) = self.user_repo.find_by_email(&email).await? {
            return Ok(AuthResponseDto {
                status: true,
                msg: None,
                user: Some(UserDto::from(existing_user)),
            });
        }

        let mut base_username = token_info
            .name
            .unwrap_or_else(|| email.split('@').next().unwrap_or("user").to_string())
            .replace(' ', "_")
            .to_lowercase();

        if self
            .user_repo
            .find_by_username(&base_username)
            .await?
            .is_some()
        {
            base_username = format!("{}_{}", base_username, &google_id[..4.min(google_id.len())]);
        }

        let avatar = token_info.picture.unwrap_or_default();
        let is_avatar_set = !avatar.is_empty();

        let new_user = User {
            id: None,
            username: base_username,
            email,
            password: String::new(),
            is_avatar_image_set: is_avatar_set,
            avatar_image: avatar,
            google_id: Some(google_id),
            auth_provider: "google".to_string(),
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

    pub async fn get_all_users(&self, user_id_str: &str) -> Result<Vec<UserListItemDto>, ApiError> {
        let object_id = ObjectId::parse_str(user_id_str).map_err(|_| UserError::InvalidUserId)?;
        let users = self.user_repo.find_all_except(object_id).await?;
        let user_dtos = users.into_iter().map(UserListItemDto::from).collect();
        Ok(user_dtos)
    }

    pub async fn set_avatar(
        &self,
        user_id_str: &str,
        payload: SetAvatarDto,
    ) -> Result<SetAvatarResponseDto, ApiError> {
        let object_id = ObjectId::parse_str(user_id_str).map_err(|_| UserError::InvalidUserId)?;
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

fn validate_registration(payload: &UserRegisterDto) -> Result<(), UserError> {
    if !(3..=32).contains(&payload.username.len())
        || !payload
            .username
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
    {
        return Err(UserError::InvalidInput(
            "Username must contain 3–32 letters, numbers, dots, hyphens or underscores".into(),
        ));
    }
    let valid_email = payload
        .email
        .split_once('@')
        .is_some_and(|(local, domain)| {
            !local.is_empty()
                && domain.contains('.')
                && !domain.starts_with('.')
                && !domain.ends_with('.')
                && !domain.contains('@')
        });
    if payload.email.len() > 254 || payload.email.chars().any(char::is_whitespace) || !valid_email {
        return Err(UserError::InvalidInput(
            "Please provide a valid email address".into(),
        ));
    }
    if !(8..=72).contains(&payload.password.len()) {
        return Err(UserError::InvalidInput(
            "Password must contain 8–72 bytes".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn registration_rejects_invalid_and_bcrypt_truncated_input() {
        let mut dto = UserRegisterDto {
            username: "alice".into(),
            email: "alice@example.com".into(),
            password: "password123".into(),
        };
        assert!(validate_registration(&dto).is_ok());
        dto.password = "a".repeat(73);
        assert!(validate_registration(&dto).is_err());
        dto.password = "password123".into();
        dto.email = "alice@@example.com".into();
        assert!(validate_registration(&dto).is_err());
        dto.email = "alice@example.com".into();
        dto.username = " ".into();
        assert!(validate_registration(&dto).is_err());
    }
}
