#[derive(Clone)]
pub struct DingTalkToMatrixFormatter;

impl DingTalkToMatrixFormatter {
    pub fn new() -> Self {
        Self
    }

    pub fn format_text(&self, content: &str, _sender: &str) -> String {
        let mut result = content.to_string();

        result = self.convert_mentions(&result);

        result
    }

    pub fn format_markdown(&self, content: &str) -> String {
        let html = self.markdown_to_html(content);
        html
    }

    fn convert_mentions(&self, content: &str) -> String {
        let mut result = content.to_string();

        result = result.replace("@所有人", "@room");

        result
    }

    fn markdown_to_html(&self, markdown: &str) -> String {
        let mut html = markdown.to_string();

        html = regex::Regex::new(r"\*\*([^*]+)\*\*")
            .map(|re| re.replace_all(&html, "<strong>$1</strong>").to_string())
            .unwrap_or(html);

        html = regex::Regex::new(r"\*([^*]+)\*")
            .map(|re| re.replace_all(&html, "<em>$1</em>").to_string())
            .unwrap_or(html);

        html = regex::Regex::new(r"`([^`]+)`")
            .map(|re| re.replace_all(&html, "<code>$1</code>").to_string())
            .unwrap_or(html);

        html = regex::Regex::new(r"\[([^\]]+)\]\(([^)]+)\)")
            .map(|re| re.replace_all(&html, r#"<a href="$2">$1</a>"#).to_string())
            .unwrap_or(html);

        html = regex::Regex::new(r"\n")
            .map(|re| re.replace_all(&html, "<br>").to_string())
            .unwrap_or(html);

        html
    }
}

impl Default for DingTalkToMatrixFormatter {
    fn default() -> Self {
        Self::new()
    }
}
