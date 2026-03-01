use salvo::prelude::*;

#[handler]
pub async fn health_endpoint(res: &mut Response) {
    res.render(Json(serde_json::json!({
        "status": "ok",
        "timestamp": chrono::Utc::now().to_rfc3339()
    })));
}
