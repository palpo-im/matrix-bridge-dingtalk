use std::sync::Arc;

use matrix_bot_sdk::appservice::Intent;
use tracing::{debug, error, info};

use super::matrix_event_parser::{MatrixEvent, MatrixEventParser, ParsedEvent};
use crate::config::Config;
use crate::dingtalk::DingTalkService;

pub struct MatrixEventProcessor {
    config: Arc<Config>,
    dingtalk_service: Arc<DingTalkService>,
    parser: MatrixEventParser,
}

impl MatrixEventProcessor {
    pub fn new(config: Arc<Config>, dingtalk_service: Arc<DingTalkService>) -> Self {
        Self {
            config,
            dingtalk_service,
            parser: MatrixEventParser::new(),
        }
    }

    pub async fn process(&self, event: MatrixEvent, _intent: &Intent) -> anyhow::Result<()> {
        let parsed = self.parser.parse(&event);

        match parsed {
            ParsedEvent::Message {
                msgtype,
                body,
                formatted_body: _,
            } => {
                self.process_message(event, msgtype, body).await?;
            }
            ParsedEvent::Member { membership, user_id } => {
                debug!("Member event: {} - {}", user_id, membership);
            }
            ParsedEvent::Redaction { redacts } => {
                debug!("Redaction event: {:?}", redacts);
            }
            ParsedEvent::Unknown(event_type) => {
                debug!("Unknown event type: {}", event_type);
            }
        }

        Ok(())
    }

    async fn process_message(
        &self,
        event: MatrixEvent,
        msgtype: String,
        body: String,
    ) -> anyhow::Result<()> {
        let room_id = event.room_id.unwrap_or_default();
        let sender = event.sender.unwrap_or_default();

        info!(
            "Processing Matrix message: room={}, sender={}, type={}",
            room_id, sender, msgtype
        );

        match msgtype.as_str() {
            "m.text" => {
                self.dingtalk_service
                    .send_text(&body, None, None, false)
                    .await?;
            }
            "m.notice" => {
                self.dingtalk_service
                    .send_text(&body, None, None, false)
                    .await?;
            }
            "m.emote" => {
                let emote_body = format!("* {}", body);
                self.dingtalk_service
                    .send_text(&emote_body, None, None, false)
                    .await?;
            }
            "m.image" | "m.video" | "m.audio" | "m.file" => {
                debug!("Media message type: {}", msgtype);
            }
            _ => {
                debug!("Unsupported message type: {}", msgtype);
            }
        }

        Ok(())
    }

    pub async fn handle_transaction(
        &self,
        events: Vec<MatrixEvent>,
        intent: &Intent,
    ) -> anyhow::Result<()> {
        for event in events {
            if let Err(e) = self.process(event, intent).await {
                error!("Failed to process event: {}", e);
            }
        }
        Ok(())
    }
}
