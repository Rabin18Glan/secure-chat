use crate::config::database::Database;
use crate::routes::{auth, messages, ws};
use crate::state::message_state::MessageState;
use crate::state::user_state::UserState;
use crate::state::ws_state::WsState;
use axum::extract::ws::Message;
use axum::http::{header, Method};
use axum::response::Json;
use axum::routing::get;
use axum::Router;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::trace::TraceLayer;

pub fn routes(db_conn: Arc<Database>) -> Router {
    let online_users = Arc::new(RwLock::new(HashMap::<String, mpsc::UnboundedSender<Message>>::new()));

    let user_state = UserState::new(&db_conn, online_users.clone());
    let message_state = MessageState::new(&db_conn);
    let ws_state = WsState::new(online_users);

    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::mirror_request())
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
        .nest("/messages", messages::routes().with_state(message_state));

    Router::new()
        .route(
            "/ping",
            get(|| async { Json(json!({ "msg": "Ping Successful" })) }),
        )
        .nest("/api", api_router)
        .merge(ws::routes().with_state(ws_state))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
}
