use crate::handler::ws_handler;
use crate::state::ws_state::WsState;
use axum::{routing::get, Router};

pub fn routes() -> Router<WsState> {
    Router::new()
        .route("/ws", get(ws_handler::ws_handler))
        .route("/ws/{id}", get(ws_handler::ws_handler))
        .route("/socket", get(ws_handler::ws_handler))
}
