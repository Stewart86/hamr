//! Virtualized result grid container widget
//!
//! Uses GTK4's `GridView` with `SignalListItemFactory` for efficient rendering.
//! Only visible items are created - critical for large datasets like emoji.

use super::design;
use super::grid_item::GridItem;
use super::result_object::ResultObject;
use crate::config::Theme;
use gtk4::gio;
use gtk4::glib;
use gtk4::glib::SourceId;
use gtk4::prelude::*;
use gtk4::{Orientation, PolicyType, SignalListItemFactory};
use hamr_rpc::SearchResult;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;
use tracing::{debug, debug_span};

/// Delay after scroll stops before re-enabling hover (ms)
const SCROLL_HOVER_DELAY_MS: u64 = 150;

/// Default number of columns
const DEFAULT_COLUMNS: u32 = 5;

/// Callback type for item selection (activation)
pub type SelectCallback = Box<dyn Fn(&str)>;
/// Callback type for action button clicks
pub type ActionCallback = Box<dyn Fn(&str, &str)>;
/// Callback type for selection change (keyboard navigation)
pub type SelectionChangeCallback = Box<dyn Fn(Option<&SearchResult>)>;
/// Callback type for action buttons in grid items (shared via Rc)
type SharedActionCallback = Rc<dyn Fn(&str, &str)>;

/// Container for grid items with virtualized rendering
pub struct ResultGrid {
    container: gtk4::Box,
    scrolled: gtk4::ScrolledWindow,
    grid_view: gtk4::GridView,
    model: gio::ListStore,
    selection_model: gtk4::SingleSelection,
    on_select: Rc<RefCell<Option<SelectCallback>>>,
    on_action: Rc<RefCell<Option<ActionCallback>>>,
    on_selection_change: Rc<RefCell<Option<SelectionChangeCallback>>>,
    max_height: Rc<RefCell<i32>>,
    columns: Rc<RefCell<u32>>,
    grid_items: Rc<RefCell<HashMap<u32, Rc<GridItem>>>>,
    selected_action: Rc<RefCell<i32>>,
    /// Cached results for selection change callback
    results: Rc<RefCell<Vec<SearchResult>>>,
}

impl ResultGrid {
    pub fn new(theme: &Theme) -> Self {
        Self::with_columns(DEFAULT_COLUMNS, theme)
    }

    pub fn with_columns(columns: u32, theme: &Theme) -> Self {
        let theme = Rc::new(RefCell::new(theme.clone()));
        let container = gtk4::Box::builder()
            .orientation(Orientation::Vertical)
            .css_classes(["result-grid-container"])
            .visible(false)
            .build();

        let model = gio::ListStore::new::<ResultObject>();

        let selection_model = gtk4::SingleSelection::new(Some(model.clone()));
        selection_model.set_autoselect(true);
        selection_model.set_can_unselect(false);

        let on_action: Rc<RefCell<Option<ActionCallback>>> = Rc::new(RefCell::new(None));
        let grid_items: Rc<RefCell<HashMap<u32, Rc<GridItem>>>> =
            Rc::new(RefCell::new(HashMap::new()));
        let on_select: Rc<RefCell<Option<SelectCallback>>> = Rc::new(RefCell::new(None));
        let factory = Self::create_factory(
            &on_action,
            &grid_items,
            selection_model.clone(),
            on_select.clone(),
            &theme,
        );

        let grid_view = gtk4::GridView::builder()
            .model(&selection_model)
            .factory(&factory)
            .min_columns(1)
            .max_columns(columns)
            .single_click_activate(false)
            .build();

        grid_view.add_css_class("result-grid");

        let scrolled = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(PolicyType::Never)
            .vscrollbar_policy(PolicyType::Automatic)
            .propagate_natural_height(true)
            .child(&grid_view)
            .build();

        container.append(&scrolled);

        let scroll_timer: Rc<RefCell<Option<SourceId>>> = Rc::new(RefCell::new(None));

        {
            let grid_view_ref = grid_view.clone();
            let scroll_timer_ref = scroll_timer.clone();

            scrolled.vadjustment().connect_value_changed(move |_| {
                grid_view_ref.add_css_class("scrolling");

                if let Some(source_id) = scroll_timer_ref.borrow_mut().take() {
                    source_id.remove();
                }

                let grid_view_inner = grid_view_ref.clone();
                let scroll_timer_inner = scroll_timer_ref.clone();

                let source_id = glib::timeout_add_local_once(
                    Duration::from_millis(SCROLL_HOVER_DELAY_MS),
                    move || {
                        scroll_timer_inner.borrow_mut().take();
                        grid_view_inner.remove_css_class("scrolling");
                    },
                );

                *scroll_timer_ref.borrow_mut() = Some(source_id);
            });
        }

        Self {
            container,
            scrolled,
            grid_view,
            model,
            selection_model,
            on_select,
            on_action,
            on_selection_change: Rc::new(RefCell::new(None)),
            max_height: Rc::new(RefCell::new(600)),
            columns: Rc::new(RefCell::new(columns)),
            grid_items,
            selected_action: Rc::new(RefCell::new(-1)),
            results: Rc::new(RefCell::new(Vec::new())),
        }
    }

    fn create_factory(
        on_action: &Rc<RefCell<Option<ActionCallback>>>,
        grid_items: &Rc<RefCell<HashMap<u32, Rc<GridItem>>>>,
        selection_model: gtk4::SingleSelection,
        on_select: Rc<RefCell<Option<SelectCallback>>>,
        theme: &Rc<RefCell<Theme>>,
    ) -> SignalListItemFactory {
        let factory = SignalListItemFactory::new();

        factory.connect_setup(move |_, list_item| {
            let list_item = list_item
                .downcast_ref::<gtk4::ListItem>()
                .expect("ListItem expected");

            list_item.connect_selected_notify(|item| unsafe {
                if let Some(grid_item) = item.data::<Rc<GridItem>>("grid-item") {
                    let grid_item = grid_item.as_ref();
                    grid_item.set_selected(item.is_selected());
                }
            });
        });

        let on_action_bind = Rc::clone(on_action);
        let grid_items_bind = Rc::clone(grid_items);
        let theme_bind = theme.clone();
        factory.connect_bind(move |_, list_item| {
            let list_item = list_item
                .downcast_ref::<gtk4::ListItem>()
                .expect("ListItem expected");

            let Some(result_obj) = list_item.item().and_downcast::<ResultObject>() else {
                return;
            };

            let Some(result) = result_obj.data() else {
                return;
            };

            let position = list_item.position();

            let theme = theme_bind.borrow();
            let grid_item = Rc::new(GridItem::new(&result, list_item.is_selected(), &theme));
            let item_id = result.id.clone();
            let on_action_local = on_action_bind.clone();
            let action_cb: SharedActionCallback = Rc::new(move |item_id, action_id| {
                if let Some(ref cb) = *on_action_local.borrow() {
                    cb(item_id, action_id);
                }
            });
            grid_item.connect_action_clicked(&action_cb);

            let selection_model_click = selection_model.clone();
            let on_select_click = on_select.clone();
            let list_item_weak = list_item.downgrade();
            let gesture = gtk4::GestureClick::new();
            gesture.connect_released(move |_, _, _, _| {
                if let Some(list_item) = list_item_weak.upgrade() {
                    let pos = list_item.position();
                    selection_model_click.set_selected(pos);
                    if let Some(ref cb) = *on_select_click.borrow() {
                        cb(&item_id);
                    }
                }
            });
            grid_item.widget().add_controller(gesture);

            list_item.set_child(Some(grid_item.widget()));
            unsafe {
                list_item.set_data("grid-item", grid_item.clone());
            }
            grid_items_bind.borrow_mut().insert(position, grid_item);
        });

        let grid_items_unbind = grid_items.clone();
        factory.connect_unbind(move |_, list_item| {
            let list_item = list_item
                .downcast_ref::<gtk4::ListItem>()
                .expect("ListItem expected");

            grid_items_unbind.borrow_mut().remove(&list_item.position());
            list_item.set_child(None::<&gtk4::Widget>);
        });

        factory
    }

    pub fn set_max_height(&self, height: i32) {
        *self.max_height.borrow_mut() = height;
        self.scrolled.set_max_content_height(height);
        // min_content_height is dynamically calculated in set_results() based on actual content
    }

    pub fn set_columns(&self, columns: u32) {
        *self.columns.borrow_mut() = columns;
        self.grid_view.set_max_columns(columns);
        self.grid_view.set_min_columns(columns);
    }

    /// Set the spacing between grid items (no-op, spacing handled via CSS)
    #[allow(clippy::unused_self)] // Public API kept for interface consistency
    pub fn set_spacing(&self, _spacing: u32) {
        // GridView spacing is controlled via CSS on `gridview > child`
    }

    pub fn connect_select<F: Fn(&str) + 'static>(&self, f: F) {
        *self.on_select.borrow_mut() = Some(Box::new(f));

        let on_select = self.on_select.clone();
        let model = self.model.clone();
        self.grid_view.connect_activate(move |_, position| {
            if let Some(item) = model.item(position).and_downcast::<ResultObject>()
                && let Some(ref cb) = *on_select.borrow()
            {
                cb(&item.id());
            }
        });
    }

    pub fn connect_action<F: Fn(&str, &str) + 'static>(&self, f: F) {
        *self.on_action.borrow_mut() = Some(Box::new(f));
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
            let selected = self.selection_model.selected() as usize;
            let result = results.get(selected);
            cb(result);
        }
    }

    pub fn set_results(&self, results: &[SearchResult]) {
        self.set_results_impl(results, true);
    }

    pub fn set_results_with_selection(&self, results: &[SearchResult], reset_selection: bool) {
        self.set_results_impl(results, reset_selection);
    }

    fn set_results_impl(&self, results: &[SearchResult], reset_selection: bool) {
        let _span = debug_span!("ResultGrid::set_results", count = results.len()).entered();
        debug!(count = results.len(), "full rebuild, reset_selection={}", reset_selection);
        self.model.remove_all();
        self.grid_items.borrow_mut().clear();

        *self.results.borrow_mut() = results.to_vec();

        for result in results {
            let obj = ResultObject::new(result.clone());
            self.model.append(&obj);
        }

        if !results.is_empty()
            && reset_selection {
                self.selection_model.set_selected(0);
                self.scroll_to_selected();
            }

        *self.selected_action.borrow_mut() = -1;

        let max_height = *self.max_height.borrow();
        self.scrolled.set_max_content_height(max_height);

        if results.is_empty() {
            self.scrolled.set_min_content_height(0);
        } else {
            let (_, natural_height, _, _) = self.grid_view.measure(gtk4::Orientation::Vertical, -1);
            let min_height = natural_height.min(max_height);
            self.scrolled.set_min_content_height(min_height);
        }

        // Notify about initial selection
        self.notify_selection_change();
    }

    pub fn update_results_diff_with_selection(&self, results: &[SearchResult], reset_selection: bool) -> bool {
        self.update_results_diff_impl(results, reset_selection)
    }

    fn update_results_diff_impl(&self, results: &[SearchResult], reset_selection: bool) -> bool {
        let _span = debug_span!("ResultGrid::update_results_diff", count = results.len()).entered();
        if self.model.n_items() == 0 || results.is_empty() {
            debug!(reason = "empty", "falling back to full rebuild");
            self.set_results_with_selection(results, reset_selection);
            return true;
        }

        let current_ids: Vec<String> = (0..self.model.n_items())
            .filter_map(|i| self.model.item(i).and_downcast::<ResultObject>())
            .map(|obj| obj.id())
            .collect();

        let new_ids: Vec<&str> = results.iter().map(|r| r.id.as_str()).collect();

        if current_ids.len() != new_ids.len()
            || !current_ids.iter().zip(new_ids.iter()).all(|(a, b)| a == *b)
        {
            debug!(reason = "ids_changed", "falling back to full rebuild");
            self.set_results(results);
            return true;
        }

        debug!(count = results.len(), "incremental update");
        for (i, result) in results.iter().enumerate() {
            if let Some(obj) = self.model.item(i as u32).and_downcast::<ResultObject>() {
                obj.set_data(result.clone());
            }
        }

        // Update cached results
        *self.results.borrow_mut() = results.to_vec();

        false
    }

    pub fn select_up(&self) {
        let columns = *self.columns.borrow();
        let current = self.selection_model.selected();
        if current >= columns {
            self.selection_model.set_selected(current - columns);
            self.scroll_to_selected();
            self.notify_selection_change();
        }
    }

    pub fn select_down(&self) {
        let columns = *self.columns.borrow();
        let current = self.selection_model.selected();
        let n_items = self.model.n_items();
        if current + columns < n_items {
            self.selection_model.set_selected(current + columns);
            self.scroll_to_selected();
            self.notify_selection_change();
        }
    }

    pub fn select_left(&self) {
        let current = self.selection_model.selected();
        if current > 0 {
            self.selection_model.set_selected(current - 1);
            self.scroll_to_selected();
            self.notify_selection_change();
        }
    }

    pub fn select_right(&self) {
        let current = self.selection_model.selected();
        let n_items = self.model.n_items();
        if current + 1 < n_items {
            self.selection_model.set_selected(current + 1);
            self.scroll_to_selected();
            self.notify_selection_change();
        }
    }

    pub fn selected_id(&self) -> Option<String> {
        let pos = self.selection_model.selected();
        self.model
            .item(pos)
            .and_downcast::<ResultObject>()
            .map(|obj| obj.id())
    }

    pub fn selected_result(&self) -> Option<SearchResult> {
        let pos = self.selection_model.selected();
        self.model
            .item(pos)
            .and_downcast::<ResultObject>()
            .and_then(|obj| obj.data())
    }

    pub fn reset_selection(&self) {
        if self.model.n_items() > 0 {
            self.selection_model.set_selected(0);
            self.scroll_to_selected();
            self.notify_selection_change();
        }
    }

    fn scroll_to_selected(&self) {
        let pos = self.selection_model.selected();
        self.grid_view
            .scroll_to(pos, gtk4::ListScrollFlags::FOCUS, None);
    }

    pub fn clear(&self) {
        self.model.remove_all();
    }

    pub fn results(&self) -> Vec<SearchResult> {
        (0..self.model.n_items())
            .filter_map(|i| {
                self.model
                    .item(i)
                    .and_downcast::<ResultObject>()
                    .and_then(|obj| obj.data())
            })
            .collect()
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.container
    }

    pub fn set_selected_action(&self, action_index: i32) {
        let grid_items = self.grid_items.borrow_mut();

        for grid_item in grid_items.values() {
            grid_item.set_focused_action(-1);
        }

        *self.selected_action.borrow_mut() = action_index;

        if action_index < 0 {
            return;
        }

        let selected = self.selection_model.selected();
        if let Some(grid_item) = grid_items.get(&selected) {
            grid_item.set_focused_action(action_index);
        }
    }
}

impl AsRef<gtk4::Widget> for ResultGrid {
    fn as_ref(&self) -> &gtk4::Widget {
        self.container.upcast_ref()
    }
}

/// Generate CSS for result grid styling
// CSS template - splitting would scatter related style rules
#[allow(clippy::too_many_lines)]
pub fn result_grid_css(theme: &crate::config::Theme) -> String {
    let colors = &theme.colors;
    format!(
        r"
        /* Result Grid Container */
        box.result-grid-container {{
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

        /* GridView */
        .result-grid {{
            background: transparent;
            background-color: transparent;
            border-spacing: 0px;
        }}

        /* Disable hover highlight during scroll */
        .result-grid.scrolling .grid-item-simple:hover,
        .result-grid.scrolling .grid-item-simple.hovering {{
            background-color: transparent;
            background: transparent;
        }}

        scrolledwindow {{
            background: transparent;
            background-color: transparent;
        }}

        scrolledwindow > viewport {{
            background: transparent;
            background-color: transparent;
        }}

        gridview > child:focus {{
            outline: none;
        }}

        /* Visual placeholder - fixed size */
        .grid-item-visual-placeholder {{
            min-width: {visual_size}px;
            min-height: {visual_size}px;
        }}

        /* Grid item container */
        .grid-item-simple {{
            border-radius: {item_radius}px;
            min-width: {visual_size}px;
            min-height: {visual_size}px;
            padding: {item_padding}px;
            background: transparent;
            background-color: transparent;
            border: {border}px solid transparent;
            transition: background-color 180ms cubic-bezier(0.25, 0.1, 0.25, 1),
                        background 180ms cubic-bezier(0.25, 0.1, 0.25, 1),
                        border-color 180ms cubic-bezier(0.25, 0.1, 0.25, 1);
        }}

        .grid-item-simple:hover,
        .grid-item-simple.hovering {{
            background-color: {primary_container};
            background: {primary_container};
        }}

        .grid-item-simple.selected {{
            background: linear-gradient(to bottom, rgba(149, 144, 136, 0.08), {surface_dark});
            background-color: {surface_dark};
            border: {border}px solid alpha({outline}, 0.28);
        }}

        .grid-item-simple.selected:hover,
        .grid-item-simple.selected.hovering {{
            background: linear-gradient(to bottom, rgba(149, 144, 136, 0.08), {surface_dark});
            background-color: {surface_high};
            border: {border}px solid alpha({outline}, 0.28);
        }}

        .result-grid.scrolling .grid-item-simple.selected:hover,
        .result-grid.scrolling .grid-item-simple.selected.hovering {{
            background: linear-gradient(to bottom, rgba(149, 144, 136, 0.08), {surface_dark});
            background-color: {surface_dark};
            border: {border}px solid alpha({outline}, 0.28);
        }}

        .grid-item-simple label {{
            padding-top: {label_padding_top}px;
        }}

        /* Action buttons overlay - hidden by default, shown on hover */
        .grid-item-actions {{
            opacity: 0;
            transition: opacity 150ms ease-in-out;
        }}

        .grid-item-actions .ripple-button:hover {{
            background-color: {primary_container};
        }}

        .grid-item-simple:hover ~ .grid-item-actions,
        .grid-item-simple.hovering ~ .grid-item-actions,
        .grid-item-simple.selected ~ .grid-item-actions {{
            opacity: 1;
        }}
        ",
        surface_container_low = colors.surface_container_low,
        outline = colors.outline,
        margin_top = theme.scaled(design::spacing::XS), // 4px
        margin_side = theme.scaled(design::spacing::SM), // 8px (mapped from 6, nearest SM=8)
        padding = theme.scaled(design::result_list::CONTAINER_PADDING),
        border = theme.scaled(1),
        radius = theme.scaled(design::rounding::SMALL),
        shadow_y = theme.scaled(1),
        visual_size = theme.scaled(design::grid::VISUAL_SIZE),
        item_radius = theme.scaled(design::radius::SM), // 8px
        item_padding = theme.scaled(design::spacing::SM), // 8px
        surface_high = colors.surface_container_high,
        surface_dark = colors.surface,
        primary_container = colors.primary_container,
        label_padding_top = theme.scaled(design::spacing::XS), // 4px
    )
}
