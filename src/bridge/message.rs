use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeMessage {
    pub msg_id: String,
    pub msg_type: MessageType,
    pub content: String,
    pub sender: String,
    pub room_id: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    Text,
    Markdown,
    Image,
    Audio,
    Video,
    File,
    Notice,
    Emote,
    Unknown,
}

impl MessageType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Markdown => "markdown",
            Self::Image => "image",
            Self::Audio => "audio",
            Self::Video => "video",
            Self::File => "file",
            Self::Notice => "notice",
            Self::Emote => "emote",
            Self::Unknown => "unknown",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "text" | "m.text" => Self::Text,
            "markdown" | "m.markdown" => Self::Markdown,
            "image" | "m.image" => Self::Image,
            "audio" | "m.audio" => Self::Audio,
            "video" | "m.video" => Self::Video,
            "file" | "m.file" => Self::File,
            "notice" | "m.notice" => Self::Notice,
            "emote" | "m.emote" => Self::Emote,
            _ => Self::Unknown,
        }
    }
}

impl std::fmt::Display for MessageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
