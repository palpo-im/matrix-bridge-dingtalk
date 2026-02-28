pub fn apply_pattern_string(pattern: &str, replacements: &[(&str, &str)]) -> String {
    let mut result = pattern.to_string();
    for (key, value) in replacements {
        result = result.replace(&format!(":{}", key), value);
    }
    result
}

pub fn preview_text(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        text.to_string()
    } else {
        format!("{}...", &text[..max_len])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_pattern_string() {
        let pattern = "[DingTalk] :name - :topic";
        let replacements = &[("name", "General"), ("topic", "Discussion")];
        assert_eq!(
            apply_pattern_string(pattern, replacements),
            "[DingTalk] General - Discussion"
        );
    }

    #[test]
    fn test_preview_text() {
        assert_eq!(preview_text("hello", 10), "hello");
        assert_eq!(preview_text("hello world", 5), "hello...");
    }
}
