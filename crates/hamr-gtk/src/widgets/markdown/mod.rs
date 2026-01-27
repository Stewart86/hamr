//! Reusable markdown rendering widget for GTK4.
//!
//! This module provides a `MarkdownView` widget that renders markdown content
//! using `pulldown-cmark` and displays it in a styled GTK `TextView`.
//!
//! ## Usage
//! ```ignore
//! let view = MarkdownView::new();
//! view.set_content("# Hello\n\nThis is **bold** and *italic*.");
//! container.append(view.widget());
//! ```

mod images;
mod renderer;
pub mod syntax;
mod tags;

use gtk4::prelude::*;
use gtk4::{ScrolledWindow, TextBuffer, TextView};
use gtk4::{gdk, gio};

/// A widget that renders markdown content.
#[allow(dead_code)]
pub struct MarkdownView {
    scrolled_window: ScrolledWindow,
    text_view: TextView,
    buffer: TextBuffer,
    max_height: std::cell::Cell<i32>,
}

impl MarkdownView {
    /// Create a new `MarkdownView` widget.
    pub fn new() -> Self {
        let tag_table = tags::create_tag_table();
        let buffer = TextBuffer::builder().tag_table(&tag_table).build();

        let text_view = TextView::builder()
            .buffer(&buffer)
            .editable(false)
            .cursor_visible(false)
            .wrap_mode(gtk4::WrapMode::WordChar) // WordChar wraps more aggressively
            .left_margin(16)
            .right_margin(16)
            .top_margin(12)
            .bottom_margin(12)
            .pixels_above_lines(1)
            .pixels_below_lines(1)
            .pixels_inside_wrap(2)
            .hexpand(false) // Don't expand horizontally
            .build();

        text_view.add_css_class("markdown-view");

        Self::setup_link_click_handler(&text_view);
        Self::setup_link_cursor_handler(&text_view);

        // Use ScrolledWindow with explicit size constraints
        // The key is to NOT propagate natural width and set max_content_width
        let scrolled_window = ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .propagate_natural_height(true)
            .propagate_natural_width(false) // Critical: don't let content determine width
            .hexpand(false)
            .vexpand(true)
            .child(&text_view)
            .build();

        Self {
            scrolled_window,
            text_view,
            buffer,
            max_height: std::cell::Cell::new(i32::MAX),
        }
    }

    fn setup_link_click_handler(text_view: &TextView) {
        let gesture = gtk4::GestureClick::new();
        gesture.set_button(gdk::BUTTON_PRIMARY);

        let tv = text_view.clone();
        gesture.connect_released(move |_, _, x, y| {
            if let Some(url) = Self::get_link_at_coords(&tv, x, y) {
                tracing::debug!("Link clicked: {}", url);
                Self::open_url(&url);
            }
        });

        text_view.add_controller(gesture);
    }

    fn setup_link_cursor_handler(text_view: &TextView) {
        let motion = gtk4::EventControllerMotion::new();

        let tv = text_view.clone();
        motion.connect_motion(move |_, x, y| {
            let is_link = Self::get_link_at_coords(&tv, x, y).is_some();
            if is_link {
                tv.set_cursor_from_name(Some("pointer"));
            } else {
                tv.set_cursor_from_name(Some("text"));
            }
        });

        text_view.add_controller(motion);
    }

    // Mouse coords are f64, GTK buffer coords are i32
    #[allow(clippy::cast_possible_truncation)]
    fn get_link_at_coords(text_view: &TextView, x: f64, y: f64) -> Option<String> {
        let (bx, by) =
            text_view.window_to_buffer_coords(gtk4::TextWindowType::Widget, x as i32, y as i32);
        let iter = text_view.iter_at_location(bx, by)?;

        for tag in iter.tags() {
            if let Some(name) = tag.name()
                && name.starts_with("link-")
            {
                return tags::get_link_url(&tag);
            }
        }
        None
    }

    fn open_url(url: &str) {
        let launcher = gtk4::UriLauncher::new(url);
        launcher.launch(gtk4::Window::NONE, gio::Cancellable::NONE, |result| {
            if let Err(e) = result {
                tracing::error!("Failed to open URL: {}", e);
            }
        });
    }

    /// Set the markdown content to render.
    pub fn set_content(&self, markdown: &str) {
        tracing::debug!(
            "MarkdownView::set_content: {} chars, first 100: {:?}",
            markdown.len(),
            &markdown[..markdown.len().min(100)]
        );
        renderer::render_markdown(markdown, &self.buffer, &self.text_view);

        // Measure actual content height and set min_content_height
        // This is needed because GtkFixed container doesn't respect natural height
        // Without min_content_height, content collapses to minimum size (tiny bar)
        if markdown.is_empty() {
            self.scrolled_window.set_min_content_height(0);
        } else {
            let (_, natural_height, _, _) = self.text_view.measure(gtk4::Orientation::Vertical, -1);
            let max_h = self.max_height.get();
            let min_height = natural_height.min(max_h);
            self.scrolled_window.set_min_content_height(min_height);
        }
    }

    /// Clear all content.
    pub fn clear(&self) {
        self.buffer.set_text("");
        self.scrolled_window.set_min_content_height(0);
    }

    /// Get the root widget for embedding in containers.
    pub fn widget(&self) -> &ScrolledWindow {
        &self.scrolled_window
    }

    /// Get the underlying `TextView` for additional customization.
    #[allow(dead_code)]
    pub fn text_view(&self) -> &TextView {
        &self.text_view
    }

    /// Get the underlying `TextBuffer`.
    #[allow(dead_code)]
    pub fn buffer(&self) -> &TextBuffer {
        &self.buffer
    }

    /// Set a fixed width for the markdown view.
    /// This constrains both the scrolled window and text view.
    pub fn set_width(&self, width: i32) {
        // Set scrolled window constraints
        self.scrolled_window.set_size_request(width, -1);
        self.scrolled_window.set_max_content_width(width);
        self.scrolled_window.set_min_content_width(width);
    }

    /// Set the maximum height for markdown content.
    pub fn set_max_content_height(&self, height: i32) {
        self.max_height.set(height);
        self.scrolled_window.set_max_content_height(height);
        // min_content_height will be computed dynamically in set_content()
    }
}

impl Default for MarkdownView {
    fn default() -> Self {
        Self::new()
    }
}
