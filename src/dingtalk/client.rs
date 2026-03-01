use anyhow::{Context, Result};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use hmac::{Hmac, Mac};
use reqwest::Client;
use serde::de::DeserializeOwned;
use sha2::Sha256;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{debug, warn};

use super::types::*;

type HmacSha256 = Hmac<Sha256>;

const DEFAULT_TIMEOUT_SECS: u64 = 30;
const DEFAULT_MAX_RETRIES: u32 = 3;
const DEFAULT_RETRY_BASE_MS: u64 = 250;

#[derive(Clone)]
pub struct DingTalkClient {
    webhook_url: String,
    access_token: String,
    secret: Option<String>,
    client: Client,
    max_retries: u32,
    retry_base_ms: u64,
}

impl DingTalkClient {
    pub fn new(webhook_url: String, access_token: String, secret: Option<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            webhook_url,
            access_token,
            secret,
            client,
            max_retries: DEFAULT_MAX_RETRIES,
            retry_base_ms: DEFAULT_RETRY_BASE_MS,
        }
    }

    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    pub fn with_retry_base_ms(mut self, retry_base_ms: u64) -> Self {
        self.retry_base_ms = retry_base_ms;
        self
    }

    fn build_signed_url(&self) -> String {
        let mut url = format!("{}?access_token={}", self.webhook_url, self.access_token);

        if let Some(secret) = &self.secret {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis();

            let string_to_sign = format!("{}\n{}", timestamp, secret);

            let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
                .expect("HMAC can take key of any size");
            mac.update(string_to_sign.as_bytes());
            let result = mac.finalize();
            let signature = STANDARD.encode(result.into_bytes());

            url = format!("{}&timestamp={}&sign={}", url, timestamp, 
                urlencoding::encode(&signature));
        }

        url
    }

    pub async fn send_text(
        &self,
        content: &str,
        at_mobiles: Option<Vec<String>>,
        at_user_ids: Option<Vec<String>>,
        is_at_all: bool,
    ) -> Result<DingTalkResponse> {
        let text_msg = DingTalkTextMessage {
            content: content.to_string(),
            at_mobiles,
            at_user_ids,
            is_at_all: if is_at_all { Some(true) } else { None },
        };

        let payload = serde_json::json!({
            "msgtype": "text",
            "text": {
                "content": text_msg.content
            },
            "at": {
                "atMobiles": text_msg.at_mobiles.unwrap_or_default(),
                "atUserIds": text_msg.at_user_ids.unwrap_or_default(),
                "isAtAll": text_msg.is_at_all.unwrap_or(false)
            }
        });

        self.send_message(payload).await
    }

    pub async fn send_markdown(
        &self,
        title: &str,
        text: &str,
        at_mobiles: Option<Vec<String>>,
        at_user_ids: Option<Vec<String>>,
        is_at_all: bool,
    ) -> Result<DingTalkResponse> {
        let payload = serde_json::json!({
            "msgtype": "markdown",
            "markdown": {
                "title": title,
                "text": text
            },
            "at": {
                "atMobiles": at_mobiles.unwrap_or_default(),
                "atUserIds": at_user_ids.unwrap_or_default(),
                "isAtAll": is_at_all
            }
        });

        self.send_message(payload).await
    }

    pub async fn send_link(
        &self,
        title: &str,
        text: &str,
        message_url: &str,
        pic_url: Option<&str>,
    ) -> Result<DingTalkResponse> {
        let mut link = serde_json::Map::new();
        link.insert("title".to_string(), serde_json::json!(title));
        link.insert("text".to_string(), serde_json::json!(text));
        link.insert("messageUrl".to_string(), serde_json::json!(message_url));
        if let Some(pic) = pic_url {
            link.insert("picUrl".to_string(), serde_json::json!(pic));
        }

        let payload = serde_json::json!({
            "msgtype": "link",
            "link": link
        });

        self.send_message(payload).await
    }

    pub async fn send_action_card(
        &self,
        title: &str,
        text: &str,
        single_title: Option<&str>,
        single_url: Option<&str>,
        buttons: Option<Vec<(String, String)>>,
    ) -> Result<DingTalkResponse> {
        let mut action_card = serde_json::Map::new();
        action_card.insert("title".to_string(), serde_json::json!(title));
        action_card.insert("text".to_string(), serde_json::json!(text));

        if let (Some(st), Some(su)) = (single_title, single_url) {
            action_card.insert("singleTitle".to_string(), serde_json::json!(st));
            action_card.insert("singleURL".to_string(), serde_json::json!(su));
        } else if let Some(btns) = buttons {
            let btn_json_list: Vec<serde_json::Value> = btns
                .iter()
                .map(|(t, u)| {
                    serde_json::json!({
                        "title": t,
                        "actionURL": u
                    })
                })
                .collect();
            action_card.insert("btns".to_string(), serde_json::json!(btn_json_list));
        }

        let payload = serde_json::json!({
            "msgtype": "actionCard",
            "actionCard": action_card
        });

        self.send_message(payload).await
    }

    pub async fn send_feed_card(
        &self,
        links: Vec<(String, String, String)>,
    ) -> Result<DingTalkResponse> {
        let links_json: Vec<serde_json::Value> = links
            .iter()
            .map(|(title, message_url, pic_url)| {
                serde_json::json!({
                    "title": title,
                    "messageURL": message_url,
                    "picURL": pic_url
                })
            })
            .collect();

        let payload = serde_json::json!({
            "msgtype": "feedCard",
            "feedCard": {
                "links": links_json
            }
        });

        self.send_message(payload).await
    }

    async fn send_message(&self, payload: serde_json::Value) -> Result<DingTalkResponse> {
        let url = self.build_signed_url();

        for attempt in 0..=self.max_retries {
            match self.execute_request(&url, &payload).await {
                Ok(response) => {
                    if response.is_success() {
                        return Ok(response);
                    }
                    if response.errcode == 45009 || response.errcode == 45010 {
                        warn!(
                            "Rate limited by DingTalk, attempt {}/{}",
                            attempt, self.max_retries
                        );
                        if attempt < self.max_retries {
                            let delay = self.retry_base_ms * (1 << attempt);
                            tokio::time::sleep(Duration::from_millis(delay)).await;
                            continue;
                        }
                    }
                    return Ok(response);
                }
                Err(e) => {
                    if attempt < self.max_retries {
                        debug!("Request failed, attempt {}/{}: {}", attempt, self.max_retries, e);
                        let delay = self.retry_base_ms * (1 << attempt);
                        tokio::time::sleep(Duration::from_millis(delay)).await;
                        continue;
                    }
                    return Err(e);
                }
            }
        }

        anyhow::bail!("Max retries exceeded")
    }

    async fn execute_request(
        &self,
        url: &str,
        payload: &serde_json::Value,
    ) -> Result<DingTalkResponse> {
        let response = self
            .client
            .post(url)
            .json(payload)
            .send()
            .await
            .context("Failed to send request to DingTalk")?;

        let status = response.status();
        let body = response.text().await.context("Failed to read response body")?;

        if !status.is_success() {
            anyhow::bail!("DingTalk API returned status {}: {}", status, body);
        }

        let result: DingTalkResponse =
            serde_json::from_str(&body).context("Failed to parse DingTalk response")?;

        Ok(result)
    }
}
