use crate::config::database::Database;
use crate::routes::{auth, messages, sessions, ws};
use crate::service::session_service::SessionService;
use crate::state::auth_state::{self, AuthStore};
use crate::state::message_state::MessageState;
use crate::state::session_state::SessionState;
use crate::state::user_state::UserState;
use crate::state::ws_state::WsState;
use axum::extract::ws::Message;
use axum::http::{header, Method};
use axum::middleware;
use axum::response::Json;
use axum::routing::get;
use axum::Router;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

pub fn routes(db_conn: Arc<Database>) -> Router {
    let online_users = Arc::new(RwLock::new(HashMap::<String, mpsc::Sender<Message>>::new()));

    let session_service = SessionService::new(&db_conn);

    let user_state = UserState::new(&db_conn, online_users.clone());
    let message_state = MessageState::new(&db_conn);
    let session_state = SessionState::new(&db_conn);
    let ws_state = WsState::new(online_users, session_service);

    let origins: Vec<axum::http::HeaderValue> = crate::config::parameter::get_or(
        "ALLOWED_ORIGINS",
        "http://localhost:5173,http://localhost:3000",
    )
    .split(',')
    .filter_map(|origin| origin.trim().parse().ok())
    .collect();
    let cors = CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            header::AUTHORIZATION,
            header::ACCEPT,
            header::CONTENT_TYPE,
            header::UPGRADE,
            header::CONNECTION,
        ])
        .allow_credentials(true);

    let api_router = Router::new()
        .nest("/auth", auth::routes().with_state(user_state))
        .nest("/messages", messages::routes().with_state(message_state))
        .nest("/sessions", sessions::routes().with_state(session_state));

    Router::new()
        .route(
            "/ping",
            get(|| async { Json(json!({ "msg": "Ping Successful" })) }),
        )
        .nest("/api", api_router)
        .merge(ws::routes().with_state(ws_state))
        .layer(middleware::from_fn_with_state(
            AuthStore::default(),
            auth_state::authenticate,
        ))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
}
