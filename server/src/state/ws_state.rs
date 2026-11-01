use crate::service::session_service::SessionService;
use axum::extract::ws::Message;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

#[derive(Clone)]
pub struct SessionParticipant {
    pub user_id: String,
    pub username: String,
    pub avatar: String,
    pub role: String, // "owner" | "admin" | "member"
    pub sender: mpsc::Sender<Message>,
}

#[derive(Clone)]
pub struct WaitingParticipant {
    pub user_id: String,
    pub username: String,
    pub avatar: String,
    pub sender: mpsc::Sender<Message>,
}

#[derive(Clone, Default)]
pub struct LiveSession {
    pub session_id: String,
    pub owner_id: String,
    pub admins: HashSet<String>,
    pub participants: HashMap<String, SessionParticipant>,
    pub waiting_room: HashMap<String, WaitingParticipant>,
}

pub type OnlineUsers = Arc<RwLock<HashMap<String, mpsc::Sender<Message>>>>;
pub type LiveSessions = Arc<RwLock<HashMap<String, LiveSession>>>;

#[derive(Clone)]
pub struct WsState {
    pub online_users: OnlineUsers,
    pub live_sessions: LiveSessions,
    pub session_service: SessionService,
}

impl WsState {
    pub fn new(online_users: OnlineUsers, session_service: SessionService) -> Self {
        Self {
            online_users,
            live_sessions: Arc::new(RwLock::new(HashMap::new())),
            session_service,
        }
    }
}
