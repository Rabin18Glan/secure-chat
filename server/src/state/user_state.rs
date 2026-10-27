use crate::config::database::Database;
use crate::service::user_service::UserService;
use axum::extract::ws::Message;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

pub type OnlineUsers = Arc<RwLock<HashMap<String, mpsc::UnboundedSender<Message>>>>;

#[derive(Clone)]
pub struct UserState {
    pub user_service: UserService,
    pub online_users: OnlineUsers,
}

impl UserState {
    pub fn new(db_conn: &Arc<Database>, online_users: OnlineUsers) -> Self {
        Self {
            user_service: UserService::new(db_conn),
            online_users,
        }
    }
}
