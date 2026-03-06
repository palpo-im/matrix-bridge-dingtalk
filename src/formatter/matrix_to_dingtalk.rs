#[derive(Clone)]
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

        result = self.sanitize_html_text(&result);
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

    fn sanitize_html_text(&self, content: &str) -> String {
        let mut result = content.to_string();

        result = regex::Regex::new(r"(?is)<a[^>]*>(.*?)</a>")
            .map(|re| re.replace_all(&result, "$1").to_string())
            .unwrap_or(result);

        result = regex::Regex::new(r"(?is)<br\\s*/?>")
            .map(|re| re.replace_all(&result, "\n").to_string())
            .unwrap_or(result);

        result = regex::Regex::new(r"(?is)</p>")
            .map(|re| re.replace_all(&result, "\n").to_string())
            .unwrap_or(result);

        result = regex::Regex::new(r"(?is)<[^>]+>")
            .map(|re| re.replace_all(&result, "").to_string())
            .unwrap_or(result);

        result = result.replace("&nbsp;", " ");
        result = result.replace("&amp;", "&");
        result = result.replace("&lt;", "<");
        result = result.replace("&gt;", ">");
        result = result.replace("&quot;", "\"");

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

#[cfg(test)]
mod tests {
    use super::MatrixToDingTalkFormatter;

    #[test]
    fn format_text_strips_anchor_tags() {
        let formatter = MatrixToDingTalkFormatter::new();
        let input = r#"<a href=\"https://matrix.to/#/@_dingtalk_manager3165:127.0.0.1:6006\">_dingtalk_manager3165</a>: wefw"#;
        let output = formatter.format_text(input, "@alice:example.com");
        assert_eq!(output, "_dingtalk_manager3165: wefw");
    }

    #[test]
    fn format_text_converts_room_mention() {
        let formatter = MatrixToDingTalkFormatter::new();
        let output = formatter.format_text("hello @room", "@alice:example.com");
        assert_eq!(output, "hello @所有人");
    }
}
