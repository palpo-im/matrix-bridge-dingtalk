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
    pub origin_server_ts: Option<i64>,
}

impl MatrixEvent {
    pub fn is_message(&self) -> bool {
        self.event_type == "m.room.message"
    }

    pub fn is_member(&self) -> bool {
        self.event_type == "m.room.member"
    }

    pub fn is_redaction(&self) -> bool {
        self.event_type == "m.room.redaction"
    }

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
            "m.room.message" => self.parse_message(event),
            "m.room.member" => self.parse_member(event),
            "m.room.redaction" => self.parse_redaction(event),
            _ => ParsedEvent::Unknown(event.event_type.clone()),
        }
    }

    fn parse_message(&self, event: &MatrixEvent) -> ParsedEvent {
        let msgtype = event.msgtype().unwrap_or("unknown");
        let body = event.body().unwrap_or("").to_string();

        ParsedEvent::Message {
            msgtype: msgtype.to_string(),
            body,
            formatted_body: event
                .content
                .get("formatted_body")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
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
        }
    }

    fn parse_redaction(&self, event: &MatrixEvent) -> ParsedEvent {
        let redacts = event
            .content
            .get("redacts")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

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
    },
    Member {
        membership: String,
        user_id: String,
    },
    Redaction {
        redacts: Option<String>,
    },
    Unknown(String),
}
