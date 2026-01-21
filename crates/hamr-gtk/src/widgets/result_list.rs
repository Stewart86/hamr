//! Result list container widget
//!
//! Manages a scrollable list of `ResultItem` widgets with selection,
//! keyboard navigation, and action handling.

use super::ResultItem;
use super::design;
use crate::compositor::Compositor;
use crate::config::Theme;
use gtk4::glib;
use gtk4::glib::SourceId;
use gtk4::prelude::*;
use gtk4::{Orientation, PolicyType};
use hamr_rpc::SearchResult;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::time::Duration;
use tracing::{debug, debug_span};

/// Delay after scroll stops before re-enabling hover (ms)
const SCROLL_HOVER_DELAY_MS: u64 = 150;

/// Callback type for item selection (activation via Enter/click)
pub type SelectCallback = Box<dyn Fn(&str)>;
/// Callback type for action button clicks
pub type ActionCallback = Box<dyn Fn(&str, &str)>;
/// Callback type for slider value changes
pub type SliderCallback = Box<dyn Fn(&str, f64)>;
/// Callback type for switch toggles
pub type SwitchCallback = Box<dyn Fn(&str, bool)>;
/// Callback type for selection change (keyboard navigation)
pub type SelectionChangeCallback = Box<dyn Fn(Option<&SearchResult>)>;

/// Container for result items with selection and scrolling
pub struct ResultList {
    container: gtk4::Box,
    scrolled: gtk4::ScrolledWindow,
    list_box: gtk4::Box,
    items: Rc<RefCell<Vec<ResultItem>>>,
    items_by_id: Rc<RefCell<HashMap<String, usize>>>, // Maps item ID to index in items vec
    results: Rc<RefCell<Vec<SearchResult>>>,
    selected: Rc<RefCell<usize>>,
    selected_action: Rc<RefCell<i32>>,
    on_select: Rc<RefCell<Option<SelectCallback>>>,
    on_action: Rc<RefCell<Option<ActionCallback>>>,
    on_slider: Rc<RefCell<Option<SliderCallback>>>,
    on_switch: Rc<RefCell<Option<SwitchCallback>>>,
    on_selection_change: Rc<RefCell<Option<SelectionChangeCallback>>>,
    /// Timer to re-enable hover after scrolling stops
    /// NOTE: This is stored to keep the Rc alive, the timer is managed via `SourceId`
    #[allow(dead_code)]
    scroll_timer: Rc<RefCell<Option<SourceId>>>,
    /// Max height from config (pixels)
    max_height: Rc<RefCell<i32>>,
    /// Set of currently running app IDs (lowercase) for showing running indicators
    running_app_ids: Rc<RefCell<HashSet<String>>>,
}

impl ResultList {
    /// Create a new result list container
    pub fn new() -> Self {
        let container = gtk4::Box::builder()
            .orientation(Orientation::Vertical)
            .css_classes(["results-container"])
            .visible(false)
            .build();

        let list_box = gtk4::Box::builder()
            .orientation(Orientation::Vertical)
            .css_classes(["results-list"])
            .build();

        let scrolled = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(PolicyType::Never)
            .vscrollbar_policy(PolicyType::Automatic)
            .propagate_natural_height(true)
            .child(&list_box)
            .build();

        container.append(&scrolled);

        let scroll_timer: Rc<RefCell<Option<SourceId>>> = Rc::new(RefCell::new(None));

        {
            let list_box_ref = list_box.clone();
            let scroll_timer_ref = scroll_timer.clone();

            scrolled.vadjustment().connect_value_changed(move |_| {
                list_box_ref.add_css_class("scrolling");

                if let Some(source_id) = scroll_timer_ref.borrow_mut().take() {
                    source_id.remove();
                }

                let list_box_inner = list_box_ref.clone();
                let scroll_timer_inner = scroll_timer_ref.clone();

                let source_id = glib::timeout_add_local_once(
                    Duration::from_millis(SCROLL_HOVER_DELAY_MS),
                    move || {
                        scroll_timer_inner.borrow_mut().take();
                        list_box_inner.remove_css_class("scrolling");
                    },
                );

                *scroll_timer_ref.borrow_mut() = Some(source_id);
            });
        }

        Self {
            container,
            scrolled,
            list_box,
            items: Rc::new(RefCell::new(Vec::new())),
            items_by_id: Rc::new(RefCell::new(HashMap::new())),
            results: Rc::new(RefCell::new(Vec::new())),
            selected: Rc::new(RefCell::new(0)),
            selected_action: Rc::new(RefCell::new(-1)),
            on_select: Rc::new(RefCell::new(None)),
            on_action: Rc::new(RefCell::new(None)),
            on_slider: Rc::new(RefCell::new(None)),
            on_switch: Rc::new(RefCell::new(None)),
            on_selection_change: Rc::new(RefCell::new(None)),
            scroll_timer,
            max_height: Rc::new(RefCell::new(600)), // Default from config
            running_app_ids: Rc::new(RefCell::new(HashSet::new())),
        }
    }

    /// Refresh the set of running app IDs from the compositor
    pub fn refresh_running_apps(&self, compositor: &Compositor) {
        let running = compositor.get_running_app_ids();
        *self.running_app_ids.borrow_mut() = running;
    }

    /// Check if an app is running based on its `app_id`
    fn is_app_running(&self, app_id: Option<&str>) -> bool {
        let Some(app_id) = app_id else {
            return false;
        };
        if app_id.is_empty() {
            return false;
        }

        let app_id_lower = app_id.to_lowercase();
        let normalized = app_id_lower.replace('-', " ");
        let hyphenated = app_id_lower.replace(' ', "-");

        let running_ids = self.running_app_ids.borrow();
        running_ids.iter().any(|running| {
            let running_normalized = running.replace('-', " ");
            *running == app_id_lower || running_normalized == normalized || *running == hyphenated
        })
    }

    /// Set the maximum height for the results list (from config)
    pub fn set_max_height(&self, height: i32) {
        *self.max_height.borrow_mut() = height;
        self.scrolled.set_max_content_height(height);
        // min_content_height is dynamically calculated in set_results() based on actual content
    }

    /// Set the callback for item selection (Enter pressed)
    pub fn connect_select<F: Fn(&str) + 'static>(&self, f: F) {
        *self.on_select.borrow_mut() = Some(Box::new(f));
    }

    /// Set the callback for action button clicks
    pub fn connect_action<F: Fn(&str, &str) + 'static>(&self, f: F) {
        *self.on_action.borrow_mut() = Some(Box::new(f));
    }

    /// Set the callback for slider value changes
    pub fn connect_slider<F: Fn(&str, f64) + 'static>(&self, f: F) {
        *self.on_slider.borrow_mut() = Some(Box::new(f));
    }

    /// Set the callback for switch toggles
    pub fn connect_switch<F: Fn(&str, bool) + 'static>(&self, f: F) {
        *self.on_switch.borrow_mut() = Some(Box::new(f));
    }

    /// Set the callback for selection changes (keyboard navigation).
    /// Called when the highlighted item changes, not on activation.
    pub fn connect_selection_change<F: Fn(Option<&SearchResult>) + 'static>(&self, f: F) {
        *self.on_selection_change.borrow_mut() = Some(Box::new(f));
    }

    /// Notify selection change callback with the currently selected result
    fn notify_selection_change(&self) {
        if let Some(ref cb) = *self.on_selection_change.borrow() {
            let results = self.results.borrow();
            let selected = *self.selected.borrow();
            let result = results.get(selected);
            cb(result);
        }
    }

    /// Update the list with new results (full rebuild)
    pub fn set_results(&self, results: &[SearchResult], theme: &Theme) {
        let _span = debug_span!("ResultList::set_results", count = results.len()).entered();
        debug!(count = results.len(), "full rebuild");
        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }

        let mut items = self.items.borrow_mut();
        let mut items_by_id = self.items_by_id.borrow_mut();
        items.clear();
        items_by_id.clear();

        *self.results.borrow_mut() = results.to_vec();

        if results.is_empty() {
            return;
        }

        // Note: Visibility is managed by window.rs based on view mode, not here

        let current_selection = *self.selected.borrow();
        let new_selection = current_selection.min(results.len().saturating_sub(1));
        *self.selected.borrow_mut() = new_selection;
        *self.selected_action.borrow_mut() = -1;

        let on_action = self.on_action.clone();
        let on_slider = self.on_slider.clone();
        let on_switch = self.on_switch.clone();

        for (i, result) in results.iter().enumerate() {
            let selected = i == new_selection;
            let show_suggestion = true;
            let running = self.is_app_running(result.app_id.as_deref());

            let item = ResultItem::new(result, selected, show_suggestion, running, theme);

            let on_action = on_action.clone();
            item.connect_action_clicked(move |item_id, action_id| {
                if let Some(ref cb) = *on_action.borrow() {
                    cb(item_id, action_id);
                }
            });

            let on_slider = on_slider.clone();
            item.connect_slider_changed(move |item_id, value| {
                if let Some(ref cb) = *on_slider.borrow() {
                    cb(item_id, value);
                }
            });

            let on_switch = on_switch.clone();
            item.connect_switch_toggled(move |item_id, value| {
                if let Some(ref cb) = *on_switch.borrow() {
                    cb(item_id, value);
                }
            });

            if !item.is_slider() && !item.is_switch() {
                let gesture = gtk4::GestureClick::new();
                let on_select = self.on_select.clone();
                let item_id = result.id.clone();
                let items_ref = self.items.clone();
                let selected_ref = self.selected.clone();
                let idx = i;

                gesture.connect_released(move |_, _, _, _| {
                    {
                        let items = items_ref.borrow();
                        let old_selected = *selected_ref.borrow();
                        if old_selected < items.len() {
                            items[old_selected].set_selected(false);
                        }
                        if idx < items.len() {
                            items[idx].set_selected(true);
                        }
                        *selected_ref.borrow_mut() = idx;
                    }

                    if let Some(ref cb) = *on_select.borrow() {
                        cb(&item_id);
                    }
                });
                item.widget().add_controller(gesture);
            }

            self.list_box.append(item.widget());
            items_by_id.insert(result.id.clone(), i);
            items.push(item);
        }

        let max_height = *self.max_height.borrow();
        self.scrolled.set_max_content_height(max_height);

        // Measure actual content height and set min_content_height
        // This is needed because GtkFixed container doesn't respect natural height
        // Without min_content_height, content collapses to minimum size (tiny bar)
        // With min_content_height, ScrolledWindow enforces the size properly
        if results.is_empty() {
            self.scrolled.set_min_content_height(0);
        } else {
            let (_, natural_height, _, _) =
                self.list_box.measure(gtk4::Orientation::Vertical, -1);
            let min_height = natural_height.min(max_height);
            self.scrolled.set_min_content_height(min_height);
        }

        // Notify about initial selection
        drop(items);
        drop(items_by_id);
        self.notify_selection_change();
    }

    /// Update results using diffing - only update changed items, preserving widget state
    /// Returns true if a full rebuild was performed, false if incremental update was done
    pub fn update_results_diff(&self, results: &[SearchResult], theme: &Theme) -> bool {
        let _span = debug_span!("ResultList::update_results_diff", count = results.len()).entered();

        let items = self.items.borrow();
        let items_by_id = self.items_by_id.borrow();

        // If count changed significantly or we have no items, do a full rebuild
        if items.is_empty() || results.is_empty() {
            debug!(reason = "empty", "falling back to full rebuild");
            drop(items);
            drop(items_by_id);
            self.set_results(results, theme);
            return true;
        }

        // Build set of new IDs and check if they match existing
        let new_ids: HashSet<&str> = results.iter().map(|r| r.id.as_str()).collect();
        let old_ids: HashSet<&str> = items_by_id
            .keys()
            .map(std::string::String::as_str)
            .collect();

        // If IDs changed (items added/removed), do full rebuild
        if new_ids != old_ids {
            debug!(reason = "ids_changed", "falling back to full rebuild");
            drop(items);
            drop(items_by_id);
            self.set_results(results, theme);
            return true;
        }

        // Same IDs - update existing items in place
        debug!(count = results.len(), "incremental update");
        for result in results {
            if let Some(&idx) = items_by_id.get(&result.id)
                && idx < items.len()
            {
                items[idx].update(result, &theme.colors);
            }
        }

        // Update stored results
        *self.results.borrow_mut() = results.to_vec();

        false
    }

    /// Move selection up
    pub fn select_prev(&self) {
        let items = self.items.borrow();
        if items.is_empty() {
            return;
        }

        let mut selected = self.selected.borrow_mut();
        if *selected > 0 {
            items[*selected].set_selected(false);
            *selected -= 1;
            items[*selected].set_selected(true);
            let idx = *selected;
            drop(selected);
            drop(items);
            self.scroll_to_selected(idx);
            self.notify_selection_change();
        }
    }

    /// Move selection down
    pub fn select_next(&self) {
        let items = self.items.borrow();
        if items.is_empty() {
            return;
        }

        let mut selected = self.selected.borrow_mut();
        if *selected < items.len() - 1 {
            items[*selected].set_selected(false);
            *selected += 1;
            items[*selected].set_selected(true);
            let idx = *selected;
            drop(selected);
            drop(items);
            self.scroll_to_selected(idx);
            self.notify_selection_change();
        }
    }

    /// Get the currently selected item ID
    pub fn selected_id(&self) -> Option<String> {
        let items = self.items.borrow();
        let selected = *self.selected.borrow();
        items.get(selected).map(|item| item.id().to_string())
    }

    /// Reset selection to first item (call when starting a new search)
    pub fn reset_selection(&self) {
        let items = self.items.borrow();
        let mut selected = self.selected.borrow_mut();

        if *selected < items.len() {
            items[*selected].set_selected(false);
        }

        *selected = 0;
        if !items.is_empty() {
            items[0].set_selected(true);
        }

        *self.selected_action.borrow_mut() = -1;

        drop(selected);
        drop(items);
        self.notify_selection_change();
    }

    /// Scroll to ensure the selected item is visible
    fn scroll_to_selected(&self, index: usize) {
        let items = self.items.borrow();
        if let Some(item) = items.get(index) {
            let widget = item.widget();

            let adj = self.scrolled.vadjustment();
            let page_size = adj.page_size();
            let current = adj.value();

            let point = gtk4::graphene::Point::new(0.0, 0.0);
            if let Some(target_point) = widget.compute_point(&self.list_box, &point) {
                let target_y = f64::from(target_point.y());
                let item_height = f64::from(widget.height());

                if target_y < current {
                    adj.set_value(target_y);
                } else if target_y + item_height > current + page_size {
                    adj.set_value(target_y + item_height - page_size);
                }
            }
        }
    }

    /// Clear all results
    pub fn clear(&self) {
        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }
        self.items.borrow_mut().clear();
        *self.selected.borrow_mut() = 0;
        // Note: Visibility is managed by window.rs based on view mode, not here
    }

    /// Check if the list is empty
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.items.borrow().is_empty()
    }

    /// Get the number of items
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.items.borrow().len()
    }

    /// Get a clone of the current results
    pub fn results(&self) -> Vec<SearchResult> {
        self.results.borrow().clone()
    }

    /// Get the underlying GTK widget
    pub fn widget(&self) -> &gtk4::Box {
        &self.container
    }

    // NOTE: These methods are for keyboard control of sliders/switches
    // Not yet wired up in window.rs - kept for future integration

    /// Adjust slider on selected item (for keyboard control)
    /// Returns true if the selected item is a slider
    #[allow(dead_code)]
    pub fn adjust_selected_slider(&self, direction: i32) -> bool {
        let items = self.items.borrow();
        let selected = *self.selected.borrow();
        if let Some(item) = items.get(selected)
            && item.is_slider()
        {
            item.adjust_slider(direction);
            return true;
        }
        false
    }

    /// Toggle switch on selected item (for keyboard control)
    /// Returns true if the selected item is a switch
    #[allow(dead_code)]
    pub fn toggle_selected_switch(&self) -> bool {
        let items = self.items.borrow();
        let selected = *self.selected.borrow();
        if let Some(item) = items.get(selected)
            && item.is_switch()
        {
            item.toggle_switch();
            return true;
        }
        false
    }

    /// Check if the selected item is a slider
    #[allow(dead_code)]
    pub fn selected_is_slider(&self) -> bool {
        let items = self.items.borrow();
        let selected = *self.selected.borrow();
        items
            .get(selected)
            .is_some_and(super::result_item::ResultItem::is_slider)
    }

    /// Check if the selected item is a switch
    #[allow(dead_code)]
    pub fn selected_is_switch(&self) -> bool {
        let items = self.items.borrow();
        let selected = *self.selected.borrow();
        items
            .get(selected)
            .is_some_and(super::result_item::ResultItem::is_switch)
    }

    /// Get the currently selected result
    pub fn selected_result(&self) -> Option<SearchResult> {
        let results = self.results.borrow();
        let selected = *self.selected.borrow();
        results.get(selected).cloned()
    }

    /// Set the highlighted action index on the selected item
    /// Pass -1 to clear action selection
    pub fn set_selected_action(&self, action_index: i32) {
        *self.selected_action.borrow_mut() = action_index;
        let items = self.items.borrow();
        let selected = *self.selected.borrow();
        if let Some(item) = items.get(selected) {
            item.set_focused_action(action_index);
        }
    }

    /// Get the currently selected action index (-1 if none)
    #[allow(dead_code)]
    pub fn selected_action_index(&self) -> i32 {
        *self.selected_action.borrow()
    }
}

impl Default for ResultList {
    fn default() -> Self {
        Self::new()
    }
}

impl AsRef<gtk4::Widget> for ResultList {
    fn as_ref(&self) -> &gtk4::Widget {
        self.container.upcast_ref()
    }
}

/// Generate CSS for result list styling
pub fn result_list_css(theme: &crate::config::Theme) -> String {
    let colors = &theme.colors;
    format!(
        r"
        box.results-container {{
            margin-top: {margin_top}px;
            margin-left: {margin_side}px;
            margin-right: {margin_side}px;
            margin-bottom: {margin_side}px;
            padding-top: {padding}px;
            padding-bottom: {padding}px;
            background: {surface_container_low};
            background-color: {surface_container_low};
            border: {border}px solid alpha({outline}, 0.18);
            border-radius: {radius}px;
            box-shadow: inset 0 {shadow_y}px rgba(255, 255, 255, 0.08), inset 0 -{shadow_y}px rgba(0, 0, 0, 0.28);
        }}

        box.results-list {{
            background: transparent;
            background-color: transparent;
            padding-bottom: {padding}px;
        }}

        scrolledwindow {{
            background: transparent;
            background-color: transparent;
        }}

        scrolledwindow > viewport {{
            background: transparent;
            background-color: transparent;
        }}
        ",
        margin_top = theme.scaled(design::spacing::XS), // 4px
        margin_side = theme.scaled(design::spacing::SM), // 8px (mapped from 6, nearest SM=8)
        padding = theme.scaled(design::result_list::CONTAINER_PADDING),
        surface_container_low = colors.surface_container_low,
        outline = colors.outline,
        border = theme.scaled(1),
        radius = theme.scaled(design::rounding::SMALL),
        shadow_y = theme.scaled(1),
    )
}
