//! Syntax highlighting for code blocks using syntect.

use syntect::highlighting::{Style, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

/// A span of highlighted text with color information
#[derive(Debug, Clone)]
pub struct HighlightSpan {
    pub text: String,
    pub color: String,
}

/// Syntax highlighter using syntect
pub struct SyntaxHighlighter {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
}

impl SyntaxHighlighter {
    /// Create a new syntax highlighter with default syntax definitions
    pub fn new() -> Self {
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
        }
    }

    /// Highlight code and return spans with color information
    pub fn highlight(&self, code: &str, lang: Option<&str>) -> Vec<Vec<HighlightSpan>> {
        let syntax = lang
            .and_then(|l| self.find_syntax(l))
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        // Use base16-eighties.dark theme - good for dark backgrounds
        let theme = &self.theme_set.themes["base16-eighties.dark"];

        let mut highlighter = syntect::easy::HighlightLines::new(syntax, theme);
        let mut result = Vec::new();

        for line in LinesWithEndings::from(code) {
            let ranges = highlighter
                .highlight_line(line, &self.syntax_set)
                .unwrap_or_default();

            let spans: Vec<HighlightSpan> = ranges
                .into_iter()
                .map(|(style, text)| HighlightSpan {
                    text: text.to_string(),
                    color: style_to_css_color(&style),
                })
                .collect();

            result.push(spans);
        }

        result
    }

    /// Find syntax definition by language name or extension
    fn find_syntax(&self, lang: &str) -> Option<&syntect::parsing::SyntaxReference> {
        let lang_lower = lang.to_lowercase();

        // Try exact name match first
        if let Some(s) = self.syntax_set.find_syntax_by_token(&lang_lower) {
            return Some(s);
        }

        // Try common aliases
        let mapped = match lang_lower.as_str() {
            "js" => "javascript",
            "ts" => "typescript",
            "py" => "python",
            "rb" => "ruby",
            "rs" => "rust",
            "sh" | "bash" | "zsh" => "shell",
            "yml" => "yaml",
            "md" => "markdown",
            "dockerfile" => "docker",
            _ => &lang_lower,
        };

        self.syntax_set.find_syntax_by_token(mapped)
    }
}

impl Default for SyntaxHighlighter {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert syntect style to CSS hex color
fn style_to_css_color(style: &Style) -> String {
    format!(
        "#{:02x}{:02x}{:02x}",
        style.foreground.r, style.foreground.g, style.foreground.b
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlight_rust() {
        let highlighter = SyntaxHighlighter::new();
        let code = "fn main() {\n    println!(\"Hello\");\n}";
        let result = highlighter.highlight(code, Some("rust"));

        assert!(!result.is_empty());
        let total_spans: usize = result.iter().map(std::vec::Vec::len).sum();
        assert!(total_spans > 1);

        // Verify we have different colors
        let unique_colors: std::collections::HashSet<_> =
            result.iter().flatten().map(|s| &s.color).collect();
        assert!(
            unique_colors.len() > 1,
            "Should have multiple different colors"
        );
    }

    #[test]
    fn test_highlight_unknown_lang() {
        let highlighter = SyntaxHighlighter::new();
        let code = "some plain text";
        let result = highlighter.highlight(code, Some("nonexistent_language"));

        assert!(!result.is_empty());
    }

    #[test]
    fn test_lang_aliases() {
        let highlighter = SyntaxHighlighter::new();

        // Test common aliases that syntect definitely supports
        assert!(highlighter.find_syntax("js").is_some());
        assert!(highlighter.find_syntax("py").is_some());
        assert!(highlighter.find_syntax("rs").is_some());
        assert!(highlighter.find_syntax("sh").is_some());
    }
}
