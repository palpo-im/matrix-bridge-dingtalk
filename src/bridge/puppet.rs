use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct BridgePuppet {
    pub dingtalk_user_id: String,
    pub matrix_mxid: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

impl BridgePuppet {
    pub fn new(dingtalk_user_id: String, matrix_mxid: String) -> Self {
        Self {
            dingtalk_user_id,
            matrix_mxid,
            display_name: None,
            avatar_url: None,
        }
    }

    pub async fn sync_profile(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}

pub struct PuppetManager {
    puppets: Vec<Arc<BridgePuppet>>,
}

impl PuppetManager {
    pub fn new() -> Self {
        Self {
            puppets: Vec::new(),
        }
    }

    pub fn add_puppet(&mut self, puppet: BridgePuppet) {
        self.puppets.push(Arc::new(puppet));
    }

    pub fn get_puppet(&self, dingtalk_user_id: &str) -> Option<Arc<BridgePuppet>> {
        self.puppets
            .iter()
            .find(|p| p.dingtalk_user_id == dingtalk_user_id)
            .cloned()
    }
}

impl Default for PuppetManager {
    fn default() -> Self {
        Self::new()
    }
}
