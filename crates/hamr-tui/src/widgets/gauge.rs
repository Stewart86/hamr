//! Gauge widget for TUI.
//!
//! Gauges display progress as a compact mini-bar with label.
//!
//! ASCII representation: `######.... 62%`

use ratatui::{
    style::{Color, Style},
    text::Span,
};

use super::color::parse_hex_color;

const BLOCK_DARK: char = '\u{2593}'; // Dark shade (filled)
const BLOCK_LIGHT: char = '\u{2591}'; // Light shade (empty)

/// Theme colors matching main.rs
const SUCCESS_COLOR: Color = Color::Rgb(0xb5, 0xcc, 0xba); // #B5CCBA
const OUTLINE_COLOR: Color = Color::Rgb(0x94, 0x8f, 0x94); // #948f94
const SUBTEXT_COLOR: Color = Color::Rgb(0xcb, 0xc5, 0xca); // #cbc5ca

/// A gauge widget - compact progress bar with label (renders as `######.... 62%`).
#[derive(Debug, Clone)]
pub struct Gauge {
    value: f64,
    min: f64,
    max: f64,
    label: Option<String>,
    color: Color,
}

impl Gauge {
    /// Creates a Gauge from strongly-typed `WidgetData::Gauge`.
    #[must_use]
    pub fn from_widget(
        value: f64,
        min: f64,
        max: f64,
        label: Option<&str>,
        color_str: Option<&str>,
    ) -> Self {
        let color = color_str.and_then(parse_hex_color).unwrap_or(SUCCESS_COLOR);

        Self {
            value,
            min,
            max,
            label: label.map(ToString::to_string),
            color,
        }
    }

    fn percentage(&self) -> f64 {
        if self.max > self.min {
            ((self.value - self.min) / (self.max - self.min)).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    fn display_label(&self) -> String {
        self.label
            .clone()
            .unwrap_or_else(|| format!("{:.0}%", self.percentage() * 100.0))
    }

    /// Renders as colored filled portion, outline empty portion, and label.
    // Gauge percentage math uses f64, filled width is bounded by terminal width
    #[must_use]
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss
    )]
    pub fn to_spans(&self, width: usize) -> Vec<Span<'static>> {
        let pct = self.percentage();
        let filled = (pct * width as f64).round() as usize;
        let empty = width.saturating_sub(filled);
        let label = self.display_label();

        vec![
            Span::styled(
                BLOCK_DARK.to_string().repeat(filled),
                Style::default().fg(self.color),
            ),
            Span::styled(
                BLOCK_LIGHT.to_string().repeat(empty),
                Style::default().fg(OUTLINE_COLOR),
            ),
            Span::styled(format!(" {label}"), Style::default().fg(SUBTEXT_COLOR)),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gauge_percentage() {
        let gauge = Gauge::from_widget(50.0, 0.0, 100.0, None, None);
        assert!((gauge.percentage() - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_gauge_with_range() {
        let gauge = Gauge::from_widget(75.0, 50.0, 100.0, None, None);
        assert!((gauge.percentage() - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_gauge_label() {
        let gauge = Gauge::from_widget(50.0, 0.0, 100.0, Some("Half"), None);
        assert_eq!(gauge.display_label(), "Half");
    }

    #[test]
    fn test_gauge_default_label() {
        let gauge = Gauge::from_widget(50.0, 0.0, 100.0, None, None);
        assert_eq!(gauge.display_label(), "50%");
    }

    #[test]
    fn test_gauge_with_color() {
        let gauge = Gauge::from_widget(75.0, 0.0, 100.0, None, Some("#00FF00"));
        assert_eq!(gauge.color, Color::Rgb(0, 255, 0));
    }

    #[test]
    fn test_gauge_default_color() {
        let gauge = Gauge::from_widget(50.0, 0.0, 100.0, None, None);
        assert_eq!(gauge.color, SUCCESS_COLOR);
    }

    #[test]
    fn test_to_spans() {
        let gauge = Gauge::from_widget(50.0, 0.0, 100.0, None, None);
        let spans = gauge.to_spans(10);
        assert_eq!(spans.len(), 3); // filled + empty + label
    }
}
