use crate::dto::user_dto::{
    AuthResponseDto, GoogleAuthDto, SetAvatarDto, SetAvatarResponseDto, UserListItemDto,
    UserLoginDto, UserRegisterDto,
};
use crate::error::api_error::ApiError;
use crate::state::auth_state::{require_actor, session_cookie, AuthStore, AuthUser};
use crate::state::user_state::UserState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    http::{header, HeaderMap},
    Extension, Json,
};

pub async fn register(
    State(state): State<UserState>,
    Extension(store): Extension<AuthStore>,
    headers: HeaderMap,
    Json(payload): Json<UserRegisterDto>,
) -> Result<(HeaderMap, Json<AuthResponseDto>), ApiError> {
    let res = state.user_service.register(payload).await?;
    let mut response_headers = HeaderMap::new();
    if let Some(user) = res.user.as_ref().filter(|_| res.status) {
        let token = store.issue(&user.id).await;
        response_headers.insert(
            header::SET_COOKIE,
            session_cookie(&token, &headers, false)
                .parse()
                .expect("valid session cookie"),
        );
    }
    Ok((response_headers, Json(res)))
}

pub async fn login(
    State(state): State<UserState>,
    Extension(store): Extension<AuthStore>,
    headers: HeaderMap,
    Json(payload): Json<UserLoginDto>,
) -> Result<(HeaderMap, Json<AuthResponseDto>), ApiError> {
    let res = state.user_service.login(payload).await?;
    let mut response_headers = HeaderMap::new();
    if let Some(user) = res.user.as_ref().filter(|_| res.status) {
        let token = store.issue(&user.id).await;
        response_headers.insert(
            header::SET_COOKIE,
            session_cookie(&token, &headers, false)
                .parse()
                .expect("valid session cookie"),
        );
    }
    Ok((response_headers, Json(res)))
}

pub async fn google_auth(
    State(state): State<UserState>,
    Extension(store): Extension<AuthStore>,
    headers: HeaderMap,
    Json(payload): Json<GoogleAuthDto>,
) -> Result<(HeaderMap, Json<AuthResponseDto>), ApiError> {
    if payload.code.is_some() {
        let cookie_state = headers
            .get(header::COOKIE)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| {
                v.split(';')
                    .find_map(|part| part.trim().strip_prefix("secure_chat_oauth="))
            });
        if cookie_state.is_none() || cookie_state != payload.state.as_deref() {
            return Err(crate::error::user_error::UserError::GoogleAuthFailed(
                "Invalid OAuth state. Please start sign-in again.".into(),
            )
            .into());
        }
    }
    let res = state.user_service.google_auth(payload).await?;
    let mut response_headers = HeaderMap::new();
    if let Some(user) = res.user.as_ref().filter(|_| res.status) {
        let token = store.issue(&user.id).await;
        response_headers.insert(
            header::SET_COOKIE,
            session_cookie(&token, &headers, false)
                .parse()
                .expect("valid session cookie"),
        );
    }
    Ok((response_headers, Json(res)))
}

pub async fn google_config(State(state): State<UserState>) -> Json<serde_json::Value> {
    Json(state.user_service.get_google_config())
}

pub async fn google_url(
    State(state): State<UserState>,
    headers: HeaderMap,
) -> (HeaderMap, Json<serde_json::Value>) {
    let mut body = state.user_service.get_google_auth_url();
    let mut response_headers = HeaderMap::new();
    if let Some(url) = body
        .get("url")
        .and_then(|v| v.as_str())
        .and_then(|v| reqwest::Url::parse(v).ok())
    {
        let nonce = uuid::Uuid::new_v4().simple().to_string();
        let mut url = url;
        url.query_pairs_mut().append_pair("state", &nonce);
        body["url"] = serde_json::json!(url.as_str());
        let secure = session_cookie("", &headers, false).contains("; Secure");
        response_headers.insert(
            header::SET_COOKIE,
            format!(
                "secure_chat_oauth={nonce}; Path=/; HttpOnly; SameSite=Lax; Max-Age=600{}",
                if secure { "; Secure" } else { "" }
            )
            .parse()
            .expect("valid OAuth cookie"),
        );
    }
    (response_headers, Json(body))
}

pub async fn get_all_users(
    Extension(user): Extension<AuthUser>,
    State(state): State<UserState>,
    Path(id): Path<String>,
) -> Result<Json<Vec<UserListItemDto>>, ApiError> {
    require_actor(&user, &id)?;
    let res = state.user_service.get_all_users(&id).await?;
    Ok(Json(res))
}

pub async fn set_avatar(
    Extension(user): Extension<AuthUser>,
    State(state): State<UserState>,
    Path(id): Path<String>,
    Json(payload): Json<SetAvatarDto>,
) -> Result<Json<SetAvatarResponseDto>, ApiError> {
    require_actor(&user, &id)?;
    let res = state.user_service.set_avatar(&id, payload).await?;
    Ok(Json(res))
}

pub async fn logout(
    State(state): State<UserState>,
    Extension(user): Extension<AuthUser>,
    Extension(store): Extension<AuthStore>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<(HeaderMap, StatusCode), ApiError> {
    require_actor(&user, &id)?;
    store.revoke(&headers).await;
    state.online_users.write().await.remove(&id);
    let mut response_headers = HeaderMap::new();
    response_headers.insert(
        header::SET_COOKIE,
        session_cookie("", &headers, true)
            .parse()
            .expect("valid cookie"),
    );
    Ok((response_headers, StatusCode::OK))
}

pub async fn me(
    State(state): State<UserState>,
    Extension(user): Extension<AuthUser>,
) -> Result<Json<crate::dto::user_dto::UserDto>, ApiError> {
    Ok(Json(state.user_service.get_user(&user.0).await?))
}
