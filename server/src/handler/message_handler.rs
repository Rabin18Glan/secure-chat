use crate::dto::message_dto::{
    AddMessageDto, GetMessagesDto, MessageStatusDto, ProjectedMessageDto,
};
use crate::error::api_error::ApiError;
use crate::state::auth_state::{require_actor, AuthUser};
use crate::state::message_state::MessageState;
use axum::Extension;
use axum::{extract::State, Json};

pub async fn add_message(
    Extension(user): Extension<AuthUser>,
    State(state): State<MessageState>,
    Json(payload): Json<AddMessageDto>,
) -> Result<Json<MessageStatusDto>, ApiError> {
    require_actor(&user, &payload.from)?;
    let res = state.message_service.add_message(payload).await?;
    Ok(Json(res))
}

pub async fn get_messages(
    Extension(user): Extension<AuthUser>,
    State(state): State<MessageState>,
    Json(payload): Json<GetMessagesDto>,
) -> Result<Json<Vec<ProjectedMessageDto>>, ApiError> {
    require_actor(&user, &payload.from)?;
    let res = state.message_service.get_messages(payload).await?;
    Ok(Json(res))
}
