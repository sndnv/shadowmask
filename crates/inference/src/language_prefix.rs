pub(crate) fn render_language_prefix(template: &str, source: Option<&str>, target: &str) -> String {
    template
        .replace("{target}", target)
        .replace("{source}", source.unwrap_or(""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substitutes_target_placeholder() {
        assert_eq!(render_language_prefix("<2{target}>", None, "es"), "<2es>");
        assert_eq!(
            render_language_prefix(">>{target}<<", Some("en"), "fr"),
            ">>fr<<"
        );
    }

    #[test]
    fn substitutes_source_and_target() {
        assert_eq!(
            render_language_prefix("{source}->{target}", Some("en"), "de"),
            "en->de"
        );
    }

    #[test]
    fn missing_source_renders_empty() {
        assert_eq!(render_language_prefix("{source}", None, "es"), "");
    }

    #[test]
    fn literal_template_is_unchanged() {
        assert_eq!(render_language_prefix("plain", None, "es"), "plain");
    }
}
