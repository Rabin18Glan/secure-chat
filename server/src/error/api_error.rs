use crate::error::{db_error::DbError, message_error::MessageError, user_error::UserError};
use axum::response::{IntoResponse, Response};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error(transparent)]
    User(#[from] UserError),
    #[error(transparent)]
    Message(#[from] MessageError),
    #[error(transparent)]
    Database(#[from] DbError),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            ApiError::User(error) => error.into_response(),
            ApiError::Message(error) => error.into_response(),
            ApiError::Database(error) => error.into_response(),
        }
    }
}
