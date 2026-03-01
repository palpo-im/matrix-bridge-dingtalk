use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use tokio::sync::RwLock;
use tracing::info;

#[derive(Debug, Clone, PartialEq)]
pub enum BridgeRequestStatus {
    Pending,
    Approved,
    Rejected,
    Expired,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ApprovalResponseStatus {
    Approved,
    Rejected,
}

#[derive(Debug, thiserror::Error)]
pub enum ProvisioningError {
    #[error("Room already bridged")]
    AlreadyBridged,
    #[error("Room not bridged")]
    NotBridged,
    #[error("Invalid request")]
    InvalidRequest,
    #[error("Permission denied")]
    PermissionDenied,
    #[error("Request expired")]
    Expired,
    #[error("Internal error: {0}")]
    Internal(String),
}

#[derive(Debug, Clone)]
pub struct PendingBridgeRequest {
    pub request_id: String,
    pub matrix_room_id: String,
    pub dingtalk_conversation_id: Option<String>,
    pub requested_by: String,
    pub status: BridgeRequestStatus,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

impl PendingBridgeRequest {
    pub fn new(
        matrix_room_id: String,
        dingtalk_conversation_id: Option<String>,
        requested_by: String,
        ttl: Duration,
    ) -> Self {
        let now = Utc::now();
        let expires_at = now + chrono::Duration::from_std(ttl).unwrap_or_else(|_| chrono::Duration::seconds(300));

        Self {
            request_id: uuid::Uuid::new_v4().to_string(),
            matrix_room_id,
            dingtalk_conversation_id,
            requested_by,
            status: BridgeRequestStatus::Pending,
            created_at: now,
            expires_at,
        }
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
}

pub struct ProvisioningCoordinator {
    webhook_timeout: Duration,
    pending_requests: Arc<RwLock<HashMap<String, PendingBridgeRequest>>>,
}

impl ProvisioningCoordinator {
    pub fn new(webhook_timeout_secs: u64) -> Self {
        Self {
            webhook_timeout: Duration::from_secs(webhook_timeout_secs),
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn create_bridge_request(
        &self,
        matrix_room_id: String,
        dingtalk_conversation_id: Option<String>,
        requested_by: String,
    ) -> Result<PendingBridgeRequest, ProvisioningError> {
        let request = PendingBridgeRequest::new(
            matrix_room_id,
            dingtalk_conversation_id,
            requested_by,
            self.webhook_timeout,
        );

        info!("Creating bridge request: {}", request.request_id);

        let mut guard = self.pending_requests.write().await;
        guard.insert(request.request_id.clone(), request.clone());

        Ok(request)
    }

    pub async fn get_request(&self, request_id: &str) -> Option<PendingBridgeRequest> {
        let guard = self.pending_requests.read().await;
        guard.get(request_id).cloned()
    }

    pub async fn approve_request(
        &self,
        request_id: &str,
    ) -> Result<PendingBridgeRequest, ProvisioningError> {
        let mut guard = self.pending_requests.write().await;

        let request = guard
            .get_mut(request_id)
            .ok_or(ProvisioningError::InvalidRequest)?;

        if request.is_expired() {
            request.status = BridgeRequestStatus::Expired;
            return Err(ProvisioningError::Expired);
        }

        request.status = BridgeRequestStatus::Approved;
        info!("Bridge request approved: {}", request_id);

        Ok(request.clone())
    }

    pub async fn reject_request(
        &self,
        request_id: &str,
    ) -> Result<PendingBridgeRequest, ProvisioningError> {
        let mut guard = self.pending_requests.write().await;

        let request = guard
            .get_mut(request_id)
            .ok_or(ProvisioningError::InvalidRequest)?;

        request.status = BridgeRequestStatus::Rejected;
        info!("Bridge request rejected: {}", request_id);

        Ok(request.clone())
    }

    pub async fn cleanup_expired(&self) {
        let mut guard = self.pending_requests.write().await;
        let expired: Vec<String> = guard
            .iter()
            .filter(|(_, r)| r.is_expired())
            .map(|(id, _)| id.clone())
            .collect();

        for id in expired {
            guard.remove(&id);
            info!("Removed expired request: {}", id);
        }
    }
}
