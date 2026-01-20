//! Gauge widget for GTK4.
//!
//! A circular arc gauge (like a speedometer) matching the QML design.
//! - 270° arc from bottom-left (135°) to bottom-right (405°)
//! - Background track in `surface_container_high`
//! - Foreground progress in `primary` (or custom color)
//! - Optional centered label

use crate::colors::Colors;
use crate::config::Theme;

use gtk4::DrawingArea;
use gtk4::cairo;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::subclass::prelude::*;
use hamr_types::GaugeData;
use std::cell::{Cell, RefCell};
use std::f64::consts::PI;

use super::design;

mod imp {
    use super::{
        Cell, DrawingArea, DrawingAreaExt, DrawingAreaExtManual, DrawingAreaImpl, ObjectImpl,
        ObjectImplExt, ObjectSubclass, ObjectSubclassExt, RefCell, WidgetImpl, design, glib,
    };

    #[derive(Debug, Default)]
    pub struct GaugeWidgetInner {
        pub value: Cell<f64>,
        pub min: Cell<f64>,
        pub max: Cell<f64>,
        pub label: RefCell<Option<String>>,
        pub color: RefCell<Option<String>>,
        pub bg_color: RefCell<String>,
        pub fg_color: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for GaugeWidgetInner {
        const NAME: &'static str = "HamrGaugeWidget";
        type Type = super::GaugeWidget;
        type ParentType = DrawingArea;
    }

    impl ObjectImpl for GaugeWidgetInner {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();
            obj.set_content_width(design::gauge::SIZE);
            obj.set_content_height(design::gauge::SIZE);

            obj.set_draw_func(glib::clone!(
                #[weak]
                obj,
                move |_area, cr, width, height| {
                    obj.draw(cr, width, height);
                }
            ));
        }
    }

    impl WidgetImpl for GaugeWidgetInner {}
    impl DrawingAreaImpl for GaugeWidgetInner {}
}

glib::wrapper! {
    pub struct GaugeWidget(ObjectSubclass<imp::GaugeWidgetInner>)
        @extends DrawingArea, gtk4::Widget,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget;
}

impl Default for GaugeWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl GaugeWidget {
    /// Create a new gauge widget
    pub fn new() -> Self {
        let obj: Self = glib::Object::builder().build();

        let colors = Colors::default();
        obj.imp()
            .bg_color
            .replace(colors.surface_container_high.clone());
        obj.imp().fg_color.replace(colors.primary.clone());

        obj.imp().min.set(0.0);
        obj.imp().max.set(100.0);

        obj
    }

    /// Create from `GaugeData`
    pub fn from_data(data: &GaugeData, colors: &Colors) -> Self {
        let obj = Self::new();
        obj.set_data(data, colors);
        obj
    }

    /// Set gauge data
    pub fn set_data(&self, data: &GaugeData, colors: &Colors) {
        self.imp().value.set(data.value);
        self.imp().min.set(data.min);
        self.imp().max.set(data.max);
        self.imp().label.replace(data.label.clone());
        self.imp().color.replace(data.color.clone());

        self.imp()
            .bg_color
            .replace(colors.surface_container_high.clone());
        self.imp()
            .fg_color
            .replace(data.color.clone().unwrap_or_else(|| colors.primary.clone()));

        self.queue_draw();
    }

    /// Set the gauge size (width and height)
    pub fn set_size(&self, size: i32) {
        self.set_content_width(size);
        self.set_content_height(size);
    }

    /// Calculate the ratio (0.0 to 1.0)
    fn ratio(&self) -> f64 {
        let value = self.imp().value.get();
        let min = self.imp().min.get();
        let max = self.imp().max.get();

        if max > min {
            ((value - min) / (max - min)).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    /// Draw the gauge
    fn draw(&self, cr: &cairo::Context, width: i32, height: i32) {
        let width = f64::from(width);
        let height = f64::from(height);

        let stroke_width = design::gauge::STROKE_WIDTH;
        let center_x = width / 2.0;
        let center_y = height / 2.0;
        let radius = (width.min(height) - stroke_width) / 2.0;

        // Convert angles to radians (Cairo uses radians, 0° is at 3 o'clock)
        let start_angle = design::gauge::START_ANGLE * PI / 180.0;
        let sweep_angle = design::gauge::SWEEP_ANGLE * PI / 180.0;
        let end_angle = start_angle + sweep_angle;

        let bg_color = parse_color(&self.imp().bg_color.borrow());
        let fg_color = parse_color(&self.imp().fg_color.borrow());
        let label_color = parse_color("#cbc5ca"); // on_surface_variant

        cr.set_line_width(stroke_width);
        cr.set_line_cap(cairo::LineCap::Round);
        cr.set_source_rgb(bg_color.0, bg_color.1, bg_color.2);
        cr.arc(center_x, center_y, radius, start_angle, end_angle);
        let _ = cr.stroke();

        let ratio = self.ratio();
        if ratio > 0.0 {
            let progress_end = start_angle + sweep_angle * ratio.max(0.001);
            cr.set_source_rgb(fg_color.0, fg_color.1, fg_color.2);
            cr.arc(center_x, center_y, radius, start_angle, progress_end);
            let _ = cr.stroke();
        }

        if let Some(label) = self.imp().label.borrow().as_ref()
            && !label.is_empty()
        {
            cr.set_source_rgb(label_color.0, label_color.1, label_color.2);
            cr.select_font_face("Sans", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
            cr.set_font_size(design::gauge::LABEL_FONT_SIZE);

            let extents = cr.text_extents(label).unwrap();
            let x = center_x - extents.width() / 2.0 - extents.x_bearing();
            let y = center_y - extents.height() / 2.0 - extents.y_bearing();

            cr.move_to(x, y);
            let _ = cr.show_text(label);
        }
    }
}

/// Parse a hex color string to RGB values (0.0-1.0)
fn parse_color(color: &str) -> (f64, f64, f64) {
    let color = color.trim_start_matches('#');
    if color.len() >= 6 {
        let r = f64::from(u8::from_str_radix(&color[0..2], 16).unwrap_or(128)) / 255.0;
        let g = f64::from(u8::from_str_radix(&color[2..4], 16).unwrap_or(128)) / 255.0;
        let b = f64::from(u8::from_str_radix(&color[4..6], 16).unwrap_or(128)) / 255.0;
        (r, g, b)
    } else {
        (0.5, 0.5, 0.5)
    }
}

/// Generate CSS for gauge widget (minimal - most styling is in draw function)
pub fn gauge_css(theme: &Theme) -> String {
    let size = theme.scaled(design::icon::CONTAINER_SIZE);

    format!(
        r"
        .gauge {{
            min-width: {size}px;
            min-height: {size}px;
        }}
        ",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_color() {
        let (r, g, b) = parse_color("#FF0000");
        assert!((r - 1.0).abs() < 0.01);
        assert!(g.abs() < 0.01);
        assert!(b.abs() < 0.01);

        let (r, g, b) = parse_color("#00FF00");
        assert!(r.abs() < 0.01);
        assert!((g - 1.0).abs() < 0.01);
        assert!(b.abs() < 0.01);

        let (r, g, b) = parse_color("0000FF");
        assert!(r.abs() < 0.01);
        assert!(g.abs() < 0.01);
        assert!((b - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_gauge_design_constants() {
        assert_eq!(design::gauge::SIZE, 40);
        assert!((design::gauge::STROKE_WIDTH - 4.0).abs() < 0.01);
        assert!((design::gauge::START_ANGLE - 135.0).abs() < 0.01);
        assert!((design::gauge::SWEEP_ANGLE - 270.0).abs() < 0.01);
    }
}
