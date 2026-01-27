//! Badge widget for TUI.
//!
//! Badges are small indicators displayed beside item names.
//! Each badge can have text (1-3 chars), an icon, and a custom color.
//!
//! ASCII representation: `[!] [2] [*]`

use ratatui::{
    style::{Color, Style},
    text::Span,
};

use super::color::parse_hex_color;
use super::icon::icon_to_str;

/// Default badge color (matches `main.rs` `colors::SECONDARY`)
const DEFAULT_COLOR: Color = Color::Rgb(0xca, 0xc5, 0xc8); // #cac5c8

/// A badge widget - small indicator with text or icon (renders as `[X]`).
struct Badge {
    text: Option<String>,
    icon: Option<String>,
    color: Color,
}

impl Badge {
    /// Create a Badge from RPC Badge type.
    fn from_rpc(badge: &hamr_rpc::Badge) -> Self {
        let color = badge
            .color
            .as_ref()
            .and_then(|c| parse_hex_color(c))
            .unwrap_or(DEFAULT_COLOR);

        Self {
            text: badge.text.clone(),
            icon: badge.icon.clone(),
            color,
        }
    }

    /// Returns `[text]` or `[icon]` matching `main.rs` `render_badge()`.
    fn content(&self) -> String {
        match (&self.text, &self.icon) {
            (Some(text), _) if !text.is_empty() => format!("[{text}]"),
            (_, Some(icon)) => format!("[{}]", icon_to_str(icon)),
            _ => String::new(),
        }
    }

    fn to_span(&self) -> Span<'static> {
        let content = self.content();
        if content.is_empty() {
            Span::raw("")
        } else {
            Span::styled(content, Style::default().fg(self.color))
        }
    }
}

/// Renders an RPC badge as a Span, matching `main.rs` `render_badge()` behavior.
#[must_use]
pub fn render_badge(badge: &hamr_rpc::Badge) -> Span<'static> {
    Badge::from_rpc(badge).to_span()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_badge_with_text() {
        let rpc_badge = hamr_rpc::Badge {
            text: Some("OK".to_string()),
            icon: None,
            color: None,
        };
        let span = render_badge(&rpc_badge);
        assert_eq!(span.content, "[OK]");
    }

    #[test]
    fn test_badge_with_icon() {
        let rpc_badge = hamr_rpc::Badge {
            text: None,
            icon: Some("check".to_string()),
            color: None,
        };
        let span = render_badge(&rpc_badge);
        assert_eq!(span.content, "[+]"); // check -> "+"
    }

    #[test]
    fn test_empty_badge() {
        let rpc_badge = hamr_rpc::Badge {
            text: None,
            icon: None,
            color: None,
        };
        let span = render_badge(&rpc_badge);
        assert_eq!(span.content, "");
    }

    #[test]
    fn test_badge_with_color() {
        let rpc_badge = hamr_rpc::Badge {
            text: Some("3".to_string()),
            icon: None,
            color: Some("#00FF00".to_string()),
        };
        let span = render_badge(&rpc_badge);
        assert_eq!(span.content, "[3]");
    }

    #[test]
    fn test_text_takes_priority_over_icon() {
        let rpc_badge = hamr_rpc::Badge {
            text: Some("X".to_string()),
            icon: Some("check".to_string()),
            color: None,
        };
        let span = render_badge(&rpc_badge);
        assert_eq!(span.content, "[X]"); // text, not icon
    }
}
