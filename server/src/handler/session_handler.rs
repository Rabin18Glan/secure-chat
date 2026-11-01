use crate::dto::session_dto::{CreateSessionDto, SessionInfoDto};
use crate::error::api_error::ApiError;
use crate::error::user_error::UserError;
use crate::state::auth_state::{require_actor, AuthUser};
use crate::state::session_state::SessionState;
use axum::Extension;
use axum::{
    extract::{Path, State},
    Json,
};

pub async fn create_session(
    Extension(user): Extension<AuthUser>,
    State(state): State<SessionState>,
    Json(payload): Json<CreateSessionDto>,
) -> Result<Json<SessionInfoDto>, ApiError> {
    require_actor(&user, &payload.owner_id)?;
    let session = state.session_service.create_session(payload).await?;
    Ok(Json(session))
}

pub async fn get_session(
    State(state): State<SessionState>,
    Path(session_id): Path<String>,
) -> Result<Json<SessionInfoDto>, ApiError> {
    let session = state
        .session_service
        .get_session(&session_id)
        .await?
        .ok_or(UserError::UserNotFound)?;
    Ok(Json(session))
}
