use crate::config::database::Database;
use crate::dto::session_dto::{CreateSessionDto, SessionInfoDto};
use crate::entity::session::Session;
use crate::error::api_error::ApiError;
use crate::error::user_error::UserError;
use crate::repository::session_repository::{SessionRepository, SessionRepositoryTrait};
use bson::oid::ObjectId;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct SessionService {
    session_repo: SessionRepository,
}

impl SessionService {
    pub fn new(db_conn: &Arc<Database>) -> Self {
        Self {
            session_repo: SessionRepository::new(db_conn),
        }
    }

    pub async fn create_session(
        &self,
        payload: CreateSessionDto,
    ) -> Result<SessionInfoDto, ApiError> {
        let owner_oid =
            ObjectId::parse_str(&payload.owner_id).map_err(|_| UserError::InvalidUserId)?;

        let random_part = Uuid::new_v4().to_string();
        let short_code = &random_part[..8];
        let session_id = format!("meet-{}", short_code);

        let session_name = payload
            .name
            .filter(|n| !n.trim().is_empty())
            .unwrap_or_else(|| format!("Room-{}", short_code));

        let new_session = Session {
            id: None,
            session_id,
            name: session_name,
            owner_id: owner_oid,
            admins: vec![owner_oid],
            members: vec![owner_oid],
            created_at: None,
            updated_at: None,
        };

        let created = self.session_repo.create(new_session).await?;
        Ok(SessionInfoDto::from(created))
    }

    pub async fn get_session(&self, session_id: &str) -> Result<Option<SessionInfoDto>, ApiError> {
        let session = self.session_repo.find_by_session_id(session_id).await?;
        Ok(session.map(SessionInfoDto::from))
    }

    pub async fn promote_admin(
        &self,
        session_id: &str,
        requester_id: &str,
        target_user_id: &str,
    ) -> Result<(), ApiError> {
        let session = self
            .session_repo
            .find_by_session_id(session_id)
            .await?
            .ok_or(UserError::UserNotFound)?;

        let req_oid = ObjectId::parse_str(requester_id).map_err(|_| UserError::InvalidUserId)?;
        let target_oid =
            ObjectId::parse_str(target_user_id).map_err(|_| UserError::InvalidUserId)?;

        // Only owner or existing admin can promote someone
        let is_admin = session.owner_id == req_oid || session.admins.contains(&req_oid);
        if !is_admin {
            return Err(UserError::IncorrectCredentials.into());
        }

        self.session_repo.add_admin(session_id, target_oid).await?;
        Ok(())
    }

    pub async fn demote_admin(
        &self,
        session_id: &str,
        requester_id: &str,
        target_user_id: &str,
    ) -> Result<(), ApiError> {
        let session = self
            .session_repo
            .find_by_session_id(session_id)
            .await?
            .ok_or(UserError::UserNotFound)?;

        let req_oid = ObjectId::parse_str(requester_id).map_err(|_| UserError::InvalidUserId)?;
        let target_oid =
            ObjectId::parse_str(target_user_id).map_err(|_| UserError::InvalidUserId)?;

        // Super Admin (Owner) cannot be demoted
        if session.owner_id == target_oid {
            return Err(
                UserError::SomethingWentWrong("Super Admin cannot be demoted".to_string()).into(),
            );
        }

        // Only Owner or Admin can demote
        let is_admin = session.owner_id == req_oid || session.admins.contains(&req_oid);
        if !is_admin {
            return Err(UserError::IncorrectCredentials.into());
        }

        self.session_repo
            .remove_admin(session_id, target_oid)
            .await?;
        Ok(())
    }

    pub async fn remove_member(
        &self,
        session_id: &str,
        requester_id: &str,
        target_user_id: &str,
    ) -> Result<(), ApiError> {
        let session = self
            .session_repo
            .find_by_session_id(session_id)
            .await?
            .ok_or(UserError::UserNotFound)?;

        let req_oid = ObjectId::parse_str(requester_id).map_err(|_| UserError::InvalidUserId)?;
        let target_oid =
            ObjectId::parse_str(target_user_id).map_err(|_| UserError::InvalidUserId)?;

        // Cannot remove owner
        if session.owner_id == target_oid {
            return Err(
                UserError::SomethingWentWrong("Super Admin cannot be removed".to_string()).into(),
            );
        }

        let is_admin = session.owner_id == req_oid || session.admins.contains(&req_oid);
        if !is_admin {
            return Err(UserError::IncorrectCredentials.into());
        }

        self.session_repo
            .remove_member(session_id, target_oid)
            .await?;
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn terminate_session(
        &self,
        session_id: &str,
        requester_id: &str,
    ) -> Result<(), ApiError> {
        let session = self
            .session_repo
            .find_by_session_id(session_id)
            .await?
            .ok_or(UserError::UserNotFound)?;

        let req_oid = ObjectId::parse_str(requester_id).map_err(|_| UserError::InvalidUserId)?;
        let is_admin = session.owner_id == req_oid || session.admins.contains(&req_oid);
        if !is_admin {
            return Err(UserError::IncorrectCredentials.into());
        }

        self.session_repo.delete_session(session_id).await?;
        Ok(())
    }

    pub async fn cleanup_empty_session(&self, session_id: &str) -> Result<(), ApiError> {
        self.session_repo.delete_session(session_id).await?;
        Ok(())
    }
}
