use matrix_bot_sdk::appservice::Intent;

#[derive(Debug, Clone)]
pub struct MatrixCommand {
    pub room_id: String,
    pub sender: String,
    pub prefix: String, // "dingtalk" or empty for legacy commands
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
        println!("[DEBUG] CommandHandler::handle called: prefix='{}', command='{}', args={:?}",
            command.prefix, command.command, command.args);

        if !self.enabled {
            println!("[DEBUG] CommandHandler is disabled, returning NoAction");
            return Ok(MatrixCommandOutcome::NoAction);
        }

        // Only handle commands with !dingtalk prefix
        let result = match (command.prefix.as_str(), command.command.as_str()) {
            ("dingtalk", "bridge") => {
                println!("[DEBUG] Handling 'bridge' command");
                self.handle_dingtalk_bridge(command).await
            },
            ("dingtalk", "unbridge") => {
                println!("[DEBUG] Handling 'unbridge' command");
                self.handle_dingtalk_unbridge(command).await
            },
            ("dingtalk", "help") => {
                println!("[DEBUG] Handling 'help' command");
                self.handle_dingtalk_help(command).await
            },
            _ => {
                println!("[DEBUG] Unknown command, returning NoAction");
                Ok(MatrixCommandOutcome::NoAction)
            },
        };

        println!("[DEBUG] CommandHandler result: {:?}", result);
        result
    }

    async fn handle_dingtalk_bridge(&self, command: MatrixCommand) -> anyhow::Result<MatrixCommandOutcome> {
        if command.args.is_empty() {
            return Ok(MatrixCommandOutcome::Error(
                "Usage: !dingtalk bridge <dingtalk_conversation_id>\nExample: !dingtalk bridge \"yourconversationid\"".to_string(),
            ));
        }

        let conversation_id = &command.args[0];
        Ok(MatrixCommandOutcome::Success(format!(
            "Bridge command received. To complete the bridging, use the HTTP API:\n\n\
            curl -X POST http://localhost:9006/admin/bridge \\\n\
              -H \"Authorization: Bearer YOUR_WRITE_TOKEN\" \\\n\
              -H \"Content-Type: application/json\" \\\n\
              -d '{{\n\
                \"matrix_room_id\": \"{}\",\n\
                \"dingtalk_conversation_id\": \"{}\"\n\
            }}'",
            command.room_id, conversation_id
        )))
    }

    async fn handle_dingtalk_unbridge(&self, command: MatrixCommand) -> anyhow::Result<MatrixCommandOutcome> {
        Ok(MatrixCommandOutcome::Success(format!(
            "Unbridge command received. To complete the unbridging, use the HTTP API:\n\n\
            curl -X POST http://localhost:9006/admin/unbridge \\\n\
              -H \"Authorization: Bearer YOUR_WRITE_TOKEN\" \\\n\
              -H \"Content-Type: application/json\" \\\n\
              -d '{{\n\
                \"matrix_room_id\": \"{}\"\n\
            }}'",
            command.room_id
        )))
    }

    async fn handle_dingtalk_help(&self, _command: MatrixCommand) -> anyhow::Result<MatrixCommandOutcome> {
        let help_text = r#"Available DingTalk bridge commands:

!dingtalk bridge <conversation_id> - Link this room to a DingTalk conversation
!dingtalk unbridge - Remove the bridge from this room
!dingtalk help - Show this help message"#;
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

        println!("[DEBUG] parse_command: content='{}', parts={:?}", content, parts);

        // Only handle !dingtalk commands
        if !parts[0].eq_ignore_ascii_case("dingtalk") {
            println!("[DEBUG] parse_command: not a dingtalk command, returning None");
            return None;
        }

        if parts.len() < 2 {
            println!("[DEBUG] parse_command: no subcommand provided, returning None");
            return None;
        }

        let command = parts[1].to_lowercase();
        let args: Vec<String> = parts[2..].iter().map(|s| s.to_string()).collect();

        println!("[DEBUG] parse_command: parsed command='{}', args={:?}", command, args);

        Some(MatrixCommand {
            room_id,
            sender,
            prefix: "dingtalk".to_string(),
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
