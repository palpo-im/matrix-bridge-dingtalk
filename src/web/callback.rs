use std::sync::Arc;

use salvo::prelude::*;
use serde_json::{Value, json};

use crate::bridge::DingTalkBridge;

fn extract_callback_token(req: &Request) -> Option<String> {
    req.query::<String>("token")
        .or_else(|| req.header::<String>("X-Dingtalk-Token"))
        .or_else(|| {
            req.header::<String>("Authorization")
                .map(|value| value.trim_start_matches("Bearer ").to_string())
        })
}

#[handler]
pub async fn dingtalk_callback(req: &mut Request, res: &mut Response, depot: &mut Depot) {
    let bridge = depot.obtain::<Arc<DingTalkBridge>>().cloned().unwrap();
    let token = extract_callback_token(req);

    if !bridge
        .dingtalk_service
        .validate_callback_token(token.as_deref())
    {
        res.status_code(StatusCode::UNAUTHORIZED);
        res.render(Json(json!({
            "errcode": 401,
            "errmsg": "unauthorized callback token"
        })));
        return;
    }

    let payload: Result<Value, _> = req.parse_json().await;
    let payload = match payload {
        Ok(value) => value,
        Err(err) => {
            res.status_code(StatusCode::BAD_REQUEST);
            res.render(Json(json!({
                "errcode": 400,
                "errmsg": format!("invalid json payload: {}", err)
            })));
            return;
        }
    };

    let body = payload.to_string();
    match bridge.dingtalk_service.handle_callback(&body).await {
        Ok(result) => res.render(Json(result)),
        Err(err) => {
            res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
            res.render(Json(json!({
                "errcode": 500,
                "errmsg": err.to_string()
            })));
        }
    }
}
