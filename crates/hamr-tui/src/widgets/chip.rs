//! Chip widget for TUI.
//!
//! Chips are pill-shaped tags with text, displayed beside item names.
//! Each chip can have text and an optional icon.
//!
//! ASCII representation: `(tag) (icon text)`

use ratatui::{
    style::{Color, Style},
    text::Span,
};

use super::icon::icon_to_str;

/// Default chip color (matches `main.rs` `colors::PRIMARY`)
const DEFAULT_COLOR: Color = Color::Rgb(0xcb, 0xc4, 0xcb); // #cbc4cb

/// A chip widget - pill-shaped tag with text (renders as `(text)` or `(icon text)`).
struct Chip {
    text: String,
    icon: Option<String>,
    color: Color,
}

impl Chip {
    /// Create a Chip from RPC Chip type.
    fn from_rpc(chip: &hamr_rpc::Chip) -> Self {
        Self {
            text: chip.text.clone(),
            icon: chip.icon.clone(),
            color: DEFAULT_COLOR,
        }
    }

    /// Returns `(text)` or `(icon text)` matching `main.rs` `render_chip()`.
    fn content(&self) -> String {
        if let Some(icon) = &self.icon {
            format!("({} {})", icon_to_str(icon), self.text)
        } else {
            format!("({})", self.text)
        }
    }

    fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    fn to_span(&self) -> Span<'static> {
        if self.is_empty() {
            Span::raw("")
        } else {
            Span::styled(self.content(), Style::default().fg(self.color))
        }
    }
}

/// Renders an RPC chip as a Span, matching `main.rs` `render_chip()` behavior.
#[must_use]
pub fn render_chip(chip: &hamr_rpc::Chip) -> Span<'static> {
    Chip::from_rpc(chip).to_span()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chip_text_only() {
        let rpc_chip = hamr_rpc::Chip {
            text: "tag".to_string(),
            ..Default::default()
        };
        let span = render_chip(&rpc_chip);
        assert_eq!(span.content, "(tag)");
    }

    #[test]
    fn test_chip_with_icon() {
        let rpc_chip = hamr_rpc::Chip {
            text: "status".to_string(),
            icon: Some("check".to_string()),
            ..Default::default()
        };
        let span = render_chip(&rpc_chip);
        assert_eq!(span.content, "(+ status)"); // check -> "+"
    }

    #[test]
    fn test_empty_chip() {
        let rpc_chip = hamr_rpc::Chip {
            text: String::new(),
            ..Default::default()
        };
        let span = render_chip(&rpc_chip);
        assert_eq!(span.content, "");
    }

    #[test]
    fn test_chip_unknown_icon() {
        let rpc_chip = hamr_rpc::Chip {
            text: "v2.0".to_string(),
            icon: Some("star".to_string()), // star is not in icon_to_str, returns "star"
            ..Default::default()
        };
        let span = render_chip(&rpc_chip);
        assert_eq!(span.content, "(star v2.0)");
    }
}
