use crate::dto::message_dto::{
    AddMessageDto, GetMessagesDto, MessageStatusDto, ProjectedMessageDto,
};
use crate::error::api_error::ApiError;
use crate::state::message_state::MessageState;
use axum::{extract::State, Json};

pub async fn add_message(
    State(state): State<MessageState>,
    Json(payload): Json<AddMessageDto>,
) -> Result<Json<MessageStatusDto>, ApiError> {
    let res = state.message_service.add_message(payload).await?;
    Ok(Json(res))
}

pub async fn get_messages(
    State(state): State<MessageState>,
    Json(payload): Json<GetMessagesDto>,
) -> Result<Json<Vec<ProjectedMessageDto>>, ApiError> {
    let res = state.message_service.get_messages(payload).await?;
    Ok(Json(res))
}
