use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct MatrixEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub content: serde_json::Value,
    #[serde(default)]
    pub room_id: Option<String>,
    #[serde(default)]
    pub sender: Option<String>,
    #[serde(default)]
    pub event_id: Option<String>,
    #[serde(default)]
    pub state_key: Option<String>,
    #[serde(default)]
    pub redacts: Option<String>,
    #[serde(default)]
    pub origin_server_ts: Option<i64>,
}

impl MatrixEvent {
    pub fn msgtype(&self) -> Option<&str> {
        self.content.get("msgtype").and_then(|v| v.as_str())
    }

    pub fn body(&self) -> Option<&str> {
        self.content.get("body").and_then(|v| v.as_str())
    }
}

#[derive(Clone)]
pub struct MatrixEventParser;

impl MatrixEventParser {
    pub fn new() -> Self {
        Self
    }

    pub fn parse(&self, event: &MatrixEvent) -> ParsedEvent {
        match event.event_type.as_str() {
            "m.room.message" | "m.sticker" => self.parse_message(event),
            "m.room.member" => self.parse_member(event),
            "m.room.redaction" => self.parse_redaction(event),
            _ => ParsedEvent::Unknown(event.event_type.clone()),
        }
    }

    fn parse_message(&self, event: &MatrixEvent) -> ParsedEvent {
        let content_for_body = event
            .content
            .get("m.new_content")
            .filter(|value| value.is_object())
            .unwrap_or(&event.content);

        let msgtype = content_for_body
            .get("msgtype")
            .and_then(|value| value.as_str())
            .or_else(|| event.msgtype())
            .unwrap_or_else(|| {
                if event.event_type == "m.sticker" {
                    "m.sticker"
                } else {
                    "unknown"
                }
            });

        let body = content_for_body
            .get("body")
            .and_then(|value| value.as_str())
            .or_else(|| event.body())
            .unwrap_or("")
            .to_string();

        let relates_to = event.content.get("m.relates_to");
        let reply_to = relates_to
            .and_then(|relation| relation.get("m.in_reply_to"))
            .and_then(|inner| inner.get("event_id"))
            .and_then(|value| value.as_str())
            .map(ToOwned::to_owned);
        let edit_of = relates_to
            .and_then(|relation| relation.get("rel_type"))
            .and_then(|value| value.as_str())
            .filter(|value| *value == "m.replace")
            .and_then(|_| relates_to.and_then(|relation| relation.get("event_id")))
            .and_then(|value| value.as_str())
            .map(ToOwned::to_owned);

        ParsedEvent::Message {
            msgtype: msgtype.to_string(),
            body,
            formatted_body: content_for_body
                .get("formatted_body")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            reply_to,
            edit_of,
        }
    }

    fn parse_member(&self, event: &MatrixEvent) -> ParsedEvent {
        let membership = event
            .content
            .get("membership")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        ParsedEvent::Member {
            membership: membership.to_string(),
            user_id: event.sender.clone().unwrap_or_default(),
            state_key: event.state_key.clone(),
        }
    }

    fn parse_redaction(&self, event: &MatrixEvent) -> ParsedEvent {
        let redacts = event.redacts.clone().or_else(|| {
            event
                .content
                .get("redacts")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        });

        ParsedEvent::Redaction { redacts }
    }
}

impl Default for MatrixEventParser {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub enum ParsedEvent {
    Message {
        msgtype: String,
        body: String,
        formatted_body: Option<String>,
        reply_to: Option<String>,
        edit_of: Option<String>,
    },
    Member {
        membership: String,
        user_id: String,
        state_key: Option<String>,
    },
    Redaction {
        redacts: Option<String>,
    },
    Unknown(String),
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{MatrixEvent, MatrixEventParser, ParsedEvent};

    #[test]
    fn parse_message_extracts_reply_and_edit_with_new_content() {
        let event = MatrixEvent {
            event_type: "m.room.message".to_string(),
            content: json!({
                "msgtype": "m.text",
                "body": "* old body",
                "m.new_content": {
                    "msgtype": "m.text",
                    "body": "new body",
                    "formatted_body": "<b>new body</b>"
                },
                "m.relates_to": {
                    "m.in_reply_to": {
                        "event_id": "$reply_target"
                    },
                    "rel_type": "m.replace",
                    "event_id": "$edit_target"
                }
            }),
            room_id: Some("!room:example.com".to_string()),
            sender: Some("@alice:example.com".to_string()),
            event_id: Some("$event".to_string()),
            state_key: None,
            redacts: None,
            origin_server_ts: None,
        };

        let parsed = MatrixEventParser::new().parse(&event);
        match parsed {
            ParsedEvent::Message {
                msgtype,
                body,
                formatted_body,
                reply_to,
                edit_of,
            } => {
                assert_eq!(msgtype, "m.text");
                assert_eq!(body, "new body");
                assert_eq!(formatted_body.as_deref(), Some("<b>new body</b>"));
                assert_eq!(reply_to.as_deref(), Some("$reply_target"));
                assert_eq!(edit_of.as_deref(), Some("$edit_target"));
            }
            _ => panic!("expected ParsedEvent::Message"),
        }
    }

    #[test]
    fn parse_member_includes_state_key() {
        let event = MatrixEvent {
            event_type: "m.room.member".to_string(),
            content: json!({
                "membership": "invite"
            }),
            room_id: Some("!room:example.com".to_string()),
            sender: Some("@alice:example.com".to_string()),
            event_id: Some("$member".to_string()),
            state_key: Some("@_dingtalk_bot:example.com".to_string()),
            redacts: None,
            origin_server_ts: None,
        };

        let parsed = MatrixEventParser::new().parse(&event);
        match parsed {
            ParsedEvent::Member {
                membership,
                user_id,
                state_key,
            } => {
                assert_eq!(membership, "invite");
                assert_eq!(user_id, "@alice:example.com");
                assert_eq!(state_key.as_deref(), Some("@_dingtalk_bot:example.com"));
            }
            _ => panic!("expected ParsedEvent::Member"),
        }
    }

    #[test]
    fn parse_redaction_prefers_top_level_field() {
        let event = MatrixEvent {
            event_type: "m.room.redaction".to_string(),
            content: json!({
                "redacts": "$legacy_value"
            }),
            room_id: Some("!room:example.com".to_string()),
            sender: Some("@alice:example.com".to_string()),
            event_id: Some("$redaction".to_string()),
            state_key: None,
            redacts: Some("$top_level".to_string()),
            origin_server_ts: None,
        };

        let parsed = MatrixEventParser::new().parse(&event);
        match parsed {
            ParsedEvent::Redaction { redacts } => {
                assert_eq!(redacts.as_deref(), Some("$top_level"));
            }
            _ => panic!("expected ParsedEvent::Redaction"),
        }
    }
}
