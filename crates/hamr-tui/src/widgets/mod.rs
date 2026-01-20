//! TUI widgets for hamr-tui.
//!
//! This module contains custom ratatui widgets used throughout the TUI.
//!
//! # Widgets in Use
//!
//! - [`render_badge`] - Render a badge from RPC type
//! - [`render_chip`] - Render a chip from RPC type
//! - [`Gauge`] - Circular gauge widget (from `WidgetData::Gauge`)
//! - [`Sparkline`] - Sparkline/graph widget (from `WidgetData::Graph`)
//! - [`Slider`] - Interactive slider widget
//!
//! # Utilities
//!
//! - [`icon_to_str`] - Icon name to display string

mod badge;
mod chip;
mod color;
mod gauge;
mod graph;
mod icon;
mod slider;

pub use badge::render_badge;
pub use chip::render_chip;
pub use gauge::Gauge;
pub use graph::Sparkline;
pub use icon::icon_to_str;
pub use slider::Slider;
