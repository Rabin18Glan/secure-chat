use crate::response::api_response::ApiErrorResponse;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

#[allow(dead_code)]
#[derive(Error, Debug)]
pub enum UserError {
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
}

impl IntoResponse for UserError {
    fn into_response(self) -> Response {
        let status_code = match self {
            UserError::UserNotFound => StatusCode::NOT_FOUND,
            UserError::InvalidUserId => StatusCode::BAD_REQUEST,
            UserError::UsernameAlreadyUsed => StatusCode::BAD_REQUEST,
            UserError::EmailAlreadyUsed => StatusCode::BAD_REQUEST,
            UserError::IncorrectCredentials => StatusCode::UNAUTHORIZED,
            UserError::BcryptError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        ApiErrorResponse::send(status_code.as_u16(), Some(self.to_string()))
    }
}
