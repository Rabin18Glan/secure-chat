use crate::response::api_response::ApiErrorResponse;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

#[allow(dead_code)]
#[derive(Error, Debug)]
pub enum UserError {
    #[error("You do not have permission to perform this action")]
    Forbidden,
    #[error("{0}")]
    InvalidInput(String),
    #[error("User not found")]
    UserNotFound,
    #[error("Invalid user ID")]
    InvalidUserId,
    #[error("Username already used")]
    UsernameAlreadyUsed,
    #[error("Email already used")]
    EmailAlreadyUsed,
    #[error("Incorrect Username or Password")]
    IncorrectCredentials,
    #[error("Password hashing error: {0}")]
    BcryptError(#[from] bcrypt::BcryptError),
    #[error("Google authentication failed: {0}")]
    GoogleAuthFailed(String),
    #[error("{0}")]
    SomethingWentWrong(String),
}

impl IntoResponse for UserError {
    fn into_response(self) -> Response {
        let status_code = match self {
            UserError::Forbidden => StatusCode::FORBIDDEN,
            UserError::InvalidInput(_) => StatusCode::BAD_REQUEST,
            UserError::UserNotFound => StatusCode::NOT_FOUND,
            UserError::InvalidUserId => StatusCode::BAD_REQUEST,
            UserError::UsernameAlreadyUsed => StatusCode::BAD_REQUEST,
            UserError::EmailAlreadyUsed => StatusCode::BAD_REQUEST,
            UserError::IncorrectCredentials => StatusCode::UNAUTHORIZED,
            UserError::BcryptError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            UserError::GoogleAuthFailed(_) => StatusCode::UNAUTHORIZED,
            UserError::SomethingWentWrong(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        ApiErrorResponse::send(status_code.as_u16(), Some(self.to_string()))
    }
}
