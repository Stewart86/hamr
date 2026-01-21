//!
//! `ActionBar` widget - Minimal action strip + ambient items
//!
//! Renders contextually based on the current mode:
//! - "hints": Shows ambient items when present
//! - "plugin": Shows home/back + plugin action menu

use crate::widgets::ambient_container::AmbientItemWithPlugin;
use crate::widgets::ambient_item::AmbientItemWidget;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::Orientation;
use hamr_rpc::PluginAction;

/// Callback type for simple button events (home, back, help)
type ButtonCallback = Rc<RefCell<Option<Box<dyn Fn()>>>>;
/// Callback type for action events (`action_id`, `was_confirmed`)
type ActionCallback = Rc<RefCell<Option<Box<dyn Fn(&str, bool)>>>>;
/// Callback type for confirmation request events (`action_id`, `confirm_message`)
type ConfirmRequestCallback = Rc<RefCell<Option<Box<dyn Fn(&str, &str)>>>>;
/// Callback type for ambient action events (`plugin_id`, `item_id`, `action_id`)
type AmbientActionCallback = Rc<RefCell<Option<Box<dyn Fn(&str, &str, &str)>>>>;
/// Callback type for ambient dismiss events (`plugin_id`, `item_id`)
type AmbientDismissCallback = Rc<RefCell<Option<Box<dyn Fn(&str, &str)>>>>;
/// Callback type for compact mode toggle events (`new_state`)
type CompactToggleCallback = Rc<RefCell<Option<Box<dyn Fn(bool)>>>>;
/// Callback type for minimize button
type MinimizeCallback = Rc<RefCell<Option<Box<dyn Fn()>>>>;

use super::design::action_bar as design;

/// Mode of the action bar
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ActionBarMode {
    /// Initial mode - shows ambient items if available
    Hints,
    /// Search mode (unused but kept for parity)
    Search,
    /// Plugin mode - shows navigation + plugin actions
    Plugin,
}

/// Represents an action that can be displayed in the action bar
#[derive(Debug, Clone)]
pub struct ActionBarAction {
    pub id: String,
    pub icon: String,
    pub name: String,
    pub shortcut: Option<String>,
    pub confirm: Option<String>,
}

impl From<&PluginAction> for ActionBarAction {
    fn from(action: &PluginAction) -> Self {
        Self {
            id: action.id.clone(),
            icon: action.icon.clone().unwrap_or_default(),
            name: action.name.clone(),
            shortcut: action.shortcut.clone(),
            confirm: action.confirm.clone(),
        }
    }
}

struct ActionBarState {
    mode: ActionBarMode,
    navigation_depth: usize,
    actions: Vec<ActionBarAction>,
    ambient_items: Vec<AmbientItemWithPlugin>,
    actions_visible: bool,
    compact_mode: bool,
}

impl Default for ActionBarState {
    fn default() -> Self {
        Self {
            mode: ActionBarMode::Hints,
            navigation_depth: 0,
            actions: Vec::new(),
            ambient_items: Vec::new(),
            actions_visible: false,
            compact_mode: false,
        }
    }
}

pub struct ActionBar {
    state: Rc<RefCell<ActionBarState>>,
    ambient_container: gtk4::Box,
    ambient_widgets: Rc<RefCell<HashMap<String, AmbientItemWidget>>>,
    actions_row: gtk4::Box,
    home_button: gtk4::Button,
    back_button: gtk4::Button,
    actions_menu_button: gtk4::MenuButton,
    actions_popover: gtk4::Popover,
    actions_popover_container: gtk4::Box,
    help_button: gtk4::Button,
    compact_toggle: gtk4::Button,
    minimize_button: gtk4::Button,
    on_home: ButtonCallback,
    on_back: ButtonCallback,
    on_action: ActionCallback,
    on_confirm_request: ConfirmRequestCallback,
    on_help: ButtonCallback,
    on_ambient_action: AmbientActionCallback,
    on_ambient_dismiss: AmbientDismissCallback,
    on_compact_toggle: CompactToggleCallback,
    on_minimize: MinimizeCallback,
}

impl ActionBar {
    pub fn new() -> Self {
        let state = Rc::new(RefCell::new(ActionBarState::default()));

        let actions_row = gtk4::Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(design::BUTTON_SPACING)
            .halign(gtk4::Align::End)
            .css_classes(["action-bar", "action-bar-actions"])
            .build();
        actions_row.set_visible(false);
        actions_row.set_sensitive(false);
        actions_row.set_can_target(false);

        let ambient_container = gtk4::Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(6)
            .css_classes(["ambient-container"])
            .build();
        ambient_container.set_visible(false);

        let home_button = Self::create_icon_button("home", "Home");
        let back_button = Self::create_icon_button("arrow_back", "Back");
        let help_button = Self::create_icon_button("keyboard", "Keymap");
        let compact_toggle = Self::create_icon_button("view_list", "Toggle compact mode");
        let minimize_button = Self::create_icon_button("minimize", "Minimize (Ctrl+M)");

        let actions_menu_button = gtk4::MenuButton::builder()
            .css_classes(["action-bar-icon-button", "action-bar-menu-button"])
            .tooltip_text("Actions")
            .build();

        let actions_menu_icon = gtk4::Label::builder()
            .label("more_horiz")
            .css_classes(["material-icon", "action-bar-icon"])
            .build();
        actions_menu_button.set_child(Some(&actions_menu_icon));

        let actions_popover_container = gtk4::Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(6)
            .margin_start(8)
            .margin_end(8)
            .margin_top(8)
            .margin_bottom(8)
            .build();

        let actions_popover = gtk4::Popover::builder()
            .has_arrow(true)
            .position(gtk4::PositionType::Bottom)
            .css_classes(["action-popover"])
            .child(&actions_popover_container)
            .build();
        actions_menu_button.set_popover(Some(&actions_popover));

        actions_row.append(&home_button);
        actions_row.append(&back_button);
        actions_row.append(&actions_menu_button);
        actions_row.append(&compact_toggle);
        actions_row.append(&minimize_button);
        actions_row.append(&help_button);

        let action_bar = Self {
            state,
            ambient_container,
            ambient_widgets: Rc::new(RefCell::new(HashMap::new())),
            actions_row,
            home_button,
            back_button,
            actions_menu_button,
            actions_popover,
            actions_popover_container,
            help_button,
            compact_toggle,
            minimize_button,
            on_home: Rc::new(RefCell::new(None)),
            on_back: Rc::new(RefCell::new(None)),
            on_action: Rc::new(RefCell::new(None)),
            on_confirm_request: Rc::new(RefCell::new(None)),
            on_help: Rc::new(RefCell::new(None)),
            on_ambient_action: Rc::new(RefCell::new(None)),
            on_ambient_dismiss: Rc::new(RefCell::new(None)),
            on_compact_toggle: Rc::new(RefCell::new(None)),
            on_minimize: Rc::new(RefCell::new(None)),
        };

        action_bar.setup_button_handlers();
        action_bar.update_visibility();
        action_bar
    }

    fn create_icon_button(icon: &str, tooltip: &str) -> gtk4::Button {
        let button = gtk4::Button::builder()
            .css_classes(["action-bar-icon-button"])
            .tooltip_text(tooltip)
            .build();

        let icon_label = gtk4::Label::builder()
            .label(icon)
            .css_classes(["material-icon", "action-bar-icon"])
            .build();

        button.set_child(Some(&icon_label));
        button
    }

    fn create_section_label(text: &str) -> gtk4::Label {
        gtk4::Label::builder()
            .label(text)
            .halign(gtk4::Align::Start)
            .css_classes(["action-popover-section"])
            .build()
    }

    fn create_section_separator() -> gtk4::Separator {
        let separator = gtk4::Separator::new(Orientation::Horizontal);
        separator.add_css_class("action-popover-separator");
        separator
    }

    fn setup_button_handlers(&self) {
        let on_home = self.on_home.clone();
        self.home_button.connect_clicked(move |_| {
            if let Some(ref cb) = *on_home.borrow() {
                cb();
            }
        });

        let on_back = self.on_back.clone();
        self.back_button.connect_clicked(move |_| {
            if let Some(ref cb) = *on_back.borrow() {
                cb();
            }
        });

        let on_help = self.on_help.clone();
        self.help_button.connect_clicked(move |_| {
            if let Some(ref cb) = *on_help.borrow() {
                cb();
            }
        });

        let state = self.state.clone();
        let on_compact_toggle = self.on_compact_toggle.clone();
        let compact_toggle = self.compact_toggle.clone();
        self.compact_toggle.connect_clicked(move |_| {
            let new_state = {
                let mut s = state.borrow_mut();
                s.compact_mode = !s.compact_mode;
                s.compact_mode
            };
            Self::update_compact_toggle_icon(&compact_toggle, new_state);
            if let Some(ref cb) = *on_compact_toggle.borrow() {
                cb(new_state);
            }
        });

        let on_minimize = self.on_minimize.clone();
        self.minimize_button.connect_clicked(move |_| {
            if let Some(ref cb) = *on_minimize.borrow() {
                cb();
            }
        });
    }

    fn update_compact_toggle_icon(button: &gtk4::Button, compact_mode: bool) {
        let icon = if compact_mode {
            "view_compact"
        } else {
            "view_list"
        };
        let tooltip = if compact_mode {
            "Show recent items on open"
        } else {
            "Hide recent items on open"
        };
        if let Some(label) = button
            .child()
            .and_then(|c| c.downcast::<gtk4::Label>().ok())
        {
            label.set_label(icon);
        }
        button.set_tooltip_text(Some(tooltip));
    }

    /// Get the GTK widget for inline actions
    pub fn actions_widget(&self) -> &gtk4::Box {
        &self.actions_row
    }

    /// Get the GTK widget for ambient items
    pub fn ambient_widget(&self) -> &gtk4::Box {
        &self.ambient_container
    }

    /// Set the action bar mode
    pub fn set_mode(&self, mode: ActionBarMode) {
        self.state.borrow_mut().mode = mode;
        self.update_visibility();
    }

    /// Set navigation depth (plugin mode)
    pub fn set_navigation_depth(&self, depth: usize) {
        self.state.borrow_mut().navigation_depth = depth;
        self.update_visibility();
    }

    /// Set plugin actions
    pub fn set_actions(&self, actions: Vec<ActionBarAction>) {
        self.state.borrow_mut().actions = actions;
        self.rebuild_actions();
        self.update_visibility();
    }

    pub fn set_actions_visible(&self, visible: bool) {
        self.state.borrow_mut().actions_visible = visible;
        if !visible {
            self.actions_popover.popdown();
        }
        self.actions_row.set_visible(visible);
        self.actions_row.set_sensitive(visible);
        self.actions_row.set_can_target(visible);
        self.update_visibility();
    }

    /// Connect home button callback
    pub fn connect_home<F: Fn() + 'static>(&self, f: F) {
        *self.on_home.borrow_mut() = Some(Box::new(f));
    }

    /// Connect back button callback
    pub fn connect_back<F: Fn() + 'static>(&self, f: F) {
        *self.on_back.borrow_mut() = Some(Box::new(f));
    }

    /// Connect action callback (`action_id`, `was_confirmed`)
    pub fn connect_action<F: Fn(&str, bool) + 'static>(&self, f: F) {
        *self.on_action.borrow_mut() = Some(Box::new(f));
    }

    /// Connect confirmation request callback (`action_id`, `confirm_message`)
    /// Called when user clicks an action that requires confirmation.
    /// The caller should show a `ConfirmDialog` and call back with the result.
    pub fn connect_confirm_request<F: Fn(&str, &str) + 'static>(&self, f: F) {
        *self.on_confirm_request.borrow_mut() = Some(Box::new(f));
    }

    /// Connect help button callback
    pub fn connect_help<F: Fn() + 'static>(&self, f: F) {
        *self.on_help.borrow_mut() = Some(Box::new(f));
    }

    /// Connect ambient action callback (`plugin_id`, `item_id`, `action_id`)
    pub fn connect_ambient_action<F: Fn(&str, &str, &str) + 'static>(&self, f: F) {
        *self.on_ambient_action.borrow_mut() = Some(Box::new(f));
    }

    /// Connect ambient dismiss callback (`plugin_id`, `item_id`)
    pub fn connect_ambient_dismiss<F: Fn(&str, &str) + 'static>(&self, f: F) {
        *self.on_ambient_dismiss.borrow_mut() = Some(Box::new(f));
    }

    /// Connect compact mode toggle callback (`new_state`)
    pub fn connect_compact_toggle<F: Fn(bool) + 'static>(&self, f: F) {
        *self.on_compact_toggle.borrow_mut() = Some(Box::new(f));
    }

    /// Connect minimize button callback
    pub fn connect_minimize<F: Fn() + 'static>(&self, f: F) {
        *self.on_minimize.borrow_mut() = Some(Box::new(f));
    }

    /// Set compact mode state (for initialization from persisted settings)
    pub fn set_compact_mode(&self, compact: bool) {
        self.state.borrow_mut().compact_mode = compact;
        Self::update_compact_toggle_icon(&self.compact_toggle, compact);
    }

    /// Get current compact mode state
    #[allow(dead_code)]
    pub fn compact_mode(&self) -> bool {
        self.state.borrow().compact_mode
    }

    /// Set ambient items (from all plugins) - uses diffing for reactive updates
    pub fn set_ambient_items(&self, items: &[AmbientItemWithPlugin]) {
        let previously_had_items = !self.state.borrow().ambient_items.is_empty();
        let has_items = !items.is_empty();

        self.state.borrow_mut().ambient_items = items.to_vec();
        self.update_ambient_items_diff(items);

        if previously_had_items != has_items {
            self.update_visibility();
        }
    }

    /// Update ambient items using diffing (only update changed, add new, remove old)
    fn update_ambient_items_diff(&self, items: &[AmbientItemWithPlugin]) {
        use std::collections::HashSet;

        let mut widgets = self.ambient_widgets.borrow_mut();

        let new_keys: HashSet<String> = items
            .iter()
            .map(|i| format!("{}:{}", i.plugin_id, i.item.id))
            .collect();

        let keys_to_remove: Vec<String> = widgets
            .keys()
            .filter(|k| !new_keys.contains(*k))
            .cloned()
            .collect();

        for key in keys_to_remove {
            if let Some(widget) = widgets.remove(&key) {
                self.ambient_container.remove(widget.widget());
            }
        }

        for item_with_plugin in items {
            let key = format!(
                "{}:{}",
                item_with_plugin.plugin_id, item_with_plugin.item.id
            );

            if let Some(widget) = widgets.get(&key) {
                widget.update(&item_with_plugin.item);
            } else {
                let widget =
                    AmbientItemWidget::new(&item_with_plugin.item, &item_with_plugin.plugin_id);

                let plugin_id = item_with_plugin.plugin_id.clone();
                let on_ambient_action = self.on_ambient_action.clone();
                widget.connect_action(move |item_id, action_id| {
                    if let Some(ref cb) = *on_ambient_action.borrow() {
                        cb(&plugin_id, item_id, action_id);
                    }
                });

                let plugin_id = item_with_plugin.plugin_id.clone();
                let on_ambient_dismiss = self.on_ambient_dismiss.clone();
                widget.connect_dismiss(move |item_id| {
                    if let Some(ref cb) = *on_ambient_dismiss.borrow() {
                        cb(&plugin_id, item_id);
                    }
                });

                self.ambient_container.append(widget.widget());
                widgets.insert(key, widget);
            }
        }
    }

    fn update_visibility(&self) {
        fn set_visible_if_changed(widget: &impl gtk4::prelude::WidgetExt, visible: bool) {
            if widget.is_visible() != visible {
                widget.set_visible(visible);
            }
        }

        let state = self.state.borrow();
        let has_ambient = !state.ambient_items.is_empty();
        let actions_visible = state.actions_visible;
        let in_plugin = state.mode == ActionBarMode::Plugin;

        set_visible_if_changed(
            &self.ambient_container,
            has_ambient && state.mode == ActionBarMode::Hints,
        );
        set_visible_if_changed(&self.actions_row, actions_visible);

        set_visible_if_changed(&self.home_button, in_plugin);
        set_visible_if_changed(&self.back_button, in_plugin);
        set_visible_if_changed(
            &self.actions_menu_button,
            in_plugin && !state.actions.is_empty(),
        );

        if in_plugin {
            let should_be_sensitive = state.navigation_depth > 0;
            if self.back_button.is_sensitive() != should_be_sensitive {
                self.back_button.set_sensitive(should_be_sensitive);
            }
        }

        if self.actions_row.height_request() != design::HEIGHT_NORMAL {
            self.actions_row.set_height_request(design::HEIGHT_NORMAL);
        }
        if self.ambient_container.height_request() != design::HEIGHT_AMBIENT {
            self.ambient_container
                .set_height_request(design::HEIGHT_AMBIENT);
        }
    }

    fn rebuild_actions(&self) {
        while let Some(child) = self.actions_popover_container.first_child() {
            self.actions_popover_container.remove(&child);
        }

        let state = self.state.borrow();
        let actions: Vec<ActionBarAction> = state.actions.iter().take(6).cloned().collect();
        let has_actions = !actions.is_empty();
        self.actions_menu_button.set_visible(has_actions);
        self.actions_menu_button.set_sensitive(has_actions);

        if !has_actions {
            self.actions_popover.popdown();
            return;
        }

        let (primary, confirm): (Vec<ActionBarAction>, Vec<ActionBarAction>) = actions
            .into_iter()
            .partition(|action| action.confirm.is_none());

        let mut first_section = true;

        if !primary.is_empty() {
            self.append_action_section("Actions", &primary);
            first_section = false;
        }

        if !confirm.is_empty() {
            if !first_section {
                self.actions_popover_container
                    .append(&Self::create_section_separator());
            }
            self.append_action_section("Confirm", &confirm);
        }
    }

    fn append_action_section(&self, title: &str, actions: &[ActionBarAction]) {
        self.actions_popover_container
            .append(&Self::create_section_label(title));

        let section = gtk4::Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(4)
            .build();

        for action in actions {
            let button = gtk4::Button::builder()
                .css_classes(["action-popover-item"])
                .tooltip_text(&action.name)
                .build();

            let content = gtk4::Box::builder()
                .orientation(Orientation::Horizontal)
                .spacing(8)
                .build();

            if !action.icon.is_empty() {
                let icon = gtk4::Label::builder()
                    .label(&action.icon)
                    .css_classes(["material-icon", "action-bar-icon"])
                    .build();
                content.append(&icon);
            }

            let label = gtk4::Label::builder()
                .label(&action.name)
                .hexpand(true)
                .halign(gtk4::Align::Start)
                .build();
            content.append(&label);

            if let Some(shortcut) = action.shortcut.as_deref() {
                let shortcut_label = gtk4::Label::builder()
                    .label(shortcut)
                    .css_classes(["action-popover-shortcut"])
                    .halign(gtk4::Align::End)
                    .build();
                content.append(&shortcut_label);
            }

            button.set_child(Some(&content));

            let action_id = action.id.clone();
            let confirm_message = action.confirm.clone();
            let on_action = self.on_action.clone();
            let on_confirm_request = self.on_confirm_request.clone();
            let actions_popover = self.actions_popover.clone();

            button.connect_clicked(move |_| {
                actions_popover.popdown();
                if let Some(ref msg) = confirm_message {
                    if let Some(ref cb) = *on_confirm_request.borrow() {
                        cb(&action_id, msg);
                    }
                } else if let Some(ref cb) = *on_action.borrow() {
                    cb(&action_id, false);
                }
            });

            section.append(&button);
        }

        self.actions_popover_container.append(&section);
    }
}

impl Default for ActionBar {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate CSS for `ActionBar` widget
pub fn action_bar_css(theme: &crate::config::Theme) -> String {
    use super::design::{font, radius, spacing};

    let colors = &theme.colors;

    // Pre-compute scaled values
    let pad_v = theme.scaled(spacing::XXXS); // 2
    let pad_h = theme.scaled(spacing::SM - spacing::XXXS); // 8 - 2 = 6
    let pad_xs = theme.scaled(spacing::XS); // 4
    let pad_sm = theme.scaled(spacing::SM); // 8
    let min_height = theme.scaled(design::HEIGHT_NORMAL); // 28
    let button_size = theme.scaled(design::BUTTON_SIZE); // 26
    let border = theme.scaled(1);
    let popover_radius = theme.scaled(radius::SM + spacing::XXXS); // 8 + 2 = 10
    let item_radius = theme.scaled(radius::SM - spacing::XXXS); // 8 - 2 = 6
    let font_tiny = theme.scaled_font(font::XS + 1); // 9 + 1 = 10
    let font_icon = theme.scaled_font(font::XL + 1); // 17 + 1 = 18

    format!(
        r#"
        .action-bar {{
            padding: {pad_v}px {pad_h}px;
            min-height: {min_height}px;
        }}

        .action-bar-actions {{
            padding: 0;
        }}

        .action-bar-icon-button {{
            background-color: transparent;
            border: none;
            padding: {pad_xs}px;
            min-width: {button_size}px;
            min-height: {button_size}px;
            border-radius: {pill}px;
            transition: background-color 150ms ease-out;
        }}

        .action-bar-icon-button:hover {{
            background-color: {surface_highest};
        }}

        .action-bar-icon-button:disabled {{
            opacity: 0.4;
        }}

        .action-bar-menu-button {{
            padding: {pad_xs}px;
        }}

        .action-popover {{
            background-color: {surface_container};
            border-radius: {popover_radius}px;
            border: {border}px solid {outline_variant};
        }}

        .action-popover-section {{
            font-size: {font_tiny}px;
            font-weight: 600;
            color: {outline};
            margin-bottom: {pad_xs}px;
        }}

        .action-popover-shortcut {{
            font-size: {font_tiny}px;
            color: {outline};
        }}

        .action-popover-separator {{
            min-height: {border}px;
            margin: {pad_xs}px 0;
            background-color: {outline_variant};
        }}

        .action-popover-item {{
            background-color: transparent;
            border: none;
            padding: {item_radius}px {pad_sm}px;
            border-radius: {item_radius}px;
        }}

        .action-popover-item:hover {{
            background-color: {surface_highest};
        }}

        .action-bar-icon {{
            font-family: "Material Symbols Rounded";
            font-size: {font_icon}px;
            color: {on_surface_variant};
        }}

        .ambient-container {{
            padding: 0;
        }}
        "#,
        outline_variant = colors.outline_variant,
        surface_highest = colors.surface_container_highest,
        on_surface_variant = colors.on_surface_variant,
        outline = colors.outline,
        surface_container = colors.surface_container,
        pill = radius::PILL,
    )
}
