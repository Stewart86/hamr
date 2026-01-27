//! Color parsing utilities for TUI widgets.
//!
//! Provides hex color parsing for widget styling.

use ratatui::style::Color;

/// Parse a hex color string into a ratatui Color.
///
/// Supports:
/// - 6-digit hex: "#FF5500"
/// - 3-digit shorthand: "#F50" (expands to #FF5500)
///
/// Returns None if the hex string is invalid.
pub fn parse_hex_color(hex: &str) -> Option<Color> {
    let hex = hex.trim_start_matches('#');

    match hex.len() {
        // Full 6-digit hex
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some(Color::Rgb(r, g, b))
        }
        // 3-digit shorthand (e.g., #F50 -> #FF5500)
        3 => {
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
            Some(Color::Rgb(r, g, b))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_colors() {
        assert_eq!(parse_hex_color("#FF0000"), Some(Color::Rgb(255, 0, 0)));
        assert_eq!(parse_hex_color("#00ff00"), Some(Color::Rgb(0, 255, 0)));
        assert_eq!(parse_hex_color("#0000FF"), Some(Color::Rgb(0, 0, 255)));
    }

    #[test]
    fn test_hex_shorthand() {
        assert_eq!(parse_hex_color("#F00"), Some(Color::Rgb(255, 0, 0)));
        assert_eq!(parse_hex_color("#0F0"), Some(Color::Rgb(0, 255, 0)));
        assert_eq!(parse_hex_color("#00F"), Some(Color::Rgb(0, 0, 255)));
    }

    #[test]
    fn test_invalid_hex() {
        assert_eq!(parse_hex_color("#GG0000"), None);
        assert_eq!(parse_hex_color("#12345"), None);
        assert_eq!(parse_hex_color("not-a-color"), None);
    }
}
