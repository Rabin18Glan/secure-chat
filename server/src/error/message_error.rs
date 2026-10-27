use crate::response::api_response::ApiErrorResponse;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

#[allow(dead_code)]
#[derive(Error, Debug)]
pub enum MessageError {
    #[error("Failed to add message to the database")]
    AddMessageFailed,
    #[error("Invalid user ID")]
    InvalidUserId,
}

impl IntoResponse for MessageError {
    fn into_response(self) -> Response {
        let status_code = match self {
            MessageError::AddMessageFailed => StatusCode::INTERNAL_SERVER_ERROR,
            MessageError::InvalidUserId => StatusCode::BAD_REQUEST,
        };

        ApiErrorResponse::send(status_code.as_u16(), Some(self.to_string()))
    }
}
