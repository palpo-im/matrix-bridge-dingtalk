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
    println!("[DEBUG] ==================================");
    println!("[DEBUG] CALLBACK RECEIVED - DingTalk callback handler invoked");
    println!("[DEBUG]   path: {}", req.uri().path());
    println!("[DEBUG]   method: {}", req.method());
    eprintln!("[DEBUG] CALLBACK RECEIVED - DingTalk callback handler invoked");
    
    let bridge = depot.obtain::<Arc<DingTalkBridge>>().cloned().unwrap();
    let token = extract_callback_token(req);

    println!("[DEBUG]   token_provided: {}", token.is_some());
    
    if !bridge
        .dingtalk_service
        .validate_callback_token(token.as_deref())
    {
        println!("[DEBUG]   token_validation: FAILED");
        res.status_code(StatusCode::UNAUTHORIZED);
        res.render(Json(json!({
            "errcode": 401,
            "errmsg": "unauthorized callback token"
        })));
        return;
    }

    println!("[DEBUG]   token_validation: PASSED");
    
    let payload: Result<Value, _> = req.parse_json().await;
    let payload = match payload {
        Ok(value) => {
            println!("[DEBUG]   payload_parsing: SUCCESS");
            value
        },
        Err(err) => {
            println!("[DEBUG]   payload_parsing: FAILED - {}", err);
            res.status_code(StatusCode::BAD_REQUEST);
            res.render(Json(json!({
                "errcode": 400,
                "errmsg": format!("invalid json payload: {}", err)
            })));
            return;
        }
    };

    let body = payload.to_string();
    println!("[DEBUG]   payload_length: {} bytes", body.len());
    println!("[DEBUG]   payload_preview: {}", if body.len() > 500 { &body[..500] } else { &body });
    
    match bridge.dingtalk_service.handle_callback(&body).await {
        Ok(result) => {
            println!("[DEBUG]   callback_result: SUCCESS");
            println!("[DEBUG] ==================================");
            res.render(Json(result))
        },
        Err(err) => {
            println!("[DEBUG]   callback_result: ERROR - {}", err);
            println!("[DEBUG] ==================================");
            res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
            res.render(Json(json!({
                "errcode": 500,
                "errmsg": err.to_string()
            })));
        }
    }
}
