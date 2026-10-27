use crate::config::database::Database;
use crate::service::message_service::MessageService;
use std::sync::Arc;

#[derive(Clone)]
pub struct MessageState {
    pub message_service: MessageService,
}

impl MessageState {
    pub fn new(db_conn: &Arc<Database>) -> Self {
        Self {
            message_service: MessageService::new(db_conn),
        }
    }
}
