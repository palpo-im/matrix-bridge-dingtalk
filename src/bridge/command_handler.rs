use matrix_bot_sdk::appservice::Intent;

#[derive(Debug, Clone)]
pub struct MatrixCommand {
    pub room_id: String,
    pub sender: String,
    pub command: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MatrixCommandOutcome {
    Success(String),
    Error(String),
    NoAction,
}

pub struct MatrixCommandHandler {
    enabled: bool,
}

impl MatrixCommandHandler {
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    pub async fn handle(
        &self,
        command: MatrixCommand,
        _intent: &Intent,
    ) -> anyhow::Result<MatrixCommandOutcome> {
        if !self.enabled {
            return Ok(MatrixCommandOutcome::NoAction);
        }

        match command.command.as_str() {
            "bridge" => self.handle_bridge(command).await,
            "unbridge" => self.handle_unbridge(command).await,
            "help" => self.handle_help(command).await,
            _ => Ok(MatrixCommandOutcome::NoAction),
        }
    }

    async fn handle_bridge(&self, _command: MatrixCommand) -> anyhow::Result<MatrixCommandOutcome> {
        Ok(MatrixCommandOutcome::Success(
            "Bridge command received. Please follow the provisioning process.".to_string(),
        ))
    }

    async fn handle_unbridge(
        &self,
        _command: MatrixCommand,
    ) -> anyhow::Result<MatrixCommandOutcome> {
        Ok(MatrixCommandOutcome::Success(
            "Unbridge command received. Room will be unbridged.".to_string(),
        ))
    }

    async fn handle_help(&self, _command: MatrixCommand) -> anyhow::Result<MatrixCommandOutcome> {
        let help_text = r#"Available commands:
!bridge - Start the bridge provisioning process
!unbridge - Remove the bridge from this room
!help - Show this help message"#;
        Ok(MatrixCommandOutcome::Success(help_text.to_string()))
    }

    pub fn parse_command(content: &str, room_id: String, sender: String) -> Option<MatrixCommand> {
        let content = content.trim();
        if !content.starts_with('!') {
            return None;
        }

        let parts: Vec<&str> = content[1..].split_whitespace().collect();
        if parts.is_empty() {
            return None;
        }

        let command = parts[0].to_lowercase();
        let args: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();

        Some(MatrixCommand {
            room_id,
            sender,
            command,
            args,
        })
    }
}

#[derive(Debug, Clone)]
pub struct DingTalkCommand {
    pub conversation_id: String,
    pub sender_id: String,
    pub command: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DingTalkCommandOutcome {
    Success(String),
    Error(String),
    NoAction,
}

pub struct DingTalkCommandHandler {
    enabled: bool,
}

impl DingTalkCommandHandler {
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    pub async fn handle(
        &self,
        _command: DingTalkCommand,
    ) -> anyhow::Result<DingTalkCommandOutcome> {
        if !self.enabled {
            return Ok(DingTalkCommandOutcome::NoAction);
        }

        Ok(DingTalkCommandOutcome::NoAction)
    }
}
