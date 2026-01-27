//! Preview panel widget for displaying item details in a side panel.
//!
//! This widget renders `PreviewData` from search results, including:
//! - Title
//! - Image preview
//! - Markdown/text content
//! - Metadata key-value pairs
//! - Action buttons

use crate::thumbnail_cache::preview_cache;
use gtk4::gdk;
use gtk4::gio;
use gtk4::glib::object::ObjectExt;
use gtk4::prelude::*;
use gtk4::{Align, Label, Orientation, ScrolledWindow};
use hamr_types::{Action, PreviewData};
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;

use super::design;
use super::markdown::MarkdownView;
use super::ripple_button::RippleButton;

// Use centralized design constants
use design::preview_panel as preview_design;

/// Callback type for action button clicks: (`item_id`, `action_id`)
type ActionCallback = Rc<RefCell<Option<Box<dyn Fn(&str, &str)>>>>;

/// Callback type for pin button clicks: (`item_id`, `title`, `preview_data`)
type PinCallback = Rc<RefCell<Option<Box<dyn Fn(&str, Option<String>, &PreviewData)>>>>;

/// A widget that displays preview data for the selected search result.
pub struct PreviewPanel {
    /// Outer container (includes border/background styling)
    container: gtk4::Box,
    /// Scrolled window for plain text content
    #[allow(dead_code)]
    scrolled: ScrolledWindow,
    /// Title label
    title_label: Label,
    /// Pin button (kept for potential future styling)
    #[allow(dead_code)]
    pin_button: gtk4::Box,
    /// Image container (holds `DrawingArea` for image)
    image_container: gtk4::Box,
    /// Content box (styled container for content)
    content_box: gtk4::Box,
    /// Content label (simple text display)
    content_label: Label,
    /// Markdown view for rendered markdown content
    markdown_view: MarkdownView,
    /// Metadata grid
    metadata_grid: gtk4::Grid,
    /// Actions container
    actions_container: gtk4::Box,
    /// Callback for action clicks
    on_action: ActionCallback,
    /// Callback for pin clicks
    on_pin: PinCallback,
    /// Current item ID for action context
    current_item_id: Rc<RefCell<String>>,
    /// Current preview data (for pinning)
    current_preview: Rc<RefCell<Option<PreviewData>>>,
}

impl PreviewPanel {
    /// Create a new `PreviewPanel` widget.
    // Widget construction - sequential GTK builder calls with gesture callback setup
    #[allow(clippy::too_many_lines)]
    pub fn new() -> Self {
        // Main container with rounded corners and border (styled via CSS)
        let container = gtk4::Box::builder()
            .orientation(Orientation::Vertical)
            .css_classes(["preview-panel"])
            .build();

        // Force exact width
        container.set_size_request(preview_design::WIDTH, -1);

        // Content width = panel width - padding on both sides
        let content_width =
            preview_design::WIDTH - preview_design::PADDING_LEFT - preview_design::PADDING_RIGHT;

        let header_box = gtk4::Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(8)
            .margin_start(preview_design::PADDING_LEFT)
            .margin_end(preview_design::PADDING_RIGHT)
            .margin_top(preview_design::PADDING_TOP)
            .margin_bottom(8)
            .build();

        // Title (left-aligned, takes remaining space)
        let title_label = Label::builder()
            .css_classes(["preview-title"])
            .halign(Align::Start)
            .hexpand(true)
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .single_line_mode(true)
            .build();
        header_box.append(&title_label);

        // Pin button (right side) - pins the preview as a sticky note
        let pin_button = gtk4::Box::builder()
            .css_classes(["preview-pin-button"])
            .halign(Align::End)
            .valign(Align::Center)
            .tooltip_text("Pin as sticky note")
            .build();
        let pin_icon = Label::builder()
            .label("push_pin")
            .css_classes(["material-icon", "preview-pin-icon"])
            .build();
        pin_button.append(&pin_icon);
        header_box.append(&pin_button);

        // Make pin button clickable
        let pin_gesture = gtk4::GestureClick::new();
        pin_button.add_controller(pin_gesture.clone());

        container.append(&header_box);

        let content_box = gtk4::Box::builder()
            .orientation(Orientation::Vertical)
            .css_classes(["preview-content-box"])
            .margin_start(preview_design::PADDING_LEFT)
            .margin_end(preview_design::PADDING_RIGHT)
            .margin_top(preview_design::PADDING_TOP)
            .vexpand(false) // Don't expand - size to content
            .overflow(gtk4::Overflow::Hidden)
            .build();

        let image_container = gtk4::Box::builder()
            .orientation(Orientation::Vertical)
            .halign(Align::Center)
            .valign(Align::Start)
            .css_classes(["preview-image-container"])
            .build();
        image_container.set_visible(false);
        content_box.append(&image_container);

        // Scrolled window inside content box (for plain text)
        let scrolled = ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .propagate_natural_height(true)
            .propagate_natural_width(false)
            .hexpand(true)
            .vexpand(true)
            .build();

        // Content label for plain text display
        let content_label = Label::builder()
            .css_classes(["preview-content"])
            .halign(Align::Start)
            .valign(Align::Start)
            .wrap(true)
            .wrap_mode(gtk4::pango::WrapMode::WordChar)
            .selectable(true)
            .hexpand(true)
            .build();
        content_label.set_size_request(content_width - 24, -1); // Account for padding
        scrolled.set_child(Some(&content_label));
        content_box.append(&scrolled);

        // Markdown view for rendered markdown content
        let markdown_view = MarkdownView::new();
        markdown_view.set_width(content_width - 24); // Account for padding
        markdown_view.widget().set_visible(false);
        markdown_view.widget().set_max_content_height(400); // Limit height
        markdown_view.text_view().add_css_class("preview-markdown");
        content_box.append(markdown_view.widget());

        content_box.set_visible(false);
        container.append(&content_box);

        let metadata_grid = gtk4::Grid::builder()
            .row_spacing(preview_design::METADATA_ROW_SPACING)
            .column_spacing(preview_design::METADATA_COLUMN_SPACING)
            .halign(Align::Fill)
            .hexpand(false)
            .margin_start(preview_design::PADDING_LEFT)
            .margin_end(preview_design::PADDING_RIGHT)
            .margin_top(8)
            .margin_bottom(8)
            .build();
        metadata_grid.set_size_request(content_width, -1);
        metadata_grid.set_visible(false);
        container.append(&metadata_grid);

        let actions_container = gtk4::Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(preview_design::ACTIONS_SPACING)
            .halign(Align::End)
            .margin_start(preview_design::PADDING_LEFT)
            .margin_end(preview_design::PADDING_RIGHT)
            .margin_top(4)
            .margin_bottom(preview_design::PADDING_BOTTOM)
            .build();
        actions_container.set_visible(false);
        container.append(&actions_container);

        let on_pin: PinCallback = Rc::new(RefCell::new(None));
        let current_preview: Rc<RefCell<Option<PreviewData>>> = Rc::new(RefCell::new(None));
        let current_item_id: Rc<RefCell<String>> = Rc::new(RefCell::new(String::new()));

        // Connect pin button click
        {
            let on_pin = on_pin.clone();
            let current_item_id = current_item_id.clone();
            let current_preview = current_preview.clone();
            let title_label = title_label.clone();
            pin_gesture.connect_released(move |_, _, _, _| {
                let item_id = current_item_id.borrow().clone();
                let preview = current_preview.borrow().clone();
                if let Some(preview) = preview {
                    let title = {
                        let text = title_label.text();
                        if text.is_empty() {
                            None
                        } else {
                            Some(text.to_string())
                        }
                    };
                    if let Some(ref cb) = *on_pin.borrow() {
                        cb(&item_id, title, &preview);
                    }
                }
            });
        }

        Self {
            container,
            scrolled,
            title_label,
            pin_button,
            image_container,
            content_box,
            content_label,
            markdown_view,
            metadata_grid,
            actions_container,
            on_action: Rc::new(RefCell::new(None)),
            on_pin,
            current_item_id,
            current_preview,
        }
    }

    /// Set the preview data to display.
    pub fn set_preview(&self, item_id: &str, preview: &PreviewData) {
        *self.current_item_id.borrow_mut() = item_id.to_string();
        *self.current_preview.borrow_mut() = Some(preview.clone());

        // Title
        if let Some(title) = &preview.title {
            self.title_label.set_text(title);
            self.title_label.set_visible(true);
        } else {
            self.title_label.set_visible(false);
        }

        // Image - load and constrain to max dimensions
        let content_width =
            preview_design::WIDTH - preview_design::PADDING_LEFT - preview_design::PADDING_RIGHT;
        let image_max_width = content_width - 24;
        let image_max_height = preview_design::IMAGE_MAX_HEIGHT;

        let has_image = if let Some(image_path) = &preview.image {
            self.load_preview_image(image_path, image_max_width, image_max_height);
            true
        } else {
            // Clear image container
            while let Some(child) = self.image_container.first_child() {
                self.image_container.remove(&child);
            }
            self.image_container.set_visible(false);
            false
        };

        // Content (prefer markdown over plain text)
        // Also detect if content looks like markdown
        let markdown_content = preview.markdown.as_ref().or_else(|| {
            preview
                .content
                .as_ref()
                .filter(|text| Self::looks_like_markdown(text))
        });

        if let Some(md) = markdown_content {
            // Use markdown renderer
            self.markdown_view.set_content(md);
            self.markdown_view.widget().set_visible(true);
            self.scrolled.set_visible(false);
            self.content_box.set_visible(true);
            self.content_box.remove_css_class("image-only");
        } else if let Some(text) = &preview.content {
            // Use plain text label
            self.content_label.set_text(text);
            self.scrolled.set_visible(true);
            self.markdown_view.widget().set_visible(false);
            self.content_box.set_visible(true);
            self.content_box.remove_css_class("image-only");
        } else if has_image {
            // Image only - show content box for the image (no padding)
            self.content_label.set_text("");
            self.markdown_view.clear();
            self.scrolled.set_visible(false);
            self.markdown_view.widget().set_visible(false);
            self.content_box.set_visible(true);
            self.content_box.add_css_class("image-only");
        } else {
            self.content_label.set_text("");
            self.markdown_view.clear();
            self.scrolled.set_visible(false);
            self.markdown_view.widget().set_visible(false);
            self.content_box.set_visible(false);
            self.content_box.remove_css_class("image-only");
        }

        // Metadata
        self.render_metadata(&preview.metadata);

        // Actions
        self.render_actions(&preview.actions);
    }

    /// Clear the preview panel.
    pub fn clear(&self) {
        *self.current_item_id.borrow_mut() = String::new();
        *self.current_preview.borrow_mut() = None;

        self.title_label.set_visible(false);
        self.image_container.set_visible(false);
        self.content_label.set_text("");
        self.markdown_view.clear();
        self.markdown_view.widget().set_visible(false);
        self.scrolled.set_visible(false);
        self.content_box.set_visible(false);
        self.content_box.remove_css_class("image-only");

        // Clear metadata grid
        while let Some(child) = self.metadata_grid.first_child() {
            self.metadata_grid.remove(&child);
        }
        self.metadata_grid.set_visible(false);

        // Clear actions
        while let Some(child) = self.actions_container.first_child() {
            self.actions_container.remove(&child);
        }
        self.actions_container.set_visible(false);
    }

    /// Connect a callback for action button clicks.
    /// Callback receives (`item_id`, `action_id`).
    pub fn connect_action<F>(&self, callback: F)
    where
        F: Fn(&str, &str) + 'static,
    {
        *self.on_action.borrow_mut() = Some(Box::new(callback));
    }

    /// Connect a callback for pin button clicks.
    /// Callback receives (`item_id`, `title`, `preview_data`).
    pub fn connect_pin<F>(&self, callback: F)
    where
        F: Fn(&str, Option<String>, &PreviewData) + 'static,
    {
        *self.on_pin.borrow_mut() = Some(Box::new(callback));
    }

    /// Get the root widget for embedding.
    pub fn widget(&self) -> &gtk4::Box {
        &self.container
    }

    /// Set the maximum height for the preview panel.
    pub fn set_max_height(&self, height: i32) {
        // Set max content height on scrollable areas
        self.scrolled.set_max_content_height(height - 150); // Account for header/metadata/actions
        self.markdown_view
            .widget()
            .set_max_content_height(height - 150);
    }

    /// Set the width of the preview panel.
    #[allow(dead_code)]
    pub fn set_width(&self, width: i32) {
        self.container.set_width_request(width);
    }

    /// Check if text content looks like markdown.
    fn looks_like_markdown(text: &str) -> bool {
        let lines: Vec<&str> = text.lines().take(20).collect();
        let mut markdown_indicators = 0;

        for line in &lines {
            let trimmed = line.trim();
            // Headers
            if trimmed.starts_with('#') {
                markdown_indicators += 1;
            }
            // List items
            if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
                markdown_indicators += 1;
            }
            // Numbered lists
            if trimmed.len() > 2
                && trimmed.chars().next().is_some_and(|c| c.is_ascii_digit())
                && trimmed.chars().nth(1) == Some('.')
            {
                markdown_indicators += 1;
            }
            // Bold/italic
            if trimmed.contains("**") || trimmed.contains("__") {
                markdown_indicators += 1;
            }
            // Code blocks
            if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
                markdown_indicators += 1;
            }
            // Links
            if trimmed.contains("](") && trimmed.contains('[') {
                markdown_indicators += 1;
            }
            // Horizontal rule
            if trimmed == "---" || trimmed == "***" || trimmed == "___" {
                markdown_indicators += 1;
            }
        }

        // Consider it markdown if we find at least 2 indicators
        markdown_indicators >= 2
    }

    /// Calculate dimensions that fit within max bounds while preserving aspect ratio
    // Dimension math uses f64 for precision, GTK requires i32 for sizes
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    fn calculate_preview_dimensions(
        width: i32,
        height: i32,
        max_width: i32,
        max_height: i32,
    ) -> (i32, i32) {
        if width <= max_width && height <= max_height {
            return (width, height);
        }
        let scale_w = f64::from(max_width) / f64::from(width);
        let scale_h = f64::from(max_height) / f64::from(height);
        let scale = scale_w.min(scale_h).min(1.0);
        (
            (f64::from(width) * scale) as i32,
            (f64::from(height) * scale) as i32,
        )
    }

    fn load_preview_image(&self, image_path: &str, max_width: i32, max_height: i32) {
        let path = Path::new(image_path);

        // Check if this is an image file that should use the preview cache
        let is_image_file = path.extension().is_some_and(|ext| {
            ext.eq_ignore_ascii_case("png")
                || ext.eq_ignore_ascii_case("jpg")
                || ext.eq_ignore_ascii_case("jpeg")
                || ext.eq_ignore_ascii_case("gif")
                || ext.eq_ignore_ascii_case("webp")
                || ext.eq_ignore_ascii_case("bmp")
                || ext.eq_ignore_ascii_case("tiff")
                || ext.eq_ignore_ascii_case("tif")
        });

        if is_image_file {
            let cache = preview_cache();
            let path_buf = path.to_path_buf();

            // Create weak reference for the async callback
            let container_weak: gtk4::glib::SendWeakRef<gtk4::Box> =
                gtk4::glib::SendWeakRef::from(self.image_container.downgrade());

            if let Some(cached_path) = cache.get(&path_buf, move |generated_path| {
                // Image ready - update on main thread
                if let Some(container) = container_weak.upgrade() {
                    container.remove_css_class("preview-skeleton");

                    let file = gio::File::for_path(&generated_path);
                    if let Ok(texture) = gdk::Texture::from_file(&file) {
                        let (img_w, img_h) = Self::calculate_preview_dimensions(
                            texture.width(),
                            texture.height(),
                            max_width,
                            max_height,
                        );

                        // Clear previous content
                        while let Some(child) = container.first_child() {
                            container.remove(&child);
                        }

                        // Create DrawingArea with exact dimensions
                        let drawing_area = Self::create_image_drawing_area(texture, img_w, img_h);
                        container.append(&drawing_area);
                        container.set_visible(true);
                    }
                }
            }) {
                self.display_preview_image(&cached_path, max_width, max_height);
            } else {
                self.show_preview_skeleton(max_width, max_height);
            }
        } else {
            self.display_preview_image(path, max_width, max_height);
        }
    }

    fn show_preview_skeleton(&self, max_width: i32, max_height: i32) {
        // Clear previous content
        while let Some(child) = self.image_container.first_child() {
            self.image_container.remove(&child);
        }

        let skeleton_size = max_height.min(max_width).min(150);
        self.image_container
            .set_size_request(skeleton_size, skeleton_size);
        self.image_container.add_css_class("preview-skeleton");
        self.image_container.set_visible(true);
    }

    fn display_preview_image(&self, path: &Path, max_width: i32, max_height: i32) {
        self.image_container.remove_css_class("preview-skeleton");

        // Clear previous content
        while let Some(child) = self.image_container.first_child() {
            self.image_container.remove(&child);
        }

        let file = gio::File::for_path(path);
        if let Ok(texture) = gdk::Texture::from_file(&file) {
            let (img_w, img_h) = Self::calculate_preview_dimensions(
                texture.width(),
                texture.height(),
                max_width,
                max_height,
            );

            let drawing_area = Self::create_image_drawing_area(texture, img_w, img_h);
            self.image_container.append(&drawing_area);
            self.image_container.set_visible(true);
        } else {
            // Failed to load - hide the image
            self.image_container.set_visible(false);
        }
    }

    /// Create a `DrawingArea` that renders the texture at exact dimensions with rounded corners
    fn create_image_drawing_area(
        texture: gdk::Texture,
        width: i32,
        height: i32,
    ) -> gtk4::DrawingArea {
        let texture = Arc::new(texture);
        let corner_radius = 6.0; // Same as image_radius in CSS

        let drawing_area = gtk4::DrawingArea::builder()
            .content_width(width)
            .content_height(height)
            .css_classes(["preview-image"])
            .build();

        drawing_area.set_draw_func(move |_area, cr, w, h| {
            Self::draw_preview_image(cr, w, h, &texture, corner_radius);
        });

        drawing_area
    }

    /// Draw preview image with rounded corners
    // Cairo drawing uses f64, texture dimensions are i32, values bounded by widget size
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss,
        clippy::cast_possible_wrap
    )]
    fn draw_preview_image(
        cr: &gtk4::cairo::Context,
        width: i32,
        height: i32,
        texture: &gdk::Texture,
        corner_radius: f64,
    ) {
        use std::f64::consts::{FRAC_PI_2, PI};

        let w = f64::from(width);
        let h = f64::from(height);
        let r = corner_radius.min(w / 2.0).min(h / 2.0);

        // Rounded rectangle clip path
        cr.new_path();
        cr.arc(w - r, r, r, -FRAC_PI_2, 0.0);
        cr.arc(w - r, h - r, r, 0.0, FRAC_PI_2);
        cr.arc(r, h - r, r, FRAC_PI_2, PI);
        cr.arc(r, r, r, PI, 3.0 * FRAC_PI_2);
        cr.close_path();
        cr.clip();

        let tex_w = f64::from(texture.width());
        let tex_h = f64::from(texture.height());
        let scale = (w / tex_w).min(h / tex_h);
        let scaled_w = tex_w * scale;
        let scaled_h = tex_h * scale;
        let offset_x = (w - scaled_w) / 2.0;
        let offset_y = (h - scaled_h) / 2.0;

        // Download texture to Cairo surface (already in ARGB32 format)
        let tex_width = texture.width() as usize;
        let tex_height = texture.height() as usize;
        let stride = tex_width * 4;
        let mut data = vec![0u8; stride * tex_height];
        texture.download(&mut data, stride);

        if let Ok(surface) = gtk4::cairo::ImageSurface::create_for_data(
            data,
            gtk4::cairo::Format::ARgb32,
            tex_width as i32,
            tex_height as i32,
            stride as i32,
        ) {
            cr.translate(offset_x, offset_y);
            cr.scale(scale, scale);
            let _ = cr.set_source_surface(&surface, 0.0, 0.0);
            let _ = cr.paint();
        }
    }

    // Grid row index is usize, GTK attach requires i32
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    fn render_metadata(&self, metadata: &[hamr_types::MetadataItem]) {
        // Clear previous metadata
        while let Some(child) = self.metadata_grid.first_child() {
            self.metadata_grid.remove(&child);
        }

        if metadata.is_empty() {
            self.metadata_grid.set_visible(false);
            return;
        }

        self.metadata_grid.set_visible(true);

        for (i, item) in metadata.iter().enumerate() {
            let row = i as i32;

            // Icon (if present)
            let col_offset = if let Some(icon) = &item.icon {
                let icon_label = Label::builder()
                    .label(icon)
                    .css_classes(["metadata-icon", "material-icon"])
                    .halign(Align::Start)
                    .valign(Align::Center)
                    .build();
                self.metadata_grid.attach(&icon_label, 0, row, 1, 1);
                1
            } else {
                0
            };

            // Label
            let label = Label::builder()
                .label(format!("{}:", item.label))
                .css_classes(["metadata-label"])
                .halign(Align::Start)
                .valign(Align::Start)
                .build();
            self.metadata_grid.attach(&label, col_offset, row, 1, 1);

            // Value
            let value = Label::builder()
                .label(&item.value)
                .css_classes(["metadata-value"])
                .halign(Align::Start)
                .valign(Align::Start)
                .wrap(true)
                .wrap_mode(gtk4::pango::WrapMode::Word)
                .hexpand(true)
                .build();
            self.metadata_grid.attach(&value, col_offset + 1, row, 1, 1);
        }
    }

    fn render_actions(&self, actions: &[Action]) {
        // Clear previous actions
        while let Some(child) = self.actions_container.first_child() {
            self.actions_container.remove(&child);
        }

        if actions.is_empty() {
            self.actions_container.set_visible(false);
            return;
        }

        self.actions_container.set_visible(true);

        for action in actions {
            let button = RippleButton::from_action(action, None);

            let on_action = self.on_action.clone();
            let item_id = self.current_item_id.clone();
            button.connect_clicked(move |action_id| {
                let id = item_id.borrow().clone();
                tracing::debug!("Preview action clicked: {} (item: {})", action_id, id);
                if let Some(ref cb) = *on_action.borrow() {
                    cb(&id, action_id);
                }
            });

            self.actions_container.append(button.widget());
        }
    }
}

impl Default for PreviewPanel {
    fn default() -> Self {
        Self::new()
    }
}

/// CSS styles for preview panel
// CSS template - splitting would scatter related style rules
#[allow(clippy::too_many_lines)]
pub fn preview_panel_css(theme: &crate::config::Theme) -> String {
    let colors = &theme.colors;
    let fonts = &theme.config.fonts;

    // Scaled dimension values
    let pin_padding = theme.scaled(design::spacing::XS); // 4
    let pin_radius = theme.scaled(design::radius::XS); // 4
    let content_border = theme.scaled(1);
    let content_radius = theme.scaled(design::radius::MD); // 12
    let content_padding_v = theme.scaled(design::spacing::SM + design::spacing::XXXS); // 10 (8+2)
    let content_padding_h = theme.scaled(design::spacing::MD); // 12
    let image_radius = theme.scaled(design::radius::SM - design::spacing::XXXS); // 6
    let code_margin = theme.scaled(design::spacing::XS); // 4
    let code_padding = theme.scaled(design::spacing::SM); // 8
    let code_line_margin = theme.scaled(7); // -7px negative margin for tight line spacing
    let code_header_padding_v = theme.scaled(design::spacing::XS); // 4
    let metadata_icon_margin = theme.scaled(design::spacing::XS); // 4
    let action_padding_v = theme.scaled(design::spacing::SM - design::spacing::XXXS); // 6
    let action_padding_h = theme.scaled(design::spacing::MD); // 12
    let action_radius = theme.scaled(design::radius::SM - design::spacing::XXXS); // 6

    format!(
        r#"
        .preview-title {{
            font-family: "{font_main}";
            font-size: {font_normal}px;
            font-weight: 600;
            color: {on_surface};
        }}

        .preview-pin-button {{
            padding: {pin_padding}px;
            border-radius: {pin_radius}px;
        }}

        .preview-pin-button:hover {{
            background: {surface_container_highest};
        }}

        .preview-pin-icon {{
            font-size: {font_small}px;
            color: {outline};
        }}

        .preview-pin-button:hover .preview-pin-icon {{
            color: {on_surface};
        }}

        .preview-content-box {{
            background: linear-gradient(to bottom, rgba(149, 144, 136, 0.08), {surface});
            background-color: {surface};
            border: {content_border}px solid alpha({outline}, 0.28);
            border-radius: {content_radius}px;
            padding: {content_padding_v}px {content_padding_h}px;
            font-size: {font_smaller}px;
        }}

        /* When showing image only, remove padding */
        .preview-content-box.image-only {{
            padding: 0;
        }}

        .preview-content-box * {{
            font-size: inherit;
        }}

        .preview-image-container {{
            border-radius: {image_radius}px;
        }}

        .preview-image-container.preview-skeleton {{
            background: linear-gradient(
                90deg,
                {surface_container} 0%,
                {surface_container_high} 50%,
                {surface_container} 100%
            );
            background-size: 200% 100%;
            animation: shimmer 1.5s ease-in-out infinite;
        }}

        @keyframes shimmer {{
            0% {{ background-position: 200% 0; }}
            100% {{ background-position: -200% 0; }}
        }}

        .preview-image {{
            border-radius: {image_radius}px;
        }}

        .preview-content {{
            font-family: "{font_mono}";
            font-size: {font_smaller}px;
            color: {on_surface};
        }}

        .preview-markdown {{
            color: {on_surface};
        }}

        /* Tighter spacing for code blocks in preview */
        .preview-content-box .markdown-code-block {{
            margin-top: {code_margin}px;
            margin-bottom: {code_margin}px;
        }}

        .preview-content-box .markdown-code-content {{
            padding: {code_padding}px;
        }}

        .preview-content-box .markdown-code-content label.markdown-code-line {{
            padding: 0;
            margin-top: -{code_line_margin}px;
            margin-bottom: -{code_line_margin}px;
            min-height: 0;
        }}

        .preview-content-box .markdown-code-header {{
            padding: {code_header_padding_v}px {code_padding}px;
        }}

        .metadata-icon {{
            font-size: {font_smaller}px;
            color: {outline};
            margin-right: {metadata_icon_margin}px;
        }}

        .metadata-label {{
            font-family: "{font_main}";
            font-size: {font_smaller}px;
            color: {outline};
        }}

        .metadata-value {{
            font-family: "{font_main}";
            font-size: {font_smaller}px;
            color: {on_surface_variant};
        }}

        .preview-action-button {{
            padding: {action_padding_v}px {action_padding_h}px;
            border-radius: {action_radius}px;
            font-family: "{font_main}";
            font-size: {font_smaller}px;
        }}

        .preview-action-primary {{
            background: {primary_container};
            color: {on_primary_container};
        }}

        .preview-action-primary:hover {{
            background: lighter({primary_container});
        }}

        .preview-action-secondary {{
            background: {secondary_container};
            color: {on_secondary_container};
        }}

        .preview-action-secondary:hover {{
            background: lighter({secondary_container});
        }}
        "#,
        on_surface = colors.on_surface,
        on_surface_variant = colors.on_surface_variant,
        outline = colors.outline,
        surface = colors.surface,
        surface_container = colors.surface_container,
        surface_container_high = colors.surface_container_high,
        surface_container_highest = colors.surface_container_highest,
        primary_container = colors.primary_container,
        on_primary_container = colors.on_primary_container,
        secondary_container = colors.secondary_container,
        on_secondary_container = colors.on_secondary_container,
        font_main = fonts.main,
        font_mono = fonts.monospace,
        font_normal = theme.scaled_font(design::font::LG),
        font_small = theme.scaled_font(design::font::MD),
        font_smaller = theme.scaled_font(design::font::SM),
    )
}
