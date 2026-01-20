//! Graph/sparkline widget for GTK4.
//!
//! A smooth line graph matching the QML `LineGraph` design.
//! - 40x40 default size
//! - 2px padding, 2px line width
//! - Smooth quadratic curves between points
//! - `Primary` color stroke

use crate::colors::Colors;
use crate::config::Theme;

use gtk4::DrawingArea;
use gtk4::cairo;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::subclass::prelude::*;
use hamr_types::GraphData;
use std::cell::RefCell;

use super::design;

mod imp {
    use super::{
        DrawingArea, DrawingAreaExt, DrawingAreaExtManual, DrawingAreaImpl, ObjectImpl,
        ObjectImplExt, ObjectSubclass, ObjectSubclassExt, RefCell, WidgetImpl, design, glib,
    };

    #[derive(Debug, Default)]
    pub struct GraphWidgetInner {
        pub data: RefCell<Vec<f64>>,
        pub min: RefCell<Option<f64>>,
        pub max: RefCell<Option<f64>>,
        pub color: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for GraphWidgetInner {
        const NAME: &'static str = "HamrGraphWidget";
        type Type = super::GraphWidget;
        type ParentType = DrawingArea;
    }

    impl ObjectImpl for GraphWidgetInner {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();
            obj.set_content_width(design::graph::SIZE);
            obj.set_content_height(design::graph::SIZE);

            // Set up draw function
            obj.set_draw_func(glib::clone!(
                #[weak]
                obj,
                move |_area, cr, width, height| {
                    obj.draw(cr, width, height);
                }
            ));
        }
    }

    impl WidgetImpl for GraphWidgetInner {}
    impl DrawingAreaImpl for GraphWidgetInner {}
}

glib::wrapper! {
    pub struct GraphWidget(ObjectSubclass<imp::GraphWidgetInner>)
        @extends DrawingArea, gtk4::Widget,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget;
}

impl Default for GraphWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl GraphWidget {
    /// Create a new graph widget
    pub fn new() -> Self {
        let obj: Self = glib::Object::builder().build();

        // Set default color
        let colors = Colors::default();
        obj.imp().color.replace(colors.primary.clone());

        obj
    }

    /// Create from `GraphData`
    pub fn from_data(data: &GraphData, colors: &Colors) -> Self {
        let obj = Self::new();
        obj.set_data(data, colors);
        obj
    }

    /// Set graph data
    pub fn set_data(&self, data: &GraphData, colors: &Colors) {
        self.imp().data.replace(data.data.clone());
        self.imp().min.replace(data.min);
        self.imp().max.replace(data.max);
        self.imp().color.replace(colors.primary.clone());
        self.queue_draw();
    }

    /// Set the graph size (width and height)
    pub fn set_size(&self, size: i32) {
        self.set_content_width(size);
        self.set_content_height(size);
    }

    /// Get effective min value (auto-calculated if not specified)
    fn effective_min(&self) -> f64 {
        let data = self.imp().data.borrow();
        self.imp()
            .min
            .borrow()
            .unwrap_or_else(|| data.iter().copied().fold(f64::INFINITY, f64::min))
    }

    /// Get effective max value (auto-calculated if not specified)
    fn effective_max(&self) -> f64 {
        let data = self.imp().data.borrow();
        self.imp()
            .max
            .borrow()
            .unwrap_or_else(|| data.iter().copied().fold(f64::NEG_INFINITY, f64::max))
    }

    /// Draw the graph
    // Array index is usize, graph coordinate calc uses f64
    #[allow(clippy::cast_precision_loss)]
    fn draw(&self, cr: &cairo::Context, width: i32, height: i32) {
        let width = f64::from(width);
        let height = f64::from(height);
        let data = self.imp().data.borrow();

        // Need at least 2 points to draw a line
        if data.len() < 2 {
            return;
        }

        let padding = design::graph::PADDING;
        let graph_width = width - padding * 2.0;
        let graph_height = height - padding * 2.0;

        // Calculate min/max for scaling
        let min_value = self.effective_min();
        let mut max_value = self.effective_max();

        // Handle case where min == max
        if (min_value - max_value).abs() < f64::EPSILON {
            max_value = min_value + 1.0;
        }

        let range = max_value - min_value;

        // Calculate points
        let points: Vec<(f64, f64)> = data
            .iter()
            .enumerate()
            .map(|(i, &value)| {
                let x = padding + (i as f64 / (data.len() - 1) as f64) * graph_width;
                let y = height - padding - ((value - min_value) / range) * graph_height;
                (x, y)
            })
            .collect();

        // Parse color
        let color = parse_color(&self.imp().color.borrow());

        // Adjust line width based on data density (thinner for more points)
        let line_width = if data.len() > 10 {
            1.5
        } else {
            design::graph::LINE_WIDTH
        };

        // Set up stroke style
        cr.set_source_rgb(color.0, color.1, color.2);
        cr.set_line_width(line_width);
        cr.set_line_cap(cairo::LineCap::Round);
        cr.set_line_join(cairo::LineJoin::Round);

        // Start path at first point
        cr.move_to(points[0].0, points[0].1);

        // Draw smooth quadratic curves between points (matching QML)
        for i in 0..points.len() - 1 {
            let (x1, y1) = points[i];
            let (x2, y2) = points[i + 1];

            // Control point is at midpoint
            let xc = f64::midpoint(x1, x2);
            let yc = f64::midpoint(y1, y2);

            // Quadratic curve to midpoint
            cr.curve_to(x1, y1, x1, y1, xc, yc);
        }

        // Final curve to last point
        if points.len() >= 2 {
            let last = points[points.len() - 1];
            let prev = points[points.len() - 2];
            cr.curve_to(prev.0, prev.1, prev.0, prev.1, last.0, last.1);
        }

        let _ = cr.stroke();
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
        (0.5, 0.5, 0.5) // Default gray
    }
}

/// Generate CSS for graph widget (minimal - most styling is in draw function)
pub fn graph_css(theme: &Theme) -> String {
    let size = theme.scaled(design::icon::CONTAINER_SIZE);

    format!(
        r"
        .graph {{
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
    }

    #[test]
    fn test_graph_design_constants() {
        assert_eq!(design::graph::SIZE, 40);
        assert!((design::graph::PADDING - 2.0).abs() < 0.01);
        assert!((design::graph::LINE_WIDTH - 2.0).abs() < 0.01);
    }
}
