//! Result item widget - Full result row matching QML `SearchItem` spec
//!
//! Layout structure:
//! ```text
//! +-- ResultItem (button/row) ----------------------------------------+
//! | +-- Running Indicator (3x16px bar, left edge)                    |
//! | +-- Main RowLayout -----------------------------------------------+
//! | | +-- Icon Container (40x40px) ----------------------------------+|
//! | | |  - System icon (32px) or Material icon (26px) or Text/Thumb  ||
//! | | +--------------------------------------------------------------+|
//! | | +-- Content Column ---------------------------------------------+|
//! | | |  +-- Type Indicator Row (optional) --------------------------+||
//! | | |  |  - "Recent"/"Suggested" label with optional sparkle icon  |||
//! | | |  +-----------------------------------------------------------+||
//! | | |  +-- Name Row -----------------------------------------------+||
//! | | |  |  - Name text (13px, highlighted matches)                  |||
//! | | |  +-----------------------------------------------------------+||
//! | | |  +-- Description (9px, monospace, subtext color) ------------+||
//! | | |  +-----------------------------------------------------------+||
//! | | |  +-- Progress Bar (optional) --------------------------------+||
//! | | |  +-----------------------------------------------------------+||
//! | | +--------------------------------------------------------------+|
//! | | +-- Control Row (right side) ----------------------------------+|
//! | | |  - Slider (for slider items) OR Switch (for switch items)    ||
//! | | |  - OR Action Row:                                            ||
//! | | |    - Primary action hint (Enter + verb, when selected)       ||
//! | | |    - Badges (20x20 circular)                                 ||
//! | | |    - Chips (18px pill)                                       ||
//! | | |    - Action Buttons (28x28, max 4)                           ||
//! | | +--------------------------------------------------------------+|
//! | +----------------------------------------------------------------+|
//! +-------------------------------------------------------------------+
//! ```

use crate::colors::Colors;
use crate::config::Theme;
use tracing::{debug_span, trace};

use super::design;
use super::ripple_button::RippleButton;
use super::{BadgeWidget, ChipWidget};
use gtk4::cairo;
use gtk4::glib::{self, SourceId};
use gtk4::prelude::*;
use gtk4::{Align, Orientation};
use hamr_rpc::SearchResult;
use hamr_types::{ResultType, WidgetData};
use std::cell::RefCell;
use std::rc::Rc;

use super::result_visual::{ResultVisual, VisualSize};

type SwitchToggleCallback = std::cell::RefCell<Option<Box<dyn Fn(bool)>>>;

pub struct CustomSwitch {
    track: gtk4::Box,
    thumb: gtk4::Box,
    active: std::cell::Cell<bool>,
    callback: SwitchToggleCallback,
}

impl CustomSwitch {
    #[allow(dead_code)]
    pub fn is_active(&self) -> bool {
        self.active.get()
    }

    #[allow(dead_code)]
    pub fn set_active(&self, active: bool) {
        self.active.set(active);
        self.update_visual();
    }

    pub fn toggle(&self) {
        let new_state = !self.active.get();
        self.active.set(new_state);
        self.update_visual();

        if let Some(ref cb) = *self.callback.borrow() {
            cb(new_state);
        }
    }

    fn update_visual(&self) {
        let active = self.active.get();
        if active {
            self.track.add_css_class("checked");
            self.thumb.add_css_class("checked");
        } else {
            self.track.remove_css_class("checked");
            self.thumb.remove_css_class("checked");
        }
    }

    pub fn set_callback<F: Fn(bool) + 'static>(&self, f: F) {
        *self.callback.borrow_mut() = Some(Box::new(f));
    }
}

/// Debounce delay for showing primary action hint (ms)
const HINT_DEBOUNCE_MS: u32 = 150;

/// Keyboard hints for action buttons (Alt+U, Alt+I, Alt+O, Alt+P)
const ACTION_HINTS: [&str; 4] = ["Alt+U", "Alt+I", "Alt+O", "Alt+P"];

pub struct ResultItem {
    container: gtk4::Overlay,
    id: String,
    button_actions: RefCell<Vec<RippleButton>>,
    slider: Option<gtk4::Scale>,
    switch: Option<Rc<CustomSwitch>>,
    result_type: ResultType,
    primary_action_hint: Option<gtk4::Box>,
    hint_debounce_timer: Rc<RefCell<Option<SourceId>>>,
    name_label: gtk4::Label,
    desc_label: Option<gtk4::Label>,
    visual: RefCell<ResultVisual>,
    badges_container: gtk4::Box,
    chips_container: gtk4::Box,
    verb_label: Option<gtk4::Label>,
    current_icon: RefCell<String>,
    action_buttons_row: Option<gtk4::Box>,
    #[allow(dead_code)] // Stored to keep widget alive in GTK hierarchy
    running_indicator: Option<gtk4::DrawingArea>,
}

impl ResultItem {
    // Widget construction - sequential GTK builder calls with conditional child widgets
    #[allow(clippy::too_many_lines)]
    pub fn new(
        result: &SearchResult,
        selected: bool,
        show_suggestion: bool,
        running: bool,
        theme: &Theme,
    ) -> Self {
        let _span = debug_span!("ResultItem::new", id = %result.id).entered();
        let is_slider = result.is_slider();
        let is_switch = result.is_switch();

        // Use Overlay so running indicator doesn't affect layout
        let overlay = gtk4::Overlay::builder()
            .css_classes(if selected {
                vec!["result-item", "selected"]
            } else {
                vec!["result-item"]
            })
            .build();

        let running_indicator = if running {
            let indicator = Self::create_running_indicator(&theme.colors);
            overlay.add_overlay(&indicator);
            Some(indicator)
        } else {
            None
        };

        // This is the main content container (child of overlay)
        let row = gtk4::Box::builder()
            .orientation(Orientation::Horizontal)
            .hexpand(true)
            .build();
        overlay.set_child(Some(&row));

        let main_row = gtk4::Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(design::item::ICON_CONTENT_SPACING)
            .margin_start(design::item::BUTTON_HORIZONTAL_PADDING)
            .margin_end(design::item::BUTTON_HORIZONTAL_PADDING)
            .margin_top(design::item::BUTTON_VERTICAL_PADDING)
            .margin_bottom(design::item::BUTTON_VERTICAL_PADDING)
            .hexpand(true)
            .build();

        let visual = ResultVisual::new(result, VisualSize::Small, theme);
        main_row.append(visual.widget());

        let content_column = gtk4::Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(design::item::CONTENT_SPACING)
            .valign(Align::Center)
            .hexpand(true)
            .build();

        let should_show_type = !matches!(result.result_type, ResultType::App | ResultType::Normal);

        if show_suggestion && (result.is_suggestion || should_show_type) {
            let type_row = Self::create_type_indicator(result);
            content_column.append(&type_row);
        }

        let (name_row, name_label) = Self::create_name_row(result);
        content_column.append(&name_row);

        let has_progress = matches!(result.widget, Some(WidgetData::Progress { .. }));
        let desc_label = if let Some(desc) = &result.description {
            if has_progress {
                None
            } else {
                let label = gtk4::Label::builder()
                    .label(desc)
                    .css_classes(["result-description"])
                    .halign(Align::Start)
                    .ellipsize(gtk4::pango::EllipsizeMode::End)
                    .max_width_chars(60)
                    .build();

                // Add tooltip for truncated description - show full text if ellipsized
                label.set_has_tooltip(true);
                label.connect_query_tooltip(move |label, _x, _y, _keyboard, tooltip| {
                    let layout = label.layout();
                    // Check if text is truncated by seeing if layout is ellipsized
                    if layout.is_ellipsized() {
                        tooltip.set_text(Some(&label.text()));
                        return true;
                    }
                    false
                });

                content_column.append(&label);
                Some(label)
            }
        } else {
            None
        };

        if let Some(WidgetData::Progress {
            value, max, label, ..
        }) = &result.widget
        {
            let progress_widget = Self::create_progress_bar(*value, *max, label.as_deref());
            content_column.append(&progress_widget);
        }

        main_row.append(&content_column);

        let mut slider: Option<gtk4::Scale> = None;
        let mut switch: Option<Rc<CustomSwitch>> = None;
        let mut button_actions = Vec::new();
        let mut primary_action_hint: Option<gtk4::Box> = None;
        let mut verb_label: Option<gtk4::Label> = None;
        let mut action_buttons_row: Option<gtk4::Box> = None;

        let badges_container = gtk4::Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(4)
            .valign(Align::Center)
            .build();

        let chips_container = gtk4::Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(4)
            .valign(Align::Center)
            .build();

        if is_slider {
            let (slider_row, scale) = Self::create_slider(result);
            main_row.append(&slider_row);
            slider = Some(scale);

            for badge in result.badges.iter().take(5) {
                let badge_widget = BadgeWidget::new(badge);
                badges_container.append(badge_widget.widget());
            }
            main_row.append(&badges_container);
        } else if is_switch {
            let (switch_row, sw) = Self::create_switch(result);
            main_row.append(&switch_row);
            switch = Some(sw);

            for badge in result.badges.iter().take(5) {
                let badge_widget = BadgeWidget::new(badge);
                badges_container.append(badge_widget.widget());
            }
            main_row.append(&badges_container);
        } else {
            let (action_row, buttons, hint, verb) =
                Self::create_action_row(result, selected, &badges_container, &chips_container);
            main_row.append(&action_row);
            button_actions = buttons;
            primary_action_hint = hint;
            verb_label = verb;
            action_buttons_row = Some(action_row);
        }

        row.append(&main_row);

        Self {
            container: overlay,
            id: result.id.clone(),
            button_actions: RefCell::new(button_actions),
            slider,
            switch,
            result_type: result.result_type,
            primary_action_hint,
            hint_debounce_timer: Rc::new(RefCell::new(None)),
            name_label,
            desc_label,
            visual: RefCell::new(visual),
            badges_container,
            chips_container,
            verb_label,
            current_icon: RefCell::new(result.icon_or_default().to_string()),
            action_buttons_row,
            running_indicator,
        }
    }

    /// Create the LED glow running indicator using Cairo drawing
    /// Positioned as overlay so it doesn't affect layout
    fn create_running_indicator(colors: &Colors) -> gtk4::DrawingArea {
        let drawing_area = gtk4::DrawingArea::builder()
            .width_request(design::running_indicator::GLOW_WIDTH)
            .vexpand(true)
            .valign(Align::Fill)
            .halign(Align::Start)
            .can_target(false) // Allow clicks to pass through to the item below
            .build();

        let primary_color = colors.primary.clone();
        drawing_area.set_draw_func(move |_, cr, width, height| {
            Self::draw_led_glow(cr, width, height, &primary_color);
        });

        drawing_area
    }

    /// Draw the LED glow effect with Cairo
    /// LED is positioned at x=0 (left edge), so only the right half is visible
    /// Creates the effect of light emanating from the border
    #[allow(clippy::many_single_char_names)] // Graphics code uses conventional w,h,r,g,b names
    fn draw_led_glow(cr: &cairo::Context, width: i32, height: i32, color: &str) {
        let w = f64::from(width);
        let h = f64::from(height);

        let (r, g, b) = Self::parse_hex_color(color);

        // LED center is at x=0 (left edge), vertically centered
        // This means only the right half of the LED and glow will be visible
        let led_x = 0.0;
        let led_y = h / 2.0;
        let led_radius = 4.0;

        // Glow Layer 1: Wide, soft ambient glow spreading into the content
        let ambient = cairo::RadialGradient::new(led_x, led_y, 0.0, led_x, led_y, w * 1.2);
        ambient.add_color_stop_rgba(0.0, r, g, b, 0.5);
        ambient.add_color_stop_rgba(0.1, r, g, b, 0.3);
        ambient.add_color_stop_rgba(0.3, r, g, b, 0.1);
        ambient.add_color_stop_rgba(0.6, r, g, b, 0.03);
        ambient.add_color_stop_rgba(1.0, r, g, b, 0.0);
        let _ = cr.set_source(&ambient);
        cr.rectangle(0.0, 0.0, w, h);
        let _ = cr.fill();

        // Glow Layer 2: Tighter bloom around the LED
        let bloom = cairo::RadialGradient::new(led_x, led_y, 0.0, led_x, led_y, 12.0);
        bloom.add_color_stop_rgba(0.0, r, g, b, 0.9);
        bloom.add_color_stop_rgba(0.4, r, g, b, 0.4);
        bloom.add_color_stop_rgba(1.0, r, g, b, 0.0);
        let _ = cr.set_source(&bloom);
        cr.rectangle(0.0, 0.0, w, h);
        let _ = cr.fill();

        // LED core: half-circle at the left edge
        cr.arc(led_x, led_y, led_radius, 0.0, 2.0 * std::f64::consts::PI);
        cr.set_source_rgba(r, g, b, 1.0);
        let _ = cr.fill();

        // Specular highlight (slightly offset into visible area)
        let bright_r = f64::midpoint(r, 1.0);
        let bright_g = f64::midpoint(g, 1.0);
        let bright_b = f64::midpoint(b, 1.0);
        cr.arc(
            led_x + 1.0,
            led_y,
            led_radius * 0.35,
            0.0,
            2.0 * std::f64::consts::PI,
        );
        cr.set_source_rgba(bright_r, bright_g, bright_b, 0.7);
        let _ = cr.fill();
    }

    /// Parse hex color string to RGB components (0.0-1.0)
    fn parse_hex_color(hex: &str) -> (f64, f64, f64) {
        let hex = hex.trim_start_matches('#');
        if hex.len() >= 6 {
            let r = f64::from(u8::from_str_radix(&hex[0..2], 16).unwrap_or(100)) / 255.0;
            let g = f64::from(u8::from_str_radix(&hex[2..4], 16).unwrap_or(100)) / 255.0;
            let b = f64::from(u8::from_str_radix(&hex[4..6], 16).unwrap_or(200)) / 255.0;
            (r, g, b)
        } else {
            (0.4, 0.4, 0.8) // Default blueish
        }
    }

    fn create_type_indicator(result: &SearchResult) -> gtk4::Box {
        let row = gtk4::Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(design::item::TYPE_ROW_SPACING)
            .build();

        let (label_text, is_suggestion): (std::borrow::Cow<'_, str>, bool) = if result.is_suggestion
        {
            (
                result
                    .suggestion_reason
                    .as_deref()
                    .unwrap_or("Suggested")
                    .into(),
                true,
            )
        } else {
            let type_label = if result.is_slider() {
                "Sound".into()
            } else if result.is_switch() {
                "Toggle".into()
            } else {
                match result.result_type {
                    ResultType::Recent => "Recent".into(),
                    ResultType::Plugin => "Plugin".into(),
                    ResultType::WebSearch => "Web".into(),
                    ResultType::IndexedItem => {
                        result.plugin_id.as_deref().unwrap_or("Index").into()
                    }
                    ResultType::PatternMatch => "Match".into(),
                    _ => "Item".into(),
                }
            };
            (type_label, false)
        };

        let label_classes = if is_suggestion {
            vec!["result-type-label", "suggestion"]
        } else {
            vec!["result-type-label"]
        };

        let label = gtk4::Label::builder()
            .label(&*label_text)
            .css_classes(label_classes)
            .halign(Align::Start)
            .build();
        row.append(&label);

        if is_suggestion {
            let icon = gtk4::Label::builder()
                .label("auto_awesome")
                .css_classes(["result-type-icon", "material-icon", "suggestion"])
                .build();
            row.append(&icon);
        }

        row
    }

    fn create_name_row(result: &SearchResult) -> (gtk4::Box, gtk4::Label) {
        let row = gtk4::Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(design::item::NAME_ROW_SPACING)
            .build();

        let name_label = gtk4::Label::builder()
            .label(&result.name)
            .css_classes(["result-name"])
            .halign(Align::Start)
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .build();

        // Add tooltip for truncated name - show full text if ellipsized
        name_label.set_has_tooltip(true);
        name_label.connect_query_tooltip(|label, _x, _y, _keyboard, tooltip| {
            let layout = label.layout();
            // Check if text is truncated by seeing if layout is ellipsized
            if layout.is_ellipsized() {
                tooltip.set_text(Some(&label.text()));
                return true;
            }
            false
        });

        row.append(&name_label);

        (row, name_label)
    }

    // Progress ratio is f64 (0.0-1.0), GTK size_request takes i32
    #[allow(clippy::cast_possible_truncation)]
    fn create_progress_bar(value: f64, max: f64, label_text: Option<&str>) -> gtk4::Box {
        let container = gtk4::Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(8)
            .margin_top(4)
            .build();

        let track = gtk4::Box::builder()
            .css_classes(["result-progress-bar"])
            .hexpand(true)
            .build();

        let ratio = (value / max).clamp(0.0, 1.0);
        let fill = gtk4::Box::builder()
            .css_classes(["result-progress-fill"])
            .hexpand(false)
            .build();

        fill.set_size_request((ratio * 200.0) as i32, design::progress::HEIGHT);
        track.append(&fill);

        container.append(&track);

        if let Some(text) = label_text {
            let label = gtk4::Label::builder()
                .label(text)
                .css_classes(["progress-label"])
                .build();
            container.append(&label);
        }

        container
    }

    fn create_slider(result: &SearchResult) -> (gtk4::Box, gtk4::Scale) {
        let container = gtk4::Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(8)
            .valign(Align::Center)
            .build();

        let (value, min, max, step) = if let Some(WidgetData::Slider {
            value,
            min,
            max,
            step,
            ..
        }) = &result.widget
        {
            (*value, *min, *max, *step)
        } else {
            (0.0, 0.0, 100.0, 1.0)
        };

        let minus_btn = gtk4::Button::builder()
            .css_classes(["slider-button"])
            .child(
                &gtk4::Label::builder()
                    .label("remove")
                    .css_classes(["slider-button-icon", "material-icon"])
                    .build(),
            )
            .build();
        container.append(&minus_btn);

        let adjustment = gtk4::Adjustment::new(value, min, max, step, step * 10.0, 0.0);
        let scale = gtk4::Scale::builder()
            .adjustment(&adjustment)
            .orientation(Orientation::Horizontal)
            .css_classes(["result-slider"])
            .hexpand(false)
            .draw_value(true)
            .value_pos(gtk4::PositionType::Left)
            .valign(Align::Center)
            .build();
        scale.set_size_request(design::slider::PREFERRED_WIDTH, -1);
        container.append(&scale);

        let plus_btn = gtk4::Button::builder()
            .css_classes(["slider-button"])
            .child(
                &gtk4::Label::builder()
                    .label("add")
                    .css_classes(["slider-button-icon", "material-icon"])
                    .build(),
            )
            .build();
        container.append(&plus_btn);

        let adj_minus = adjustment.clone();
        minus_btn.connect_clicked(move |_| {
            let current = adj_minus.value();
            let step = adj_minus.step_increment();
            adj_minus.set_value(current - step);
        });

        let adj_plus = adjustment;
        plus_btn.connect_clicked(move |_| {
            let current = adj_plus.value();
            let step = adj_plus.step_increment();
            adj_plus.set_value(current + step);
        });

        (container, scale)
    }

    fn create_switch(result: &SearchResult) -> (gtk4::Box, std::rc::Rc<CustomSwitch>) {
        use std::rc::Rc;

        let value = matches!(result.widget, Some(WidgetData::Switch { value: true }));

        let track = gtk4::Box::builder()
            .css_classes(if value {
                vec!["switch-track", "checked"]
            } else {
                vec!["switch-track"]
            })
            .valign(Align::Center)
            .halign(Align::Center)
            .build();

        // Thumb (inner circle) - position controlled by CSS margin-start
        let thumb = gtk4::Box::builder()
            .css_classes(if value {
                vec!["switch-thumb", "checked"]
            } else {
                vec!["switch-thumb"]
            })
            .halign(Align::Start)
            .valign(Align::Center)
            .build();

        track.append(&thumb);

        let custom_switch = Rc::new(CustomSwitch {
            track: track.clone(),
            thumb: thumb.clone(),
            active: std::cell::Cell::new(value),
            callback: std::cell::RefCell::new(None),
        });

        let gesture = gtk4::GestureClick::new();
        let switch_ref = custom_switch.clone();
        gesture.connect_released(move |_, _, _, _| {
            switch_ref.toggle();
        });
        track.add_controller(gesture);

        (track.clone(), custom_switch)
    }

    fn default_actions_for_result(result: &SearchResult) -> Vec<hamr_types::Action> {
        // Plugins are responsible for providing appropriate actions
        // (e.g., apps plugin parses .desktop files)
        result.actions.clone()
    }

    fn create_action_row(
        result: &SearchResult,
        selected: bool,
        badges_container: &gtk4::Box,
        chips_container: &gtk4::Box,
    ) -> (
        gtk4::Box,
        Vec<RippleButton>,
        Option<gtk4::Box>,
        Option<gtk4::Label>,
    ) {
        let row = gtk4::Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(design::item::ACTION_ROW_SPACING)
            .valign(Align::Center)
            .build();

        let (primary_action_hint, verb_label) = if result.verb_or_default().is_empty() {
            (None, None)
        } else {
            let css_classes = if selected {
                vec!["primary-action-hint", "hint-visible"]
            } else {
                vec!["primary-action-hint", "hint-hidden"]
            };
            let hint_box = gtk4::Box::builder()
                .orientation(Orientation::Horizontal)
                .spacing(4)
                .margin_end(6)
                .valign(Align::Center)
                .css_classes(css_classes)
                .build();

            let kbd = gtk4::Label::builder()
                .label("Enter")
                .css_classes(["kbd"])
                .valign(Align::Center)
                .build();
            hint_box.append(&kbd);

            let action_name = gtk4::Label::builder()
                .label(result.verb_or_default())
                .css_classes(["action-hint-text"])
                .build();
            hint_box.append(&action_name);

            row.append(&hint_box);
            (Some(hint_box), Some(action_name))
        };

        for badge in &result.badges {
            let badge_widget = BadgeWidget::new(badge);
            badges_container.append(badge_widget.widget());
        }
        row.append(badges_container);

        for chip in &result.chips {
            let chip_widget = ChipWidget::new(chip);
            chips_container.append(chip_widget.widget());
        }
        row.append(chips_container);

        let actions = Self::default_actions_for_result(result);

        let mut button_actions = Vec::new();
        for (index, action) in actions
            .iter()
            .take(design::item::MAX_ACTION_BUTTONS)
            .enumerate()
        {
            let hint = ACTION_HINTS.get(index).copied();
            let button = RippleButton::from_action(action, hint);
            row.append(button.widget());
            button_actions.push(button);
        }

        (row, button_actions, primary_action_hint, verb_label)
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn widget(&self) -> &gtk4::Overlay {
        &self.container
    }

    pub fn connect_action_clicked<F: Fn(&str, &str) + Clone + 'static>(&self, f: F) {
        let id = self.id.clone();
        for button in self.button_actions.borrow().iter() {
            let f = f.clone();
            let id = id.clone();
            button.connect_clicked(move |action_id| {
                f(&id, action_id);
            });
        }
    }

    pub fn set_selected(&self, selected: bool) {
        if selected {
            self.container.add_css_class("selected");
        } else {
            self.container.remove_css_class("selected");
        }

        if let Some(source_id) = self.hint_debounce_timer.borrow_mut().take() {
            source_id.remove();
        }

        if let Some(ref hint) = self.primary_action_hint {
            if selected {
                let hint = hint.clone();
                let timer_ref = self.hint_debounce_timer.clone();
                let source_id = glib::timeout_add_local_once(
                    std::time::Duration::from_millis(u64::from(HINT_DEBOUNCE_MS)),
                    move || {
                        hint.remove_css_class("hint-hidden");
                        hint.add_css_class("hint-visible");
                        *timer_ref.borrow_mut() = None;
                    },
                );
                *self.hint_debounce_timer.borrow_mut() = Some(source_id);
            } else {
                hint.remove_css_class("hint-visible");
                hint.add_css_class("hint-hidden");
            }
        }
    }

    pub fn connect_slider_changed<F: Fn(&str, f64) + 'static>(&self, f: F) {
        if let Some(ref scale) = self.slider {
            let id = self.id.clone();
            scale.connect_value_changed(move |s| {
                f(&id, s.value());
            });
        }
    }

    pub fn connect_switch_toggled<F: Fn(&str, bool) + 'static>(&self, f: F) {
        if let Some(ref switch) = self.switch {
            let id = self.id.clone();
            switch.set_callback(move |state| {
                f(&id, state);
            });
        }
    }

    #[allow(dead_code)]
    pub fn result_type(&self) -> &ResultType {
        &self.result_type
    }

    pub fn is_slider(&self) -> bool {
        self.slider.is_some()
    }

    pub fn is_switch(&self) -> bool {
        self.switch.is_some()
    }

    #[allow(dead_code)]
    pub fn set_slider_value(&self, value: f64) {
        if let Some(ref scale) = self.slider {
            scale.set_value(value);
        }
    }

    #[allow(dead_code)]
    pub fn set_switch_active(&self, active: bool) {
        if let Some(ref switch) = self.switch {
            switch.set_active(active);
        }
    }

    #[allow(dead_code)]
    pub fn adjust_slider(&self, direction: i32) {
        if let Some(ref scale) = self.slider {
            let adj = scale.adjustment();
            let step = adj.step_increment();
            let current = adj.value();
            adj.set_value(current + (f64::from(direction) * step));
        }
    }

    #[allow(dead_code)]
    pub fn toggle_switch(&self) {
        if let Some(ref switch) = self.switch {
            switch.toggle();
        }
    }

    // Enumerate index is usize, Tab navigation uses i32 (can be negative)
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    pub fn set_focused_action(&self, index: i32) {
        for (i, button) in self.button_actions.borrow().iter().enumerate() {
            button.set_focused(i as i32 == index);
        }
    }

    // Widget update - sequential field updates with diff checking for performance
    #[allow(clippy::too_many_lines)]
    pub fn update(&self, result: &SearchResult, colors: &Colors) {
        let _span = debug_span!("ResultItem::update", id = %result.id).entered();
        if self.name_label.text() != result.name {
            trace!(old = %self.name_label.text(), new = %result.name, "name changed");
            self.name_label.set_text(&result.name);
        }

        if let Some(ref desc_label) = self.desc_label
            && let Some(desc) = &result.description
            && desc_label.text() != *desc
        {
            desc_label.set_text(desc);
        }

        {
            let current_icon = self.current_icon.borrow();
            if *current_icon != result.icon_or_default() {
                drop(current_icon);
                *self.current_icon.borrow_mut() = result.icon_or_default().to_string();
                self.visual.borrow_mut().update(result, VisualSize::Small);
            }
        }

        if let Some(ref verb_label) = self.verb_label
            && verb_label.text() != result.verb_or_default()
        {
            verb_label.set_text(result.verb_or_default());
        }

        if let Some(ref action_buttons_row) = self.action_buttons_row {
            let current_buttons = self.button_actions.borrow();
            let count_changed = current_buttons.len() != result.actions.len();
            let ids_changed = current_buttons
                .iter()
                .zip(result.actions.iter())
                .any(|(btn, action)| btn.action_id() != action.id);
            drop(current_buttons);

            if count_changed || ids_changed {
                let old_buttons = self.button_actions.borrow();
                for button in old_buttons.iter() {
                    action_buttons_row.remove(button.widget());
                }
                drop(old_buttons);

                let mut new_buttons = Vec::new();
                for (index, action) in result
                    .actions
                    .iter()
                    .take(design::item::MAX_ACTION_BUTTONS)
                    .enumerate()
                {
                    let hint = ACTION_HINTS.get(index).copied();
                    let button = RippleButton::from_action(action, hint);
                    action_buttons_row.append(button.widget());
                    new_buttons.push(button);
                }
                *self.button_actions.borrow_mut() = new_buttons;
            } else {
                let buttons = self.button_actions.borrow();
                for (index, (button, action)) in
                    buttons.iter().zip(result.actions.iter()).enumerate()
                {
                    let hint = ACTION_HINTS.get(index).copied();
                    button.update(action, hint);
                }
            }
        }

        let current_badge_count = {
            let mut count = 0;
            let mut child = self.badges_container.first_child();
            while child.is_some() {
                count += 1;
                child = child.and_then(|c| c.next_sibling());
            }
            count
        };

        if current_badge_count != result.badges.len() {
            while let Some(child) = self.badges_container.first_child() {
                self.badges_container.remove(&child);
            }
            for badge in result.badges.iter().take(5) {
                let badge_widget = BadgeWidget::new(badge);
                self.badges_container.append(badge_widget.widget());
            }
        }

        let current_chip_count = {
            let mut count = 0;
            let mut child = self.chips_container.first_child();
            while child.is_some() {
                count += 1;
                child = child.and_then(|c| c.next_sibling());
            }
            count
        };

        if current_chip_count != result.chips.len() {
            while let Some(child) = self.chips_container.first_child() {
                self.chips_container.remove(&child);
            }
            for chip in result.chips.iter().take(5) {
                let chip_widget = ChipWidget::new(chip);
                self.chips_container.append(chip_widget.widget());
            }
        }

        if let Some(WidgetData::Gauge {
            value,
            min,
            max,
            label,
            color,
        }) = &result.widget
            && let Some(gauge_widget) = self.visual.borrow().gauge()
        {
            let gauge_data = hamr_types::GaugeData {
                value: *value,
                min: *min,
                max: *max,
                label: label.clone(),
                color: color.clone(),
            };
            gauge_widget.set_data(&gauge_data, colors);
        }

        if let Some(WidgetData::Graph { data, min, max }) = &result.widget
            && let Some(graph_widget) = self.visual.borrow().graph()
        {
            let graph_data = hamr_types::GraphData {
                data: data.clone(),
                min: *min,
                max: *max,
            };
            graph_widget.set_data(&graph_data, colors);
        }
    }
}

impl AsRef<gtk4::Widget> for ResultItem {
    fn as_ref(&self) -> &gtk4::Widget {
        self.container.upcast_ref()
    }
}

// CSS template - splitting would scatter related style rules
#[allow(clippy::too_many_lines)]
pub fn result_item_css(theme: &crate::config::Theme) -> String {
    let colors = &theme.colors;
    let fonts = &theme.config.fonts;
    format!(
        r#"
        /* Result Item Container */
        overlay.result-item {{
            border-radius: {item_radius}px;
            margin-left: {h_margin}px;
            margin-right: {h_margin}px;
            margin-top: {margin_v}px;
            margin-bottom: {margin_v}px;
            background-color: transparent;
            background: transparent;
            border: {border}px solid transparent;
            /* Fade transition - cubic-bezier heavy on brighter side (slow out) */
            transition: background-color 180ms cubic-bezier(0.25, 0.1, 0.25, 1),
                        background 180ms cubic-bezier(0.25, 0.1, 0.25, 1),
                        border-color 180ms cubic-bezier(0.25, 0.1, 0.25, 1);
        }}

        overlay.result-item:hover {{
            background-color: {primary_container};
            background: {primary_container};
        }}

        /* Disable hover highlight during scroll */
        box.scrolling overlay.result-item:hover {{
            background-color: transparent;
            background: transparent;
        }}

        overlay.result-item.selected {{
            background: linear-gradient(to bottom, rgba(149, 144, 136, 0.08), 10%, {surface_dark});
            background-color: {surface_dark};
            border: {border}px solid alpha({outline}, 0.28);
        }}

        overlay.result-item.selected:hover {{
            background: linear-gradient(to bottom, rgba(149, 144, 136, 0.08), 10%, {surface_dark});
            background-color: {surface_high};
            border: {border}px solid alpha({outline}, 0.28);
        }}

        /* Keep selected style during scroll, just no hover enhancement */
        box.scrolling overlay.result-item.selected:hover {{
            background: linear-gradient(to bottom, rgba(149, 144, 136, 0.08), 10%, {surface_dark});
            background-color: {surface_dark};
            border: {border}px solid alpha({outline}, 0.28);
        }}

        /* Icon Container */
        .result-icon-container {{
            min-width: {icon_container}px;
            min-height: {icon_container}px;
        }}

        .result-icon {{
            font-family: "{icon_font}";
            font-size: {icon_material}px;
            color: {on_surface_variant};
        }}

        .result-icon-image {{
            min-width: {icon_system}px;
            min-height: {icon_system}px;
        }}

        .result-icon-text {{
            font-size: {icon_text}px;
            min-width: {icon_container}px;
            min-height: {icon_container}px;
        }}

        .result-thumbnail {{
            border-radius: {thumb_radius}px;
            min-width: {icon_container}px;
            min-height: {icon_container}px;
        }}

        /* Content */
        .result-name {{
            font-family: "{main_font}", sans-serif;
            font-size: {name_size}px;
            color: {on_surface};
        }}

        .result-description {{
            font-family: "{mono_font}", monospace;
            font-size: {desc_size}px;
            color: {subtext};
        }}

        /* Type Label */
        .result-type {{
            font-family: "{mono_font}", monospace;
            font-size: {desc_size}px;
            color: {subtext};
        }}

        .result-type-label {{
            font-family: "{mono_font}", monospace;
            font-size: {type_size}px;
            color: alpha({on_surface_variant}, 0.6);
        }}

        .result-type-icon {{
            font-family: "{icon_font}";
            font-size: {type_size}px;
            color: alpha({on_surface_variant}, 0.6);
            /* Ensure icon doesn't add extra height */
            min-height: {type_size}px;
            margin-top: 0;
            margin-bottom: 0;
        }}

        /* Suggestion styling - use primary color */
        .result-type-label.suggestion {{
            color: {primary};
        }}

        .result-type-icon.suggestion {{
            color: {primary};
        }}

        /* Action Buttons */
        .result-actions {{
            min-height: {action_row_h}px;
        }}

        .action-button {{
            min-width: {action_row_h}px;
            min-height: {action_btn_h}px;
            border-radius: {item_radius}px;
            margin-left: {action_margin}px;
            margin-right: {action_margin}px;
        }}

        /* Primary Action Verb */
        .primary-action-hint {{
            border-radius: {kbd_radius}px;
            padding-top: {kbd_v_pad}px;
            padding-bottom: {kbd_v_pad}px;
            padding-left: {kbd_h_pad}px;
            padding-right: {kbd_h_pad}px;
            background-color: alpha({outline}, 0.05);
        }}

        .primary-action-hint.hint-hidden {{
            opacity: 0;
            transition: opacity 120ms ease-out;
        }}

        .primary-action-hint.hint-visible {{
            opacity: 1;
            transition: opacity 120ms ease-in;
        }}

        .action-hint-text {{
            font-size: {hint_size}px;
            color: {on_surface_variant};
        }}

        /* Keyboard hint pill */
        .kbd {{
            border-radius: {kbd_radius}px;
            padding-top: {kbd_v_pad}px;
            padding-bottom: {kbd_v_pad}px;
            padding-left: {kbd_h_pad}px;
            padding-right: {kbd_h_pad}px;
            background-color: alpha({outline}, 0.12);
            font-size: {kbd_size}px;
            color: {on_surface};
        }}

        /* Progress bar */
        .result-progress-bar {{
            min-height: {progress_h}px;
            border-radius: {progress_r}px;
            background-color: alpha({on_surface_variant}, 0.12);
        }}

        .result-progress-fill {{
            border-radius: {progress_r}px;
            background-color: {primary};
        }}

        /* Slider */
        scale.result-slider > value {{
            font-size: {slider_value_size}px;
            color: {on_surface};
        }}

        scale.result-slider {{
            min-width: {slider_width}px;
        }}

        scale.result-slider trough {{
            min-height: {slider_track}px;
            border-radius: {slider_track_radius}px;
            background-color: alpha({on_surface_variant}, 0.18);
        }}

        scale.result-slider trough > highlight {{
            min-height: {slider_track}px;
            border-radius: {slider_track_radius}px;
            background-color: {primary};
        }}

        scale.result-slider slider {{
            min-width: {slider_thumb}px;
            min-height: {slider_thumb}px;
            border-radius: 9999px;
            background-color: {primary};
            margin: -{slider_thumb_margin}px;
        }}

        .slider-button {{
            min-width: {slider_btn}px;
            min-height: {slider_btn}px;
            border-radius: 9999px;
            background-color: transparent;
            padding: 0;
            border: none;
            transition: background-color 150ms ease;
        }}

        .slider-button:hover {{
            background-color: alpha({on_surface_variant}, 0.1);
        }}

        .slider-button:active {{
            background-color: alpha({on_surface_variant}, 0.2);
        }}

        .slider-button-icon {{
            font-family: "{icon_font}";
            font-size: {slider_icon_size}px;
            color: {on_surface_variant};
        }}

        /* Custom Switch - Material Design 3 style */
        .switch-track {{
            min-width: {switch_w}px;
            min-height: {switch_h}px;
            border-radius: 9999px;
            background-color: {surface_high};
            border: {switch_border}px solid {outline_variant};
            transition: background-color 200ms ease-out, border-color 200ms ease-out;
        }}

        .switch-track.checked {{
            background-color: {primary};
            border-color: {primary};
        }}

        /* Switch thumb - fully circular with slide animation */
        .switch-thumb {{
            min-width: {switch_thumb}px;
            min-height: {switch_thumb}px;
            border-radius: 9999px;
            background-color: {outline};
            margin-top: {thumb_margin_v}px;
            margin-bottom: {thumb_margin_v}px;
            margin-left: 0px;
            margin-right: 0px;
            transition: margin-left 200ms ease, background-color 200ms ease;
        }}

        .switch-thumb.checked {{
            background-color: {on_primary};
            margin-left: {switch_slide}px;
        }}
        "#,
        // Item dimensions (scaled)
        item_radius = theme.scaled(design::item::BUTTON_RADIUS),
        h_margin = theme.scaled(design::item::HORIZONTAL_MARGIN),
        margin_v = theme.scaled(design::spacing::XXXS), // 2px
        border = theme.scaled(1),                       // 1px border
        // Icon dimensions (scaled)
        icon_container = theme.scaled(design::icon::CONTAINER_SIZE),
        icon_material = theme.scaled_font(design::icon::MATERIAL_SIZE),
        icon_system = theme.scaled(design::icon::SYSTEM_SIZE),
        icon_text = theme.scaled_font(design::icon::TEXT_SIZE),
        thumb_radius = theme.scaled(design::icon::THUMBNAIL_RADIUS),
        // Font sizes (scaled)
        name_size = theme.scaled_font(design::font::MD),
        desc_size = theme.scaled_font(design::font::XS),
        type_size = theme.scaled_font(design::font::XS),
        kbd_size = theme.scaled_font(design::kbd::FONT_SIZE),
        hint_size = theme.scaled_font(design::font::XS),
        slider_icon_size = theme.scaled_font(design::font::XL), // 18 -> 17 (nearest token)
        slider_value_size = theme.scaled_font(design::font::XS),
        // Kbd padding (scaled)
        kbd_h_pad = theme.scaled(design::kbd::PADDING_HORIZONTAL),
        kbd_v_pad = theme.scaled(design::kbd::PADDING_VERTICAL),
        kbd_radius = theme.scaled(design::kbd::RADIUS),
        // Progress (scaled)
        progress_h = theme.scaled(design::progress::HEIGHT),
        progress_r = theme.scaled(design::progress::RADIUS),
        // Slider (scaled)
        slider_width = theme.scaled(design::slider::PREFERRED_WIDTH),
        slider_track = theme.scaled(design::slider::TRACK_HEIGHT),
        slider_track_radius = theme.scaled(design::slider::TRACK_HEIGHT / 2),
        slider_thumb = theme.scaled(design::slider::THUMB_SIZE),
        // Negative margin to center thumb on thin track: (thumb - track) / 2
        slider_thumb_margin =
            theme.scaled((design::slider::THUMB_SIZE - design::slider::TRACK_HEIGHT) / 2),
        slider_btn = theme.scaled(design::slider::BUTTON_SIZE),
        // Switch (scaled)
        switch_w = theme.scaled(design::switch::WIDTH),
        switch_h = theme.scaled(design::switch::HEIGHT),
        switch_thumb = theme.scaled(design::switch::THUMB_SIZE),
        switch_border = theme.scaled(design::switch::BORDER_WIDTH),
        thumb_margin_v = theme.scaled(1), // 1px margin
        // Slide distance: track_width - thumb_width (no gap)
        switch_slide = theme.scaled(design::switch::WIDTH - design::switch::THUMB_SIZE),
        // Action buttons (scaled)
        action_row_h = theme.scaled(44), // 44px -> nearest token would be icon::CONTAINER_SIZE + spacing, keep as constant
        action_btn_h = theme.scaled(design::icon::XL), // 32px -> icon::XL
        action_margin = theme.scaled(design::spacing::XS), // 4px
        // Fonts
        main_font = fonts.main,
        mono_font = fonts.monospace,
        icon_font = fonts.icon,
        // Colors
        primary = colors.primary,
        primary_container = colors.primary_container,
        on_surface = colors.on_surface,
        on_surface_variant = colors.on_surface_variant,
        on_primary = colors.on_primary,
        subtext = colors.outline,
        outline = colors.outline,
        outline_variant = colors.outline_variant,
        surface_high = colors.surface_container_high,
        surface_dark = colors.surface,
    )
}
