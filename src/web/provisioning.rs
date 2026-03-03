use std::sync::Arc;

use chrono::{Duration as ChronoDuration, Utc};
use salvo::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::bridge::DingTalkBridge;

#[derive(Clone)]
pub struct ProvisioningApi {
    bridge: Arc<DingTalkBridge>,
    read_token: Option<String>,
    write_token: Option<String>,
    delete_token: Option<String>,
    admin_token: Option<String>,
}

impl ProvisioningApi {
    pub fn new(
        bridge: Arc<DingTalkBridge>,
        read_token: Option<String>,
        write_token: Option<String>,
        delete_token: Option<String>,
        admin_token: Option<String>,
    ) -> Self {
        Self {
            bridge,
            read_token,
            write_token,
            delete_token,
            admin_token,
        }
    }

    pub fn bridge(&self) -> &Arc<DingTalkBridge> {
        &self.bridge
    }

    pub fn router(self) -> Router {
        Router::new()
            .hoop(affix_state::inject(self))
            .push(Router::with_path("status").get(get_status))
            .push(Router::with_path("mappings").get(mappings))
            .push(Router::with_path("bridge").post(bridge_room))
            .push(Router::with_path("unbridge").post(unbridge_room))
            .push(Router::with_path("dead-letters").get(list_dead_letters))
            .push(Router::with_path("dead-letters/<id>/replay").post(replay_dead_letter))
            .push(Router::with_path("dead-letters/replay").post(replay_dead_letters))
            .push(Router::with_path("dead-letters/cleanup").post(cleanup_dead_letters))
    }

    pub fn validate_read_token(&self, token: Option<&str>) -> bool {
        self.validate_token(
            token,
            [
                &self.read_token,
                &self.write_token,
                &self.delete_token,
                &self.admin_token,
            ],
        )
    }

    pub fn validate_write_token(&self, token: Option<&str>) -> bool {
        self.validate_token(
            token,
            [&self.write_token, &self.delete_token, &self.admin_token],
        )
    }

    pub fn validate_delete_token(&self, token: Option<&str>) -> bool {
        self.validate_token(token, [&self.delete_token, &self.admin_token])
    }

    fn validate_token<const N: usize>(
        &self,
        token: Option<&str>,
        expected: [&Option<String>; N],
    ) -> bool {
        let configured: Vec<&str> = expected
            .iter()
            .filter_map(|value| value.as_deref())
            .collect();

        if configured.is_empty() {
            return true;
        }

        let Some(token) = token else {
            return false;
        };

        configured.iter().any(|expected| *expected == token)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BridgeStatus {
    pub started_at: String,
    pub uptime_secs: u64,
    pub version: String,
    pub mappings: i64,
    pub dead_letters: DeadLetterSummary,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeadLetterSummary {
    pub pending: i64,
    pub failed: i64,
    pub replayed: i64,
    pub total: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MappingInfo {
    pub matrix_room_id: String,
    pub dingtalk_conversation_id: String,
}

#[derive(Debug, Deserialize)]
struct BridgeRequest {
    matrix_room_id: String,
    dingtalk_conversation_id: String,
    dingtalk_conversation_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UnbridgeRequest {
    matrix_room_id: String,
}

#[derive(Debug, Deserialize)]
struct ReplayBatchRequest {
    status: Option<String>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct CleanupRequest {
    status: Option<String>,
    older_than_hours: Option<i64>,
    limit: Option<i64>,
    dry_run: Option<bool>,
}

fn bearer_token(req: &Request) -> Option<String> {
    req.header::<String>("Authorization")
        .map(|value| value.trim_start_matches("Bearer ").to_string())
}

fn unauthorized(res: &mut Response) {
    res.status_code(StatusCode::UNAUTHORIZED);
    res.render(Json(json!({ "error": "Unauthorized" })));
}

#[handler]
pub async fn get_status(req: &mut Request, res: &mut Response, depot: &mut Depot) {
    let api = depot.obtain::<ProvisioningApi>().cloned().unwrap();
    let token = bearer_token(req);

    if !api.validate_read_token(token.as_deref()) {
        unauthorized(res);
        return;
    }

    let bridge = api.bridge();
    let started_at = bridge.started_at();
    let uptime = started_at.elapsed().as_secs();

    let mapping_count = match bridge.list_room_mappings(i64::MAX, 0).await {
        Ok(room_mappings) => room_mappings.len() as i64,
        Err(err) => {
            res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
            res.render(Json(json!({
                "error": format!("failed to list mappings: {}", err)
            })));
            return;
        }
    };

    let dead_letters = match bridge.dead_letter_counts().await {
        Ok((pending, failed, replayed, total)) => DeadLetterSummary {
            pending,
            failed,
            replayed,
            total,
        },
        Err(err) => {
            res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
            res.render(Json(json!({
                "error": format!("failed to query dead-letter counts: {}", err)
            })));
            return;
        }
    };

    let resp = BridgeStatus {
        started_at: Utc::now()
            .checked_sub_signed(ChronoDuration::seconds(uptime as i64))
            .unwrap_or_else(Utc::now)
            .to_rfc3339(),
        uptime_secs: uptime,
        version: env!("CARGO_PKG_VERSION").to_string(),
        mappings: mapping_count,
        dead_letters,
    };

    res.render(Json(resp));
}

#[handler]
pub async fn mappings(req: &mut Request, res: &mut Response, depot: &mut Depot) {
    let api = depot.obtain::<ProvisioningApi>().cloned().unwrap();
    let token = bearer_token(req);

    if !api.validate_read_token(token.as_deref()) {
        unauthorized(res);
        return;
    }

    let limit: i64 = req.query("limit").unwrap_or(100);
    let offset: i64 = req.query("offset").unwrap_or(0);

    match api.bridge().list_room_mappings(limit, offset).await {
        Ok(room_mappings) => {
            res.render(Json(json!({
                "mappings": room_mappings,
                "count": room_mappings.len(),
                "limit": limit.max(1),
                "offset": offset.max(0),
            })));
        }
        Err(err) => {
            res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
            res.render(Json(json!({
                "error": format!("failed to query mappings: {}", err)
            })));
        }
    }
}

#[handler]
pub async fn bridge_room(req: &mut Request, res: &mut Response, depot: &mut Depot) {
    let api = depot.obtain::<ProvisioningApi>().cloned().unwrap();
    let token = bearer_token(req);

    if !api.validate_write_token(token.as_deref()) {
        unauthorized(res);
        return;
    }

    let body: Result<BridgeRequest, _> = req.parse_json().await;

    match body {
        Ok(payload) => match api
            .bridge()
            .bridge_room(
                &payload.matrix_room_id,
                &payload.dingtalk_conversation_id,
                payload.dingtalk_conversation_name,
            )
            .await
        {
            Ok(mapping) => {
                res.status_code(StatusCode::CREATED);
                res.render(Json(json!({
                    "status": "bridged",
                    "mapping": mapping,
                })));
            }
            Err(err) => {
                res.status_code(StatusCode::BAD_REQUEST);
                res.render(Json(json!({
                    "error": err.to_string()
                })));
            }
        },
        Err(err) => {
            res.status_code(StatusCode::BAD_REQUEST);
            res.render(Json(json!({
                "error": format!("Invalid request: {}", err)
            })));
        }
    }
}

#[handler]
pub async fn unbridge_room(req: &mut Request, res: &mut Response, depot: &mut Depot) {
    let api = depot.obtain::<ProvisioningApi>().cloned().unwrap();
    let token = bearer_token(req);

    if !api.validate_write_token(token.as_deref()) {
        unauthorized(res);
        return;
    }

    let body: Result<UnbridgeRequest, _> = req.parse_json().await;
    let Ok(payload) = body else {
        res.status_code(StatusCode::BAD_REQUEST);
        res.render(Json(json!({ "error": "Invalid request" })));
        return;
    };

    match api.bridge().unbridge_room(&payload.matrix_room_id).await {
        Ok(true) => res.render(Json(json!({
            "status": "unbridged",
            "matrix_room_id": payload.matrix_room_id,
        }))),
        Ok(false) => {
            res.status_code(StatusCode::NOT_FOUND);
            res.render(Json(json!({ "error": "mapping not found" })));
        }
        Err(err) => {
            res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
            res.render(Json(json!({ "error": err.to_string() })));
        }
    }
}

#[handler]
pub async fn list_dead_letters(req: &mut Request, res: &mut Response, depot: &mut Depot) {
    let api = depot.obtain::<ProvisioningApi>().cloned().unwrap();
    let token = bearer_token(req);

    if !api.validate_read_token(token.as_deref()) {
        unauthorized(res);
        return;
    }

    let status = req.query::<String>("status");
    let limit = req.query::<i64>("limit").unwrap_or(100).max(1);

    match api
        .bridge()
        .list_dead_letters(status.as_deref(), limit)
        .await
    {
        Ok(dead_letters) => {
            res.render(Json(json!({
                "dead_letters": dead_letters,
                "count": dead_letters.len(),
                "status": status,
                "limit": limit,
            })));
        }
        Err(err) => {
            res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
            res.render(Json(json!({ "error": err.to_string() })));
        }
    }
}

#[handler]
pub async fn replay_dead_letter(req: &mut Request, res: &mut Response, depot: &mut Depot) {
    let api = depot.obtain::<ProvisioningApi>().cloned().unwrap();
    let token = bearer_token(req);

    if !api.validate_write_token(token.as_deref()) {
        unauthorized(res);
        return;
    }

    let Some(id) = req.param::<i64>("id") else {
        res.status_code(StatusCode::BAD_REQUEST);
        res.render(Json(json!({ "error": "invalid dead-letter id" })));
        return;
    };

    match api.bridge().replay_dead_letter(id).await {
        Ok(()) => {
            res.render(Json(json!({
                "status": "replayed",
                "id": id,
            })));
        }
        Err(err) => {
            res.status_code(StatusCode::BAD_REQUEST);
            res.render(Json(json!({ "error": err.to_string() })));
        }
    }
}

#[handler]
pub async fn replay_dead_letters(req: &mut Request, res: &mut Response, depot: &mut Depot) {
    let api = depot.obtain::<ProvisioningApi>().cloned().unwrap();
    let token = bearer_token(req);

    if !api.validate_write_token(token.as_deref()) {
        unauthorized(res);
        return;
    }

    let body: Result<ReplayBatchRequest, _> = req.parse_json().await;
    let body = body.unwrap_or(ReplayBatchRequest {
        status: Some("pending".to_string()),
        limit: Some(20),
    });

    let status = body.status.unwrap_or_else(|| "pending".to_string());
    let limit = body.limit.unwrap_or(20).max(1);

    match api.bridge().replay_dead_letters(&status, limit).await {
        Ok((replayed, errors)) => {
            res.render(Json(json!({
                "status": "ok",
                "replayed": replayed,
                "errors": errors,
                "requested_status": status,
                "limit": limit,
            })));
        }
        Err(err) => {
            res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
            res.render(Json(json!({ "error": err.to_string() })));
        }
    }
}

#[handler]
pub async fn cleanup_dead_letters(req: &mut Request, res: &mut Response, depot: &mut Depot) {
    let api = depot.obtain::<ProvisioningApi>().cloned().unwrap();
    let token = bearer_token(req);

    if !api.validate_delete_token(token.as_deref()) {
        unauthorized(res);
        return;
    }

    let body: Result<CleanupRequest, _> = req.parse_json().await;
    let body = body.unwrap_or(CleanupRequest {
        status: None,
        older_than_hours: None,
        limit: Some(200),
        dry_run: Some(false),
    });

    let status = body.status;
    let older_than_hours = body.older_than_hours;
    let limit = body.limit.unwrap_or(200).max(1);
    let dry_run = body.dry_run.unwrap_or(false);

    if dry_run {
        let cutoff = older_than_hours.map(|hours| Utc::now() - ChronoDuration::hours(hours.max(0)));
        match api
            .bridge()
            .list_dead_letters(status.as_deref(), limit)
            .await
        {
            Ok(dead_letters) => {
                let filtered: Vec<_> = dead_letters
                    .into_iter()
                    .filter(|item| {
                        if let Some(cutoff) = cutoff {
                            item.created_at < cutoff
                        } else {
                            true
                        }
                    })
                    .collect();
                let ids: Vec<i64> = filtered.iter().map(|item| item.id).collect();
                res.render(Json(json!({
                    "status": "dry_run",
                    "would_delete": ids.len(),
                    "sample_ids": ids,
                    "filter": {
                        "status": status,
                        "older_than_hours": older_than_hours,
                        "limit": limit,
                    }
                })));
            }
            Err(err) => {
                res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
                res.render(Json(json!({ "error": err.to_string() })));
            }
        }
        return;
    }

    match api
        .bridge()
        .cleanup_dead_letters(status.as_deref(), older_than_hours, limit)
        .await
    {
        Ok(deleted) => {
            res.render(Json(json!({
                "status": "cleaned",
                "deleted": deleted,
                "filter": {
                    "status": status,
                    "older_than_hours": older_than_hours,
                    "limit": limit,
                }
            })));
        }
        Err(err) => {
            res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
            res.render(Json(json!({ "error": err.to_string() })));
        }
    }
}
