use crate::handler::session_handler;
use crate::state::session_state::SessionState;
use axum::{
    routing::{get, post},
    Router,
};

pub fn routes() -> Router<SessionState> {
    Router::new()
        .route("/create", post(session_handler::create_session))
        .route("/{session_id}", get(session_handler::get_session))
}
