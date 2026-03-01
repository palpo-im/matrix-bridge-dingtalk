use std::time::Duration;

#[derive(Debug, Clone)]
pub struct UserSyncPolicy {
    pub sync_interval: Duration,
    pub stale_ttl: chrono::Duration,
}

impl UserSyncPolicy {
    pub fn new(sync_interval: Duration, stale_ttl: chrono::Duration) -> Self {
        Self {
            sync_interval,
            stale_ttl,
        }
    }
}

impl Default for UserSyncPolicy {
    fn default() -> Self {
        Self {
            sync_interval: Duration::from_secs(300),
            stale_ttl: chrono::Duration::hours(24),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BridgeUser {
    pub dingtalk_user_id: String,
    pub matrix_mxid: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub last_synced_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl BridgeUser {
    pub fn new(dingtalk_user_id: String, matrix_mxid: String) -> Self {
        Self {
            dingtalk_user_id,
            matrix_mxid,
            display_name: None,
            avatar_url: None,
            last_synced_at: None,
        }
    }

    pub fn needs_sync(&self, policy: &UserSyncPolicy) -> bool {
        match self.last_synced_at {
            None => true,
            Some(last_sync) => {
                let elapsed = chrono::Utc::now() - last_sync;
                elapsed > policy.stale_ttl
            }
        }
    }

    pub fn mark_synced(&mut self) {
        self.last_synced_at = Some(chrono::Utc::now());
    }

    pub async fn sync_from_dingtalk(&mut self) -> anyhow::Result<()> {
        self.mark_synced();
        Ok(())
    }
}
