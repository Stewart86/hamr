//! Rendering functions for the TUI.
//!
//! This module contains all UI rendering logic, organized by view type.

mod ambient;
mod card;
mod error;
mod form;
mod grid_browser;
mod helpers;
mod image_browser;
mod preview;
mod results;
mod window_picker;

pub use ambient::render_ambient_bar;
pub use card::render_card;
pub use error::render_error;
pub use form::render_form;
pub use grid_browser::render_grid_browser;
pub use helpers::{render_progress_bar, render_slider_spans};
pub use image_browser::render_image_browser;
pub use preview::render_preview_panel;
pub use results::render_results_ui;
pub use window_picker::render_window_picker;
