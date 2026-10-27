use crate::handler::message_handler;
use crate::state::message_state::MessageState;
use axum::{routing::post, Router};

pub fn routes() -> Router<MessageState> {
    Router::new()
        .route("/addmsg", post(message_handler::add_message))
        .route("/addmsg/", post(message_handler::add_message))
        .route("/getmsg", post(message_handler::get_messages))
        .route("/getmsg/", post(message_handler::get_messages))
}
