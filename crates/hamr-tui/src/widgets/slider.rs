//! Slider widget for TUI display.
//!
//! Renders a horizontal slider with filled/empty sections and value display.
//!
//! # Example
//!
//! ```text
//! [################....] 75%
//! ```

use ratatui::{
    style::{Color, Modifier, Style},
    text::Span,
};

const FILLED_CHAR: char = '\u{2588}'; // Full block
const EMPTY_CHAR: char = '\u{2591}'; // Light shade

/// Theme colors matching main.rs
const SUCCESS_COLOR: Color = Color::Rgb(0xb5, 0xcc, 0xba);
const OUTLINE_COLOR: Color = Color::Rgb(0x94, 0x8f, 0x94);
const PRIMARY_COLOR: Color = Color::Rgb(0xcb, 0xc4, 0xcb);

/// A slider widget that displays a value within a range as `[####....] value`.
pub struct Slider<'a> {
    value: f64,
    min: f64,
    max: f64,
    step: f64,
    display_value: Option<&'a str>,
    selected: bool,
}

impl<'a> Slider<'a> {
    /// Creates a slider from individual components.
    #[must_use]
    pub fn from_slider_value(
        value: f64,
        min: f64,
        max: f64,
        step: f64,
        display_value: Option<&'a str>,
    ) -> Self {
        Self {
            value,
            min,
            max,
            step,
            display_value,
            selected: false,
        }
    }

    #[must_use]
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn percentage(&self) -> f64 {
        if self.max <= self.min {
            return 0.0;
        }
        ((self.value - self.min) / (self.max - self.min)).clamp(0.0, 1.0)
    }

    fn format_value(&self) -> String {
        if let Some(display) = self.display_value {
            return display.to_string();
        }

        if self.step >= 1.0 {
            format!("{:.0}", self.value)
        } else if self.step >= 0.1 {
            format!("{:.1}", self.value)
        } else {
            format!("{:.2}", self.value)
        }
    }

    // Percentage is f64, bar width is bounded by terminal size
    #[must_use]
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss
    )]
    pub fn render_inline(&self, available_width: usize) -> Vec<Span<'a>> {
        let mut spans = Vec::new();

        let value_str = self.format_value();
        let value_width = value_str.len() + 1;

        let bracket_width = 2;
        let bar_width = available_width
            .saturating_sub(bracket_width)
            .saturating_sub(value_width)
            .max(5);

        let pct = self.percentage();
        let filled_count = (pct * bar_width as f64).round() as usize;
        let empty_count = bar_width.saturating_sub(filled_count);

        let bracket_style = if self.selected {
            Style::default().fg(PRIMARY_COLOR)
        } else {
            Style::default().fg(OUTLINE_COLOR)
        };
        let value_style = if self.selected {
            Style::default()
                .fg(PRIMARY_COLOR)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(PRIMARY_COLOR)
        };

        spans.push(Span::styled("[", bracket_style));

        if filled_count > 0 {
            spans.push(Span::styled(
                FILLED_CHAR.to_string().repeat(filled_count),
                Style::default().fg(SUCCESS_COLOR),
            ));
        }

        if empty_count > 0 {
            spans.push(Span::styled(
                EMPTY_CHAR.to_string().repeat(empty_count),
                Style::default().fg(OUTLINE_COLOR),
            ));
        }

        spans.push(Span::styled("]", bracket_style));

        spans.push(Span::styled(format!(" {value_str}"), value_style));

        spans
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_percentage_calculation() {
        let slider = Slider::from_slider_value(50.0, 0.0, 100.0, 1.0, None);
        assert!((slider.percentage() - 0.5).abs() < 0.001);

        let slider = Slider::from_slider_value(0.0, 0.0, 100.0, 1.0, None);
        assert!((slider.percentage() - 0.0).abs() < 0.001);

        let slider = Slider::from_slider_value(100.0, 0.0, 100.0, 1.0, None);
        assert!((slider.percentage() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_format_value() {
        let slider = Slider::from_slider_value(75.0, 0.0, 100.0, 1.0, None);
        assert_eq!(slider.format_value(), "75");

        let slider = Slider::from_slider_value(75.5, 0.0, 100.0, 0.1, None);
        assert_eq!(slider.format_value(), "75.5");

        let slider = Slider::from_slider_value(50.0, 0.0, 100.0, 1.0, Some("Custom"));
        assert_eq!(slider.format_value(), "Custom");
    }

    #[test]
    fn test_render_inline_has_brackets() {
        let slider = Slider::from_slider_value(50.0, 0.0, 100.0, 1.0, None);
        let spans = slider.render_inline(30);

        let content: String = spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(content.contains('['));
        assert!(content.contains(']'));
    }
}
