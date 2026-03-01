pub struct MatrixToDingTalkFormatter {
    max_text_length: usize,
}

impl MatrixToDingTalkFormatter {
    pub fn new() -> Self {
        Self {
            max_text_length: 20000,
        }
    }

    pub fn format_text(&self, content: &str, _sender: &str) -> String {
        let mut result = content.to_string();

        result = self.convert_mentions(&result);

        if result.len() > self.max_text_length {
            result = format!("{}... (truncated)", &result[..self.max_text_length]);
        }

        result
    }

    pub fn format_markdown(&self, content: &str, title: &str) -> (String, String) {
        let markdown = self.html_to_markdown(content);
        (title.to_string(), markdown)
    }

    fn convert_mentions(&self, content: &str) -> String {
        let mut result = content.to_string();

        result = result.replace("@room", "@所有人");

        result
    }

    fn html_to_markdown(&self, html: &str) -> String {
        let mut markdown = html.to_string();

        markdown = markdown.replace("<strong>", "**");
        markdown = markdown.replace("</strong>", "**");
        markdown = markdown.replace("<b>", "**");
        markdown = markdown.replace("</b>", "**");

        markdown = markdown.replace("<em>", "*");
        markdown = markdown.replace("</em>", "*");
        markdown = markdown.replace("<i>", "*");
        markdown = markdown.replace("</i>", "*");

        markdown = markdown.replace("<code>", "`");
        markdown = markdown.replace("</code>", "`");

        markdown = regex::Regex::new(r#"<a[^>]*href="([^"]+)"[^>]*>([^<]+)</a>"#)
            .map(|re| re.replace_all(&markdown, "[$2]($1)").to_string())
            .unwrap_or(markdown);

        markdown = regex::Regex::new(r"<br\s*/?>")
            .map(|re| re.replace_all(&markdown, "\n").to_string())
            .unwrap_or(markdown);

        markdown = regex::Regex::new(r"</p>")
            .map(|re| re.replace_all(&markdown, "\n").to_string())
            .unwrap_or(markdown);
        markdown = regex::Regex::new(r"<p>")
            .map(|re| re.replace_all(&markdown, "").to_string())
            .unwrap_or(markdown);

        markdown
    }
}

impl Default for MatrixToDingTalkFormatter {
    fn default() -> Self {
        Self::new()
    }
}
