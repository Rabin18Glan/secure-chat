use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct ApiSuccessResponse<T: Serialize> {
    pub data: T,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ApiErrorResponse {
    pub message: Option<String>,
    #[serde(rename = "code")]
    pub status: u16,
}

#[allow(dead_code)]
impl<T: Serialize> ApiSuccessResponse<T>
where
    T: Serialize,
{
    pub fn send(data: T) -> Self {
        ApiSuccessResponse { data }
    }
}

impl ApiErrorResponse {
    pub fn send(status: u16, message: Option<String>) -> Response {
        ApiErrorResponse { message, status }.into_response()
    }
}

impl IntoResponse for ApiErrorResponse {
    fn into_response(self) -> Response {
        (
            StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            Json(self),
        )
            .into_response()
    }
}
