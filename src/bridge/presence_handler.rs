use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;
use tracing::info;

#[derive(Debug, Clone, PartialEq)]
pub enum DingTalkPresenceStatus {
    Online,
    Offline,
    Busy,
    Idle,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct DingTalkPresence {
    pub user_id: String,
    pub status: DingTalkPresenceStatus,
    pub last_updated: Instant,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MatrixPresenceState {
    Online,
    Offline,
    Unavailable,
}

#[derive(Debug, Clone)]
pub enum MatrixPresenceTarget {
    User(String),
    Room(String),
}

pub struct PresenceHandler {
    cache: Arc<RwLock<HashMap<String, DingTalkPresence>>>,
    poll_interval: Duration,
}

impl PresenceHandler {
    pub fn new(cache_size: Option<usize>) -> Self {
        let _ = cache_size;
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            poll_interval: Duration::from_secs(60),
        }
    }

    pub async fn get_presence(&self, user_id: &str) -> Option<DingTalkPresence> {
        let guard = self.cache.read().await;
        guard.get(user_id).cloned()
    }

    pub async fn update_presence(&self, user_id: &str, status: DingTalkPresenceStatus) {
        let mut guard = self.cache.write().await;
        guard.insert(
            user_id.to_string(),
            DingTalkPresence {
                user_id: user_id.to_string(),
                status,
                last_updated: Instant::now(),
            },
        );
    }

    pub async fn sync_to_matrix(
        &self,
        _target: MatrixPresenceTarget,
        _presence: DingTalkPresence,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    pub async fn run_poll_loop(&self) {
        loop {
            tokio::time::sleep(self.poll_interval).await;
            info!("Presence poll loop running");
        }
    }
}
