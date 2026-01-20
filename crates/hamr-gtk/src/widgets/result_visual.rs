//! Shared visual widget for search result items.
//!
//! Renders the visual representation of a search result, which can be:
//! - System icon (from icon theme or file path)
//! - Material icon
//! - Text/emoji
//! - Thumbnail image (with lazy background caching)
//! - Gauge widget
//! - Graph widget
//! - Initial avatar (fallback)

use crate::config::Theme;
use crate::thumbnail_cache::thumbnail_cache;

use super::design::{font, icon, radius, spacing};
use super::gauge::GaugeWidget;
use super::graph::GraphWidget;
use gtk4::gdk;
use gtk4::gio;
use gtk4::glib::object::ObjectExt;
use gtk4::prelude::*;
use gtk4::{Align, CenterBox};
use hamr_rpc::SearchResult;
use hamr_types::{GaugeData, GraphData, IconSpec, WidgetData};
use std::path::Path;
use std::sync::Arc;

/// Size preset for the visual (base values, apply `theme.scaled()` for actual sizes)
#[derive(Debug, Clone, Copy)]
pub enum VisualSize {
    /// Small size for list items (40px container, 32px icon)
    Small,
    /// Large size for grid items (80px container, 64px icon)
    #[allow(dead_code)]
    Large,
    /// Thumbnail size for image browsers (120px container, fills space)
    Thumbnail,
}

impl VisualSize {
    /// Base container size (unscaled)
    pub fn container_size_base(self) -> i32 {
        match self {
            // 40 = icon::CONTAINER_SIZE
            VisualSize::Small => icon::CONTAINER_SIZE,
            // 80 = 2 * icon::CONTAINER_SIZE
            VisualSize::Large => icon::CONTAINER_SIZE * 2,
            // 120 = 3 * icon::CONTAINER_SIZE
            VisualSize::Thumbnail => icon::CONTAINER_SIZE * 3,
        }
    }

    /// Scaled container size
    pub fn container_size(self, theme: &Theme) -> i32 {
        theme.scaled(self.container_size_base())
    }

    /// Base icon size (unscaled)
    pub fn icon_size_base(self) -> i32 {
        match self {
            // 32 = icon::XL
            VisualSize::Small => icon::XL,
            // 64 = 2 * icon::XL
            VisualSize::Large => icon::XL * 2,
            // 80 = 2 * icon::CONTAINER_SIZE (larger but not filling entire container)
            VisualSize::Thumbnail => icon::CONTAINER_SIZE * 2,
        }
    }

    /// Scaled icon size
    pub fn icon_size(self, theme: &Theme) -> i32 {
        theme.scaled(self.icon_size_base())
    }

    /// Avatar font size (scaled) - used for initial fallback
    fn avatar_font_size(self, theme: &Theme) -> i32 {
        let base = match self {
            // 14 -> nearest token font::LG (15)
            VisualSize::Small => font::LG,
            // 28 -> font::XXL * 1.4 (20 * 1.4 = 28)
            VisualSize::Large => font::XXL + spacing::SM,
            // 36 -> font::XXL + spacing::LG (20 + 16 = 36)
            VisualSize::Thumbnail => font::XXL + spacing::LG,
        };
        theme.scaled_font(base)
    }

    /// Material icon font size (scaled)
    fn material_font_size(self, theme: &Theme) -> i32 {
        let base = match self {
            // 26 = icon::MATERIAL_SIZE
            VisualSize::Small => icon::MATERIAL_SIZE,
            // 48 = icon::XXL
            VisualSize::Large => icon::XXL,
            // 56 = icon::XXL + spacing::SM (48 + 8 = 56) - balanced size for grid
            VisualSize::Thumbnail => icon::XXL + spacing::SM,
        };
        theme.scaled_font(base)
    }

    /// Emoji/text font size (scaled)
    fn emoji_font_size(self, theme: &Theme) -> i32 {
        let base = match self {
            // 20 = icon::MD
            VisualSize::Small => icon::MD,
            // 48 = icon::XXL
            VisualSize::Large => icon::XXL,
            // 56 = icon::XXL + spacing::SM (48 + 8 = 56) - balanced size for grid
            VisualSize::Thumbnail => icon::XXL + spacing::SM,
        };
        theme.scaled_font(base)
    }
}

/// The rendered visual widget with optional special widgets that need updating
pub struct ResultVisual {
    container: CenterBox,
    gauge: Option<GaugeWidget>,
    graph: Option<GraphWidget>,
    material_icon_label: Option<gtk4::Label>,
    theme: Theme,
}

impl ResultVisual {
    /// Create a new visual for the given search result
    pub fn new(result: &SearchResult, size: VisualSize) -> Self {
        let theme = Theme::load();
        let container_size = size.container_size(&theme);
        // CenterBox properly centers its child widget
        // Use Fill alignment so overlays position correctly relative to full size
        let container = CenterBox::builder()
            .css_classes(["result-visual"])
            .halign(Align::Fill)
            .valign(Align::Fill)
            .width_request(container_size)
            .height_request(container_size)
            .build();

        let mut visual = Self {
            container,
            gauge: None,
            graph: None,
            material_icon_label: None,
            theme,
        };

        visual.set_content(result, size);
        visual
    }

    /// Update the visual content
    pub fn update(&mut self, result: &SearchResult, size: VisualSize) {
        // Clear existing center widget
        self.container.set_center_widget(None::<&gtk4::Widget>);
        self.gauge = None;
        self.graph = None;
        self.material_icon_label = None;

        self.set_content(result, size);
    }

    fn set_content(&mut self, result: &SearchResult, size: VisualSize) {
        // Priority: thumbnail > gauge > graph > icon
        if let Some(thumbnail) = &result.thumbnail {
            self.set_thumbnail(thumbnail, size);
            return;
        }

        if let Some(ref widget) = result.widget {
            match widget {
                WidgetData::Gauge {
                    value,
                    min,
                    max,
                    label,
                    color,
                } => {
                    let gauge_data = GaugeData {
                        value: *value,
                        min: *min,
                        max: *max,
                        label: label.clone(),
                        color: color.clone(),
                    };
                    self.set_gauge(&gauge_data, size);
                    return;
                }
                WidgetData::Graph { data, min, max } => {
                    let graph_data = GraphData {
                        data: data.clone(),
                        min: *min,
                        max: *max,
                    };
                    self.set_graph(&graph_data, size);
                    return;
                }
                _ => {}
            }
        }

        self.set_icon(result, size);
    }

    fn set_thumbnail(&mut self, path: &str, size: VisualSize) {
        let container_size = size.container_size(&self.theme);
        let path_buf = Path::new(path).to_path_buf();

        // Check if this is an image file that needs caching
        let is_image_file = is_image_path(path);

        // For image files, use the thumbnail cache
        if is_image_file {
            let cache = thumbnail_cache();

            // Use SendWeakRef which can be sent across threads safely
            let container_weak: gtk4::glib::SendWeakRef<CenterBox> =
                gtk4::glib::SendWeakRef::from(self.container.downgrade());

            // Try to get cached thumbnail, or queue generation
            let thumb_radius = f64::from(self.theme.scaled(radius::SM - spacing::XXXS));

            if let Some(thumb_path) = cache.get(&path_buf, move |generated_path| {
                // Thumbnail generated - update the widget on main thread
                if let Some(container) = container_weak.upgrade() {
                    let file = gio::File::for_path(&generated_path);
                    if let Ok(texture) = gdk::Texture::from_file(&file) {
                        let texture = Arc::new(texture);

                        let drawing_area = gtk4::DrawingArea::builder()
                            .content_width(container_size)
                            .content_height(container_size)
                            .css_classes(["result-visual-thumbnail"])
                            .build();

                        drawing_area.set_draw_func(move |_area, cr, width, height| {
                            draw_thumbnail_rounded(cr, width, height, &texture, thumb_radius);
                        });

                        container.set_center_widget(Some(&drawing_area));
                    }
                }
            }) {
                self.display_thumbnail(&thumb_path, container_size);
            } else {
                self.show_skeleton(container_size);
            }
        } else {
            self.display_thumbnail(&path_buf, container_size);
        }
    }

    fn display_thumbnail(&mut self, path: &Path, container_size: i32) {
        let file = gio::File::for_path(path);
        if let Ok(texture) = gdk::Texture::from_file(&file) {
            let thumb_radius = f64::from(self.theme.scaled(radius::SM - spacing::XXXS));
            let texture = Arc::new(texture);

            // Use DrawingArea for exact size control with rounded corners
            let drawing_area = gtk4::DrawingArea::builder()
                .content_width(container_size)
                .content_height(container_size)
                .css_classes(["result-visual-thumbnail"])
                .build();

            drawing_area.set_draw_func(move |_area, cr, width, height| {
                draw_thumbnail_rounded(cr, width, height, &texture, thumb_radius);
            });

            self.container.set_center_widget(Some(&drawing_area));
        } else {
            // Fallback to image icon if thumbnail load fails
            let label = gtk4::Label::builder()
                .label("image")
                .css_classes(["result-visual-material", "material-icon"])
                .halign(Align::Center)
                .valign(Align::Center)
                .build();
            apply_font_size(&label, VisualSize::Small.material_font_size(&self.theme));
            self.container.set_center_widget(Some(&label));
        }
    }

    fn show_skeleton(&mut self, container_size: i32) {
        let skeleton = gtk4::Frame::builder()
            .width_request(container_size)
            .height_request(container_size)
            .css_classes(["thumbnail-skeleton"])
            .build();
        self.container.set_center_widget(Some(&skeleton));
    }

    fn set_gauge(&mut self, data: &hamr_rpc::GaugeData, size: VisualSize) {
        let gauge_widget = GaugeWidget::from_data(data, &self.theme.colors);
        // For Large/Thumbnail sizes, scale up the gauge slightly but not to full container
        let gauge_size = match size {
            VisualSize::Small => self.theme.scaled(icon::CONTAINER_SIZE), // 40
            VisualSize::Large => self.theme.scaled(icon::CONTAINER_SIZE + spacing::LG), // 56
            VisualSize::Thumbnail => self.theme.scaled(icon::CONTAINER_SIZE * 2), // 80
        };
        gauge_widget.set_size(gauge_size);
        self.container.set_center_widget(Some(&gauge_widget));
        self.gauge = Some(gauge_widget);
    }

    fn set_graph(&mut self, data: &hamr_rpc::GraphData, size: VisualSize) {
        let graph_widget = GraphWidget::from_data(data, &self.theme.colors);
        // For Large/Thumbnail sizes, scale up the graph slightly but not to full container
        let graph_size = match size {
            VisualSize::Small => self.theme.scaled(icon::CONTAINER_SIZE), // 40
            VisualSize::Large => self.theme.scaled(icon::CONTAINER_SIZE + spacing::LG), // 56
            VisualSize::Thumbnail => self.theme.scaled(icon::CONTAINER_SIZE * 2), // 80
        };
        graph_widget.set_size(graph_size);
        self.container.set_center_widget(Some(&graph_widget));
        self.graph = Some(graph_widget);
    }

    fn set_icon(&mut self, result: &SearchResult, size: VisualSize) {
        let icon_spec = IconSpec::from_wire(
            result.icon_or_default().to_string(),
            result.icon_type.as_deref(),
        );
        let icon_size = size.icon_size(&self.theme);

        match icon_spec {
            IconSpec::System(name) => match resolve_icon(&name) {
                ResolvedIcon::Theme(icon_name) => {
                    let image = gtk4::Image::builder()
                        .icon_name(&icon_name)
                        .pixel_size(icon_size)
                        .css_classes(["result-visual-system-icon"])
                        .build();
                    self.container.set_center_widget(Some(&image));
                }
                ResolvedIcon::Path(path) => {
                    let image = gtk4::Image::builder()
                        .file(&path)
                        .pixel_size(icon_size)
                        .css_classes(["result-visual-system-icon"])
                        .build();
                    self.container.set_center_widget(Some(&image));
                }
                ResolvedIcon::Fallback => {
                    self.set_avatar(&result.name, size);
                }
            },
            IconSpec::Material(name) => {
                let label = gtk4::Label::builder()
                    .label(&name)
                    .css_classes(["result-visual-material", "material-icon"])
                    .build();
                apply_font_size(&label, size.material_font_size(&self.theme));
                self.container.set_center_widget(Some(&label));
                self.material_icon_label = Some(label);
            }
            IconSpec::Text(text) => {
                let label = gtk4::Label::builder()
                    .label(&text)
                    .css_classes(["result-visual-text"])
                    .build();
                apply_font_size(&label, size.emoji_font_size(&self.theme));
                self.container.set_center_widget(Some(&label));
            }
            IconSpec::Path(path) => {
                if path.exists() {
                    let image = gtk4::Image::builder()
                        .file(path.to_string_lossy().as_ref())
                        .pixel_size(icon_size)
                        .css_classes(["result-visual-image"])
                        .build();
                    self.container.set_center_widget(Some(&image));
                } else {
                    self.set_avatar(&result.name, size);
                }
            }
        }
    }

    fn set_avatar(&mut self, name: &str, size: VisualSize) {
        let initial = name
            .chars()
            .next()
            .map_or_else(|| "?".to_string(), |c| c.to_uppercase().to_string());
        let icon_size = size.icon_size(&self.theme);

        let frame = gtk4::Frame::builder()
            .css_classes(["result-visual-avatar"])
            .width_request(icon_size)
            .height_request(icon_size)
            .build();

        let label = gtk4::Label::builder()
            .label(&initial)
            .css_classes(["result-visual-avatar-label"])
            .halign(Align::Center)
            .valign(Align::Center)
            .build();

        apply_font_size(&label, size.avatar_font_size(&self.theme));

        frame.set_child(Some(&label));
        self.container.set_center_widget(Some(&frame));
    }

    /// Get the widget
    pub fn widget(&self) -> &CenterBox {
        &self.container
    }

    /// Get the gauge widget if present (for updates)
    pub fn gauge(&self) -> Option<&GaugeWidget> {
        self.gauge.as_ref()
    }

    /// Get the graph widget if present (for updates)
    pub fn graph(&self) -> Option<&GraphWidget> {
        self.graph.as_ref()
    }
}

/// Draw a thumbnail with rounded corners, scaled to cover the area
// Texture dimensions are i32, stride calc uses usize, Cairo needs i32, values bounded by image size
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]
fn draw_thumbnail_rounded(
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

    // Scale to cover (crop to fill, centered)
    let tex_w = f64::from(texture.width());
    let tex_h = f64::from(texture.height());
    let scale = (w / tex_w).max(h / tex_h);
    let offset_x = (w - tex_w * scale) / 2.0;
    let offset_y = (h - tex_h * scale) / 2.0;

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

/// Check if a path looks like an image file based on extension
fn is_image_path(path: &str) -> bool {
    std::path::Path::new(path).extension().is_some_and(|ext| {
        ext.eq_ignore_ascii_case("png")
            || ext.eq_ignore_ascii_case("jpg")
            || ext.eq_ignore_ascii_case("jpeg")
            || ext.eq_ignore_ascii_case("gif")
            || ext.eq_ignore_ascii_case("webp")
            || ext.eq_ignore_ascii_case("bmp")
            || ext.eq_ignore_ascii_case("tiff")
            || ext.eq_ignore_ascii_case("tif")
    })
}

/// Apply font size to a label using Pango attributes
// Pango sizing uses f64 for conversion, then i32 for API
#[allow(clippy::cast_possible_truncation)]
fn apply_font_size(label: &gtk4::Label, size_px: i32) {
    let attrs = gtk4::pango::AttrList::new();
    // Pango uses points * PANGO_SCALE, and 1pt ≈ 1.333px, so we convert
    let size_pango = (f64::from(size_px) * 0.75 * f64::from(gtk4::pango::SCALE)) as i32;
    let size_attr = gtk4::pango::AttrSize::new(size_pango);
    attrs.insert(size_attr);
    label.set_attributes(Some(&attrs));
}

/// Result of icon resolution
pub enum ResolvedIcon {
    /// Icon found in theme
    Theme(String),
    /// Icon found at file path
    Path(String),
    /// No icon found - use fallback
    Fallback,
}

/// Resolve an icon name to a theme icon or file path.
/// Tries multiple variations similar to QML `IconResolver`.
pub fn resolve_icon(icon_name: &str) -> ResolvedIcon {
    if icon_name.starts_with('/') {
        if Path::new(icon_name).exists() {
            return ResolvedIcon::Path(icon_name.to_string());
        }
        return ResolvedIcon::Fallback;
    }

    let display = gtk4::gdk::Display::default().expect("No display");
    let icon_theme = gtk4::IconTheme::for_display(&display);

    // Try exact name first
    if icon_theme.has_icon(icon_name) {
        return ResolvedIcon::Theme(icon_name.to_string());
    }

    // Generate name variations to try (like QML IconResolver)
    let variations = generate_icon_variations(icon_name);
    for variation in &variations {
        if icon_theme.has_icon(variation) {
            return ResolvedIcon::Theme(variation.clone());
        }
    }

    // Search in icon directories
    let home = std::env::var("HOME").unwrap_or_default();
    let icon_dirs = [
        "/var/lib/flatpak/exports/share/icons/hicolor".to_string(),
        format!("{home}/.local/share/flatpak/exports/share/icons/hicolor"),
        "/usr/share/icons/hicolor".to_string(),
        "/usr/share/pixmaps".to_string(),
        format!("{home}/.local/share/icons/hicolor"),
    ];

    let sizes = [
        "scalable/apps",
        "512x512/apps",
        "256x256/apps",
        "128x128/apps",
        "96x96/apps",
        "64x64/apps",
        "48x48/apps",
        "32x32/apps",
    ];
    let extensions = ["svg", "png"];

    // Try all variations in all directories
    let all_names: Vec<&str> = std::iter::once(icon_name)
        .chain(variations.iter().map(std::string::String::as_str))
        .collect();

    for name in &all_names {
        for dir in &icon_dirs {
            for size in &sizes {
                for ext in &extensions {
                    let path = format!("{dir}/{size}/{name}.{ext}");
                    if Path::new(&path).exists() {
                        return ResolvedIcon::Path(path);
                    }
                }
            }
            // Also check pixmaps directory directly
            for ext in &extensions {
                let path = format!("{dir}/{name}.{ext}");
                if Path::new(&path).exists() {
                    return ResolvedIcon::Path(path);
                }
            }
        }
    }

    ResolvedIcon::Fallback
}

/// Generate icon name variations to try (similar to QML `IconResolver`)
fn generate_icon_variations(icon_name: &str) -> Vec<String> {
    let mut variations = Vec::new();

    // Lowercase
    let lowercased = icon_name.to_lowercase();
    if lowercased != icon_name {
        variations.push(lowercased.clone());
    }

    // Reverse domain name (org.gnome.App -> App)
    if icon_name.contains('.')
        && let Some(last) = icon_name.split('.').next_back()
    {
        variations.push(last.to_string());
        variations.push(last.to_lowercase());
    }

    // Kebab-case normalized ("My App" -> "my-app")
    let kebab = icon_name.to_lowercase().replace(' ', "-");
    if kebab != icon_name && kebab != lowercased {
        variations.push(kebab);
    }

    // Underscore to kebab ("my_app" -> "my-app")
    let underscore_to_kebab = icon_name.to_lowercase().replace('_', "-");
    if !variations.contains(&underscore_to_kebab) {
        variations.push(underscore_to_kebab);
    }

    if icon_name.starts_with("io.")
        || icon_name.starts_with("com.")
        || icon_name.starts_with("org.")
    {
        let parts: Vec<&str> = icon_name.split('.').collect();
        if let [_, _, .., app_name] = parts.as_slice() {
            if !variations.contains(&(*app_name).to_string()) {
                variations.push((*app_name).to_string());
            }
            let app_lower = app_name.to_lowercase();
            if !variations.contains(&app_lower) {
                variations.push(app_lower);
            }
        }
    }

    variations
}

/// Generate CSS for result visual styling
pub fn result_visual_css(theme: &Theme) -> String {
    let colors = &theme.colors;

    // Pre-compute scaled values
    let min_size = theme.scaled(icon::CONTAINER_SIZE); // 40
    let thumb_radius = theme.scaled(radius::SM - spacing::XXXS); // 8 - 2 = 6

    format!(
        r"
        /* Result Visual Container - fixed size with centered content */
        .result-visual {{
            background: transparent;
            min-width: {min_size}px;
            min-height: {min_size}px;
        }}

        /* Thumbnail */
        .result-visual-thumbnail {{
            border-radius: {thumb_radius}px;
        }}

        /* Thumbnail skeleton loading placeholder with shimmer animation */
        .thumbnail-skeleton {{
            border-radius: {thumb_radius}px;
            border: none;
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

        /* System Icon */
        .result-visual-system-icon {{
        }}

        /* Material Icon */
        .result-visual-material {{
            color: {on_surface_variant};
        }}

        /* Text/Emoji */
        .result-visual-text {{
        }}

        /* Image */
        .result-visual-image {{
            border-radius: {thumb_radius}px;
        }}

        /* Avatar (initial fallback) */
        .result-visual-avatar {{
            border-radius: 50%;
            background-color: {primary_container};
            border: none;
        }}

        .result-visual-avatar-label {{
            font-weight: bold;
            color: {on_primary_container};
        }}
        ",
        min_size = min_size,
        thumb_radius = thumb_radius,
        surface_container = colors.surface_container,
        surface_container_high = colors.surface_container_high,
        on_surface_variant = colors.on_surface_variant,
        primary_container = colors.primary_container,
        on_primary_container = colors.on_primary_container,
    )
}
