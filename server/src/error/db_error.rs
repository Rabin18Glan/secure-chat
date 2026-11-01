use crate::response::api_response::ApiErrorResponse;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

#[allow(dead_code)]
#[derive(Error, Debug)]
pub enum DbError {
    #[error("Database error: {0}")]
    MongoError(#[from] mongodb::error::Error),
    #[error("Database error: {0}")]
    BsonError(#[from] bson::de::Error),
    #[error("Database error: {0}")]
    BsonSerError(#[from] bson::ser::Error),
    #[error("Database error: {0}")]
    SomethingWentWrong(String),
}

impl IntoResponse for DbError {
    fn into_response(self) -> Response {
        if let Self::MongoError(error) = &self {
            if matches!(error.kind.as_ref(), mongodb::error::ErrorKind::Write(mongodb::error::WriteFailure::WriteError(write)) if write.code == 11000)
            {
                return ApiErrorResponse::send(
                    StatusCode::CONFLICT.as_u16(),
                    Some("This username or email is already registered".into()),
                );
            }
        }
        tracing::error!(error = %self, "Database operation failed");
        ApiErrorResponse::send(
            StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
            Some("Database operation failed. Please try again.".into()),
        )
    }
}
