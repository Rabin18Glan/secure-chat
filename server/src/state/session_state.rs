use crate::config::database::Database;
use crate::service::session_service::SessionService;
use std::sync::Arc;

#[derive(Clone)]
pub struct SessionState {
    pub session_service: SessionService,
}

impl SessionState {
    pub fn new(db_conn: &Arc<Database>) -> Self {
        Self {
            session_service: SessionService::new(db_conn),
        }
    }
}
