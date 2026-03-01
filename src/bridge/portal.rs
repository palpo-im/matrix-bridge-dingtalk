use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::info;

#[derive(Debug, Clone, PartialEq)]
pub enum RoomType {
    Direct,
    Group,
}

#[derive(Debug, Clone)]
pub struct BridgePortal {
    pub matrix_room_id: String,
    pub dingtalk_conversation_id: String,
    pub room_type: RoomType,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
}

impl BridgePortal {
    pub fn new(
        matrix_room_id: String,
        dingtalk_conversation_id: String,
        room_type: RoomType,
    ) -> Self {
        Self {
            matrix_room_id,
            dingtalk_conversation_id,
            room_type,
            name: None,
            avatar_url: None,
        }
    }

    pub async fn sync_info(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}

#[derive(Clone)]
pub struct PortalManager {
    portals_by_matrix: Arc<RwLock<HashMap<String, BridgePortal>>>,
    portals_by_dingtalk: Arc<RwLock<HashMap<String, String>>>,
}

impl PortalManager {
    pub fn new() -> Self {
        Self {
            portals_by_matrix: Arc::new(RwLock::new(HashMap::new())),
            portals_by_dingtalk: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn get_by_matrix_room(&self, room_id: &str) -> Option<BridgePortal> {
        let guard = self.portals_by_matrix.read().await;
        guard.get(room_id).cloned()
    }

    pub async fn get_matrix_room(&self, conversation_id: &str) -> Option<String> {
        let guard = self.portals_by_dingtalk.read().await;
        guard.get(conversation_id).cloned()
    }

    pub async fn add_portal(&self, portal: BridgePortal) {
        let mut matrix_guard = self.portals_by_matrix.write().await;
        let mut dingtalk_guard = self.portals_by_dingtalk.write().await;

        info!(
            "Adding portal: {} <-> {}",
            portal.matrix_room_id, portal.dingtalk_conversation_id
        );

        dingtalk_guard.insert(
            portal.dingtalk_conversation_id.clone(),
            portal.matrix_room_id.clone(),
        );
        matrix_guard.insert(portal.matrix_room_id.clone(), portal);
    }

    pub async fn remove_portal(&self, matrix_room_id: &str) {
        let mut matrix_guard = self.portals_by_matrix.write().await;
        let mut dingtalk_guard = self.portals_by_dingtalk.write().await;

        if let Some(portal) = matrix_guard.remove(matrix_room_id) {
            info!(
                "Removing portal: {} <-> {}",
                portal.matrix_room_id, portal.dingtalk_conversation_id
            );
            dingtalk_guard.remove(&portal.dingtalk_conversation_id);
        }
    }
}

impl Default for PortalManager {
    fn default() -> Self {
        Self::new()
    }
}
