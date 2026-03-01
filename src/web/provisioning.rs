use std::sync::Arc;

use salvo::prelude::*;
use serde::{Deserialize, Serialize};

use crate::bridge::DingTalkBridge;

#[derive(Clone)]
pub struct ProvisioningApi {
    bridge: Arc<DingTalkBridge>,
    read_token: Option<String>,
    write_token: Option<String>,
    admin_token: Option<String>,
}

impl ProvisioningApi {
    pub fn new(
        bridge: Arc<DingTalkBridge>,
        read_token: Option<String>,
        write_token: Option<String>,
        admin_token: Option<String>,
    ) -> Self {
        Self {
            bridge,
            read_token,
            write_token,
            admin_token,
        }
    }

    pub fn bridge(&self) -> &Arc<DingTalkBridge> {
        &self.bridge
    }

    pub fn validate_read_token(&self, token: Option<&str>) -> bool {
        match (&self.read_token, token) {
            (Some(expected), Some(provided)) => expected == provided,
            (Some(_), None) => false,
            (None, _) => true,
        }
    }

    pub fn validate_write_token(&self, token: Option<&str>) -> bool {
        match (&self.write_token, &self.admin_token, token) {
            (Some(expected), _, Some(provided)) |
            (_, Some(expected), Some(provided)) => expected == provided,
            (Some(_), _, None) |
            (_, Some(_), None) => false,
            (None, None, _) => true,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BridgeStatus {
    pub started_at: String,
    pub uptime_secs: u64,
    pub version: String,
}

#[handler]
pub async fn get_status(req: &mut Request, res: &mut Response, depot: &mut Depot) {
    let api = depot.obtain::<ProvisioningApi>().cloned().unwrap();
    let token = req.header::<String>("Authorization").map(|s| {
        s.trim_start_matches("Bearer ").to_string()
    });

    if !api.validate_read_token(token.as_deref()) {
        res.status_code(StatusCode::UNAUTHORIZED);
        res.render(Json(serde_json::json!({
            "error": "Unauthorized"
        })));
        return;
    }

    let bridge = &api.bridge;
    let started_at = bridge.started_at();
    let uptime = started_at.elapsed().as_secs();

    let resp = BridgeStatus {
        started_at: chrono::Utc::now()
            .checked_sub_signed(chrono::Duration::seconds(uptime as i64))
            .unwrap()
            .to_rfc3339(),
        uptime_secs: uptime,
        version: env!("CARGO_PKG_VERSION").to_string(),
    };

    res.render(Json(resp));
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MappingInfo {
    pub matrix_room_id: String,
    pub dingtalk_conversation_id: String,
}

#[handler]
pub async fn mappings(req: &mut Request, res: &mut Response, depot: &mut Depot) {
    let api = depot.obtain::<ProvisioningApi>().cloned().unwrap();
    let token = req.header::<String>("Authorization").map(|s| {
        s.trim_start_matches("Bearer ").to_string()
    });

    if !api.validate_read_token(token.as_deref()) {
        res.status_code(StatusCode::UNAUTHORIZED);
        res.render(Json(serde_json::json!({
            "error": "Unauthorized"
        })));
        return;
    }

    res.render(Json(serde_json::json!({
        "mappings": [],
        "total": 0
    })));
}

#[handler]
pub async fn bridge_room(req: &mut Request, res: &mut Response, depot: &mut Depot) {
    let api = depot.obtain::<ProvisioningApi>().cloned().unwrap();
    let token = req.header::<String>("Authorization").map(|s| {
        s.trim_start_matches("Bearer ").to_string()
    });

    if !api.validate_write_token(token.as_deref()) {
        res.status_code(StatusCode::UNAUTHORIZED);
        res.render(Json(serde_json::json!({
            "error": "Unauthorized"
        })));
        return;
    }

    #[derive(Deserialize)]
    struct BridgeRequest {
        matrix_room_id: String,
        dingtalk_conversation_id: Option<String>,
    }

    let body: Result<BridgeRequest, _> = req.parse_json().await;

    match body {
        Ok(payload) => {
            res.render(Json(serde_json::json!({
                "status": "pending",
                "matrix_room_id": payload.matrix_room_id,
                "dingtalk_conversation_id": payload.dingtalk_conversation_id
            })));
        }
        Err(e) => {
            res.status_code(StatusCode::BAD_REQUEST);
            res.render(Json(serde_json::json!({
                "error": format!("Invalid request: {}", e)
            })));
        }
    }
}
