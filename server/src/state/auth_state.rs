use axum::{
    extract::{Request, State},
    http::{header, HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::RwLock;
use uuid::Uuid;

const SESSION_TTL: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Clone, Default)]
pub struct AuthStore(Arc<RwLock<HashMap<String, (String, Instant)>>>);

#[derive(Clone)]
pub struct AuthUser(pub String);

impl AuthStore {
    pub async fn issue(&self, user_id: &str) -> String {
        let mut sessions = self.0.write().await;
        sessions.retain(|_, (_, expiry)| *expiry > Instant::now());
        let token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
        sessions.insert(
            token.clone(),
            (user_id.to_owned(), Instant::now() + SESSION_TTL),
        );
        token
    }

    pub async fn authenticate(&self, headers: &HeaderMap) -> Option<AuthUser> {
        let token = session_token(headers)?;
        self.0
            .read()
            .await
            .get(token)
            .filter(|(_, expiry)| *expiry > Instant::now())
            .map(|(id, _)| AuthUser(id.clone()))
    }

    pub async fn revoke(&self, headers: &HeaderMap) {
        if let Some(token) = session_token(headers) {
            self.0.write().await.remove(token);
        }
    }
}

fn session_token(headers: &HeaderMap) -> Option<&str> {
    if let Some(token) = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
    {
        return Some(token);
    }
    headers
        .get(header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .find_map(|cookie| cookie.trim().strip_prefix("secure_chat_session="))
}

pub fn session_cookie(token: &str, headers: &HeaderMap, clear: bool) -> String {
    let secure = std::env::var("COOKIE_SECURE").is_ok_and(|v| v == "true")
        || headers
            .get("x-forwarded-proto")
            .is_some_and(|v| v == "https");
    format!(
        "secure_chat_session={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}{}",
        if clear { 0 } else { 86400 },
        if secure { "; Secure" } else { "" }
    )
}

pub async fn authenticate(
    State(store): State<AuthStore>,
    mut request: Request,
    next: Next,
) -> Response {
    let path = request.uri().path();
    let public = matches!(
        path,
        "/ping"
            | "/api/auth/register"
            | "/api/auth/login"
            | "/api/auth/google"
            | "/api/auth/google/config"
            | "/api/auth/google/url"
    );
    // Browser cookie requests must originate from this host or an explicit allowed origin.
    if let Some(origin) = request
        .headers()
        .get(header::ORIGIN)
        .and_then(|v| v.to_str().ok())
    {
        let same_host = reqwest::Url::parse(origin).ok().is_some_and(|url| {
            let authority = match url.port() {
                Some(port) => format!("{}:{port}", url.host_str().unwrap_or_default()),
                None => url.host_str().unwrap_or_default().to_owned(),
            };
            request
                .headers()
                .get(header::HOST)
                .and_then(|v| v.to_str().ok())
                == Some(authority.as_str())
        });
        let allowed = crate::config::parameter::get_or(
            "ALLOWED_ORIGINS",
            "http://localhost:5173,http://localhost:3000",
        )
        .split(',')
        .any(|value| value.trim() == origin);
        if !same_host && !allowed {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({"status": false, "msg": "Origin is not allowed"})),
            )
                .into_response();
        }
    }
    let user = store.authenticate(request.headers()).await;
    if !public && request.method() != axum::http::Method::OPTIONS && user.is_none() {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"status": false, "msg": "Please sign in again"})),
        )
            .into_response();
    }
    if let Some(user) = user {
        request.extensions_mut().insert(user);
    }
    request.extensions_mut().insert(store);
    next.run(request).await
}

pub fn require_actor(
    user: &AuthUser,
    claimed: &str,
) -> Result<(), crate::error::api_error::ApiError> {
    if user.0 == claimed {
        Ok(())
    } else {
        Err(crate::error::user_error::UserError::Forbidden.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn session_lifecycle_and_impersonation() {
        let store = AuthStore::default();
        let token = store.issue("alice").await;
        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            format!("other=1; secure_chat_session={token}")
                .parse()
                .unwrap(),
        );
        let user = store.authenticate(&headers).await.unwrap();
        assert_eq!(user.0, "alice");
        assert!(require_actor(&user, "mallory").is_err());
        store.revoke(&headers).await;
        assert!(store.authenticate(&headers).await.is_none());
    }
    #[tokio::test]
    async fn expired_and_unknown_sessions_are_rejected() {
        let store = AuthStore::default();
        store.0.write().await.insert(
            "expired".into(),
            ("alice".into(), Instant::now() - Duration::from_secs(1)),
        );
        for token in ["expired", "unknown"] {
            let mut headers = HeaderMap::new();
            headers.insert(
                header::AUTHORIZATION,
                format!("Bearer {token}").parse().unwrap(),
            );
            assert!(store.authenticate(&headers).await.is_none());
        }
    }
}
