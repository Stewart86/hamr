//! Unified result view abstraction
//!
//! Provides a single interface for displaying search results in either
//! list or grid mode. Only one view exists in memory at a time.

use super::result_grid::ResultGrid;
use super::result_list::ResultList;
use crate::config::Theme;
use crate::window::ResultViewMode;
use gtk4::prelude::*;
use hamr_rpc::SearchResult;
use std::cell::RefCell;
use std::rc::Rc;

/// Callback types
pub type SelectCallback = Box<dyn Fn(&str)>;
pub type ActionCallback = Box<dyn Fn(&str, &str)>;
pub type SliderCallback = Box<dyn Fn(&str, f64)>;
pub type SwitchCallback = Box<dyn Fn(&str, bool)>;
pub type SelectionChangeCallback = Box<dyn Fn(Option<&SearchResult>)>;

/// Unified result view that can be either List or Grid mode
pub struct ResultView {
    mode: ResultViewMode,
    container: gtk4::Box,
    list: Option<Rc<ResultList>>,
    grid: Option<Rc<ResultGrid>>,
    // Callbacks stored for reconnection when switching views
    on_select: Rc<RefCell<Option<SelectCallback>>>,
    on_action: Rc<RefCell<Option<ActionCallback>>>,
    on_slider: Rc<RefCell<Option<SliderCallback>>>,
    on_switch: Rc<RefCell<Option<SwitchCallback>>>,
    on_selection_change: Rc<RefCell<Option<SelectionChangeCallback>>>,
    // Config
    max_height: i32,
    grid_columns: u32,
    grid_spacing: u32,
    /// Cached theme for creating new views when switching modes
    theme: Rc<RefCell<Theme>>,
}

impl ResultView {
    pub fn new(mode: ResultViewMode, theme: &Theme) -> Self {
        let container = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .build();

        let theme = Rc::new(RefCell::new(theme.clone()));

        let mut view = Self {
            mode,
            container,
            list: None,
            grid: None,
            on_select: Rc::new(RefCell::new(None)),
            on_action: Rc::new(RefCell::new(None)),
            on_slider: Rc::new(RefCell::new(None)),
            on_switch: Rc::new(RefCell::new(None)),
            on_selection_change: Rc::new(RefCell::new(None)),
            max_height: 400,
            grid_columns: 4,
            grid_spacing: 8,
            theme,
        };

        view.create_view(mode);
        view
    }

    fn create_view(&mut self, mode: ResultViewMode) {
        let theme = self.theme.borrow();
        match mode {
            ResultViewMode::List => {
                let list = Rc::new(ResultList::new());
                list.set_max_height(self.max_height);
                self.container.append(list.widget());
                self.reconnect_list_callbacks(&list);
                self.list = Some(list);
            }
            ResultViewMode::Grid => {
                let grid = Rc::new(ResultGrid::new(&theme));
                grid.set_max_height(self.max_height);
                grid.set_columns(self.grid_columns);
                grid.set_spacing(self.grid_spacing);
                self.container.append(grid.widget());
                self.reconnect_grid_callbacks(&grid);
                self.grid = Some(grid);
            }
        }
    }

    fn reconnect_list_callbacks(&self, list: &Rc<ResultList>) {
        if self.on_select.borrow().is_some() {
            let on_select = self.on_select.clone();
            list.connect_select(move |id| {
                if let Some(ref cb) = *on_select.borrow() {
                    cb(id);
                }
            });
        }
        if self.on_action.borrow().is_some() {
            let on_action = self.on_action.clone();
            list.connect_action(move |id, action| {
                if let Some(ref cb) = *on_action.borrow() {
                    cb(id, action);
                }
            });
        }
        if self.on_slider.borrow().is_some() {
            let on_slider = self.on_slider.clone();
            list.connect_slider(move |id, val| {
                if let Some(ref cb) = *on_slider.borrow() {
                    cb(id, val);
                }
            });
        }
        if self.on_switch.borrow().is_some() {
            let on_switch = self.on_switch.clone();
            list.connect_switch(move |id, val| {
                if let Some(ref cb) = *on_switch.borrow() {
                    cb(id, val);
                }
            });
        }
        if self.on_selection_change.borrow().is_some() {
            let on_selection_change = self.on_selection_change.clone();
            list.connect_selection_change(move |result| {
                if let Some(ref cb) = *on_selection_change.borrow() {
                    cb(result);
                }
            });
        }
    }

    fn reconnect_grid_callbacks(&self, grid: &Rc<ResultGrid>) {
        if self.on_select.borrow().is_some() {
            let on_select = self.on_select.clone();
            grid.connect_select(move |id| {
                if let Some(ref cb) = *on_select.borrow() {
                    cb(id);
                }
            });
        }
        if self.on_action.borrow().is_some() {
            let on_action = self.on_action.clone();
            grid.connect_action(move |id, action| {
                if let Some(ref cb) = *on_action.borrow() {
                    cb(id, action);
                }
            });
        }
        if self.on_selection_change.borrow().is_some() {
            let on_selection_change = self.on_selection_change.clone();
            grid.connect_selection_change(move |result| {
                if let Some(ref cb) = *on_selection_change.borrow() {
                    cb(result);
                }
            });
        }
    }

    /// Switch to a different view mode
    pub fn set_mode(&mut self, mode: ResultViewMode) {
        if self.mode == mode {
            return;
        }

        // Store current results before switching
        let results = self.results();

        // Remove old view from container
        match self.mode {
            ResultViewMode::List => {
                if let Some(ref list) = self.list {
                    self.container.remove(list.widget());
                }
                self.list = None;
            }
            ResultViewMode::Grid => {
                if let Some(ref grid) = self.grid {
                    self.container.remove(grid.widget());
                }
                self.grid = None;
            }
        }

        // Create new view
        self.mode = mode;
        self.create_view(mode);

        // Restore results
        if !results.is_empty() {
            let theme = self.theme.borrow();
            self.set_results(&results, &theme);
            self.widget().set_visible(true);
        }
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.container
    }

    // Callback setters - always connect to current view
    pub fn connect_select<F: Fn(&str) + 'static>(&self, f: F) {
        *self.on_select.borrow_mut() = Some(Box::new(f));
        // Reconnect to current view
        match self.mode {
            ResultViewMode::List => {
                if let Some(ref list) = self.list {
                    let on_select = self.on_select.clone();
                    list.connect_select(move |id| {
                        if let Some(ref cb) = *on_select.borrow() {
                            cb(id);
                        }
                    });
                }
            }
            ResultViewMode::Grid => {
                if let Some(ref grid) = self.grid {
                    let on_select = self.on_select.clone();
                    grid.connect_select(move |id| {
                        if let Some(ref cb) = *on_select.borrow() {
                            cb(id);
                        }
                    });
                }
            }
        }
    }

    pub fn connect_action<F: Fn(&str, &str) + 'static>(&self, f: F) {
        *self.on_action.borrow_mut() = Some(Box::new(f));
        match self.mode {
            ResultViewMode::List => {
                if let Some(ref list) = self.list {
                    let on_action = self.on_action.clone();
                    list.connect_action(move |id, action| {
                        if let Some(ref cb) = *on_action.borrow() {
                            cb(id, action);
                        }
                    });
                }
            }
            ResultViewMode::Grid => {
                if let Some(ref grid) = self.grid {
                    let on_action = self.on_action.clone();
                    grid.connect_action(move |id, action| {
                        if let Some(ref cb) = *on_action.borrow() {
                            cb(id, action);
                        }
                    });
                }
            }
        }
    }

    pub fn connect_slider<F: Fn(&str, f64) + 'static>(&self, f: F) {
        *self.on_slider.borrow_mut() = Some(Box::new(f));
        if let ResultViewMode::List = self.mode
            && let Some(ref list) = self.list
        {
            let on_slider = self.on_slider.clone();
            list.connect_slider(move |id, val| {
                if let Some(ref cb) = *on_slider.borrow() {
                    cb(id, val);
                }
            });
        }
    }

    pub fn connect_switch<F: Fn(&str, bool) + 'static>(&self, f: F) {
        *self.on_switch.borrow_mut() = Some(Box::new(f));
        if let ResultViewMode::List = self.mode
            && let Some(ref list) = self.list
        {
            let on_switch = self.on_switch.clone();
            list.connect_switch(move |id, val| {
                if let Some(ref cb) = *on_switch.borrow() {
                    cb(id, val);
                }
            });
        }
    }

    /// Set the callback for selection changes (keyboard navigation).
    /// Called when the highlighted item changes, not on activation.
    pub fn connect_selection_change<F: Fn(Option<&SearchResult>) + 'static>(&self, f: F) {
        *self.on_selection_change.borrow_mut() = Some(Box::new(f));
        match self.mode {
            ResultViewMode::List => {
                if let Some(ref list) = self.list {
                    let on_selection_change = self.on_selection_change.clone();
                    list.connect_selection_change(move |result| {
                        if let Some(ref cb) = *on_selection_change.borrow() {
                            cb(result);
                        }
                    });
                }
            }
            ResultViewMode::Grid => {
                if let Some(ref grid) = self.grid {
                    let on_selection_change = self.on_selection_change.clone();
                    grid.connect_selection_change(move |result| {
                        if let Some(ref cb) = *on_selection_change.borrow() {
                            cb(result);
                        }
                    });
                }
            }
        }
    }

    // Data methods - delegate to current view
    pub fn set_results(&self, results: &[SearchResult], theme: &Theme) {
        self.set_results_with_selection(results, theme, true);
    }

    pub fn set_results_with_selection(
        &self,
        results: &[SearchResult],
        theme: &Theme,
        reset_selection: bool,
    ) {
        let has_results = !results.is_empty();
        match self.mode {
            ResultViewMode::List => {
                if let Some(ref list) = self.list {
                    list.set_results_with_selection(results, theme, reset_selection);
                    list.widget().set_visible(has_results);
                }
            }
            ResultViewMode::Grid => {
                if let Some(ref grid) = self.grid {
                    grid.set_results_with_selection(results, reset_selection);
                    grid.widget().set_visible(has_results);
                }
            }
        }
    }

    pub fn update_results_diff(&self, results: &[SearchResult], theme: &Theme) {
        self.update_results_diff_with_selection(results, theme, true);
    }

    pub fn update_results_diff_with_selection(
        &self,
        results: &[SearchResult],
        theme: &Theme,
        reset_selection: bool,
    ) {
        match self.mode {
            ResultViewMode::List => {
                if let Some(ref list) = self.list {
                    list.update_results_diff_with_selection(results, theme, reset_selection);
                }
            }
            ResultViewMode::Grid => {
                if let Some(ref grid) = self.grid {
                    grid.update_results_diff_with_selection(results, reset_selection);
                }
            }
        }
    }

    pub fn clear(&self) {
        match self.mode {
            ResultViewMode::List => {
                if let Some(ref list) = self.list {
                    list.clear();
                    list.widget().set_visible(false);
                }
            }
            ResultViewMode::Grid => {
                if let Some(ref grid) = self.grid {
                    grid.clear();
                    grid.widget().set_visible(false);
                }
            }
        }
    }

    pub fn results(&self) -> Vec<SearchResult> {
        match self.mode {
            ResultViewMode::List => self.list.as_ref().map(|l| l.results()).unwrap_or_default(),
            ResultViewMode::Grid => self.grid.as_ref().map(|g| g.results()).unwrap_or_default(),
        }
    }

    // Selection methods
    pub fn selected_id(&self) -> Option<String> {
        match self.mode {
            ResultViewMode::List => self.list.as_ref().and_then(|l| l.selected_id()),
            ResultViewMode::Grid => self.grid.as_ref().and_then(|g| g.selected_id()),
        }
    }

    pub fn selected_result(&self) -> Option<SearchResult> {
        match self.mode {
            ResultViewMode::List => self.list.as_ref().and_then(|l| l.selected_result()),
            ResultViewMode::Grid => self.grid.as_ref().and_then(|g| g.selected_result()),
        }
    }

    pub fn reset_selection(&self) {
        match self.mode {
            ResultViewMode::List => {
                if let Some(ref list) = self.list {
                    list.reset_selection();
                }
            }
            ResultViewMode::Grid => {
                if let Some(ref grid) = self.grid {
                    grid.reset_selection();
                }
            }
        }
    }

    // Navigation
    pub fn select_up(&self) {
        match self.mode {
            ResultViewMode::List => {
                if let Some(ref list) = self.list {
                    list.select_prev();
                }
            }
            ResultViewMode::Grid => {
                if let Some(ref grid) = self.grid {
                    grid.select_up();
                }
            }
        }
    }

    pub fn select_down(&self) {
        match self.mode {
            ResultViewMode::List => {
                if let Some(ref list) = self.list {
                    list.select_next();
                }
            }
            ResultViewMode::Grid => {
                if let Some(ref grid) = self.grid {
                    grid.select_down();
                }
            }
        }
    }

    pub fn select_left(&self) {
        if let ResultViewMode::Grid = self.mode
            && let Some(ref grid) = self.grid
        {
            grid.select_left();
        }
    }

    pub fn select_right(&self) {
        if let ResultViewMode::Grid = self.mode
            && let Some(ref grid) = self.grid
        {
            grid.select_right();
        }
    }

    // Action selection
    pub fn set_selected_action(&self, index: i32) {
        match self.mode {
            ResultViewMode::List => {
                if let Some(ref list) = self.list {
                    list.set_selected_action(index);
                }
            }
            ResultViewMode::Grid => {
                if let Some(ref grid) = self.grid {
                    grid.set_selected_action(index);
                }
            }
        }
    }

    // Config
    pub fn set_max_height(&mut self, height: i32) {
        self.max_height = height;
        if let Some(ref list) = self.list {
            list.set_max_height(height);
        }
        if let Some(ref grid) = self.grid {
            grid.set_max_height(height);
        }
    }

    pub fn set_grid_columns(&mut self, columns: u32) {
        self.grid_columns = columns;
        if let Some(ref grid) = self.grid {
            grid.set_columns(columns);
        }
    }

    pub fn set_grid_spacing(&mut self, spacing: u32) {
        self.grid_spacing = spacing;
        if let Some(ref grid) = self.grid {
            grid.set_spacing(spacing);
        }
    }

    // List-only methods
    pub fn refresh_running_apps(&self, compositor: &crate::compositor::Compositor) {
        if let Some(ref list) = self.list {
            list.refresh_running_apps(compositor);
        }
    }

    /// Check if the selected item is a switch (List mode only)
    pub fn selected_is_switch(&self) -> bool {
        if let ResultViewMode::List = self.mode
            && let Some(ref list) = self.list
        {
            return list.selected_is_switch();
        }
        false
    }

    /// Toggle the selected switch (List mode only)
    pub fn toggle_selected_switch(&self) {
        if let ResultViewMode::List = self.mode
            && let Some(ref list) = self.list
        {
            list.toggle_selected_switch();
        }
    }

    /// Check if the selected item is a slider (List mode only)
    pub fn selected_is_slider(&self) -> bool {
        if let ResultViewMode::List = self.mode
            && let Some(ref list) = self.list
        {
            return list.selected_is_slider();
        }
        false
    }

    /// Adjust the selected slider by one step (List mode only)
    /// Returns true if the selected item was a slider and was adjusted
    pub fn adjust_selected_slider(&self, direction: i32) -> bool {
        if let ResultViewMode::List = self.mode
            && let Some(ref list) = self.list
        {
            return list.adjust_selected_slider(direction);
        }
        false
    }
}
