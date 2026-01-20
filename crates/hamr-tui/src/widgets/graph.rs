//! Sparkline/graph widget for displaying time-series data.
//!
//! Uses Unicode block characters to create a compact visualization of data trends.

use ratatui::{
    style::{Color, Style},
    text::Span,
};

const SPARK_CHARS: [char; 9] = [
    '_',        // baseline
    '\u{2581}', // 1/8 block
    '\u{2582}', // 2/8 block
    '\u{2583}', // 3/8 block
    '\u{2584}', // 4/8 block (half)
    '\u{2585}', // 5/8 block
    '\u{2586}', // 6/8 block
    '\u{2587}', // 7/8 block
    '\u{2588}', // full block
];

/// Default sparkline color (matches `main.rs` `colors::PRIMARY`)
const PRIMARY_COLOR: Color = Color::Rgb(0xcb, 0xc4, 0xcb); // #cbc4cb

/// A sparkline widget for compact data visualization as block characters.
#[derive(Debug, Clone)]
pub struct Sparkline {
    data: Vec<f64>,
    min: Option<f64>,
    max: Option<f64>,
    color: Color,
}

impl Sparkline {
    /// Creates a Sparkline from strongly-typed `WidgetData::Graph`.
    #[must_use]
    pub fn from_widget(data: &[f64], min: Option<f64>, max: Option<f64>) -> Self {
        Self {
            data: data.to_vec(),
            min,
            max,
            color: PRIMARY_COLOR,
        }
    }

    fn effective_min(&self) -> f64 {
        self.min
            .unwrap_or_else(|| self.data.iter().copied().fold(f64::INFINITY, f64::min))
    }

    fn effective_max(&self) -> f64 {
        self.max
            .unwrap_or_else(|| self.data.iter().copied().fold(f64::NEG_INFINITY, f64::max))
    }

    /// Renders the last `width` points as a Span.
    // Normalized value 0.0-1.0 to char index 0-8
    #[must_use]
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn to_span(&self, width: usize) -> Span<'static> {
        if self.data.is_empty() {
            return Span::raw("");
        }

        let min = self.effective_min();
        let max = self.effective_max();
        let range = max - min;

        let points: Vec<f64> = self.data.iter().rev().take(width).rev().copied().collect();

        let chars: String = points
            .iter()
            .map(|&v| {
                if range == 0.0 {
                    SPARK_CHARS[4] // middle
                } else {
                    let normalized = ((v - min) / range).clamp(0.0, 1.0);
                    let idx = (normalized * 8.0).round() as usize;
                    SPARK_CHARS[idx.min(8)]
                }
            })
            .collect();

        Span::styled(chars, Style::default().fg(self.color))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sparkline_empty() {
        let sparkline = Sparkline::from_widget(&[], None, None);
        let span = sparkline.to_span(10);
        assert_eq!(span.content, "");
    }

    #[test]
    fn test_sparkline_basic() {
        let sparkline = Sparkline::from_widget(&[0.0, 50.0, 100.0], None, None);
        let span = sparkline.to_span(10);
        assert!(!span.content.is_empty());
        // Should contain block characters
        assert!(span.content.chars().any(|c| SPARK_CHARS.contains(&c)));
    }

    #[test]
    fn test_sparkline_with_bounds() {
        let sparkline = Sparkline::from_widget(&[25.0, 50.0, 75.0], Some(0.0), Some(100.0));
        assert!((sparkline.effective_min() - 0.0).abs() < f64::EPSILON);
        assert!((sparkline.effective_max() - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_sparkline_constant_data() {
        // When all values are the same, should use middle char
        let sparkline = Sparkline::from_widget(&[50.0, 50.0, 50.0], None, None);
        let span = sparkline.to_span(3);
        // All characters should be the same (middle)
        let chars: Vec<char> = span.content.chars().collect();
        assert!(chars.iter().all(|&c| c == chars[0]));
        assert_eq!(chars[0], SPARK_CHARS[4]);
    }

    #[test]
    fn test_sparkline_width_truncation() {
        let data: Vec<f64> = (0..20).map(f64::from).collect();
        let sparkline = Sparkline::from_widget(&data, None, None);
        let span = sparkline.to_span(5);
        // Should only show last 5 data points
        assert_eq!(span.content.chars().count(), 5);
    }
}
