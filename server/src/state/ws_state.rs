use axum::extract::ws::Message;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

pub type OnlineUsers = Arc<RwLock<HashMap<String, mpsc::UnboundedSender<Message>>>>;

#[derive(Clone)]
pub struct WsState {
    pub online_users: OnlineUsers,
}

impl WsState {
    pub fn new(online_users: OnlineUsers) -> Self {
        Self { online_users }
    }
}
