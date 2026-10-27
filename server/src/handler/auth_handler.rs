use crate::dto::user_dto::{
    AuthResponseDto, SetAvatarDto, SetAvatarResponseDto, UserListItemDto, UserLoginDto,
    UserRegisterDto,
};
use crate::error::api_error::ApiError;
use crate::state::user_state::UserState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

pub async fn register(
    State(state): State<UserState>,
    Json(payload): Json<UserRegisterDto>,
) -> Result<Json<AuthResponseDto>, ApiError> {
    let res = state.user_service.register(payload).await?;
    Ok(Json(res))
}

pub async fn login(
    State(state): State<UserState>,
    Json(payload): Json<UserLoginDto>,
) -> Result<Json<AuthResponseDto>, ApiError> {
    let res = state.user_service.login(payload).await?;
    Ok(Json(res))
}

pub async fn get_all_users(
    State(state): State<UserState>,
    Path(id): Path<String>,
) -> Result<Json<Vec<UserListItemDto>>, ApiError> {
    let res = state.user_service.get_all_users(&id).await?;
    Ok(Json(res))
}

pub async fn set_avatar(
    State(state): State<UserState>,
    Path(id): Path<String>,
    Json(payload): Json<SetAvatarDto>,
) -> Result<Json<SetAvatarResponseDto>, ApiError> {
    let res = state.user_service.set_avatar(&id, payload).await?;
    Ok(Json(res))
}

pub async fn logout(
    State(state): State<UserState>,
    Path(id): Path<String>,
) -> StatusCode {
    state.online_users.write().await.remove(&id);
    StatusCode::OK
}
