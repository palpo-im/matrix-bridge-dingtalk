use std::sync::Arc;

use crate::config::Config;
use crate::dingtalk::DingTalkService;
use crate::formatter::MatrixToDingTalkFormatter;

pub struct MatrixToDingTalkDispatcher {
    _config: Arc<Config>,
    dingtalk_service: Arc<DingTalkService>,
    formatter: MatrixToDingTalkFormatter,
}

impl MatrixToDingTalkDispatcher {
    pub fn new(_config: Arc<Config>, dingtalk_service: Arc<DingTalkService>) -> Self {
        Self {
            _config,
            dingtalk_service,
            formatter: MatrixToDingTalkFormatter::new(),
        }
    }

    pub async fn dispatch_text(
        &self,
        _conversation_id: &str,
        content: &str,
        sender: &str,
    ) -> anyhow::Result<()> {
        let formatted = self.formatter.format_text(content, sender);
        self.dingtalk_service
            .send_text(&formatted, None, None, false)
            .await?;
        Ok(())
    }

    pub async fn dispatch_markdown(
        &self,
        _conversation_id: &str,
        content: &str,
        title: &str,
    ) -> anyhow::Result<()> {
        self.dingtalk_service
            .send_markdown(title, content, None, None, false)
            .await?;
        Ok(())
    }
}
