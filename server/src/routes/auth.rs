use crate::handler::auth_handler;
use crate::state::user_state::UserState;
use axum::{
    routing::{get, post},
    Router,
};

pub fn routes() -> Router<UserState> {
    Router::new()
        .route("/register", post(auth_handler::register))
        .route("/login", post(auth_handler::login))
        .route("/allusers/{id}", get(auth_handler::get_all_users))
        .route("/setavatar/{id}", post(auth_handler::set_avatar))
        .route("/logout/{id}", get(auth_handler::logout))
}
