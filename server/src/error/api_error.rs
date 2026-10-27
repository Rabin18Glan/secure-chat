use crate::error::{db_error::DbError, message_error::MessageError, user_error::UserError};
use axum::response::{IntoResponse, Response};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error(transparent)]
    UserError(#[from] UserError),
    #[error(transparent)]
    MessageError(#[from] MessageError),
    #[error(transparent)]
    DbError(#[from] DbError),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            ApiError::UserError(error) => error.into_response(),
            ApiError::MessageError(error) => error.into_response(),
            ApiError::DbError(error) => error.into_response(),
        }
    }
}
