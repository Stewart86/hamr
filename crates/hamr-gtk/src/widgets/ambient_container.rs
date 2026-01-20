//! Reusable ambient items container
//!
//! Used by both `ActionBar` and FAB to display ambient items with
//! consistent styling and behavior (diffing, dismiss, actions).

use super::AmbientItemWidget;
use gtk4::prelude::*;
use hamr_types::AmbientItem;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// Ambient item with plugin ID
#[derive(Debug, Clone)]
pub struct AmbientItemWithPlugin {
    pub item: AmbientItem,
    pub plugin_id: String,
}

/// Callback types
type AmbientActionCallback = Rc<RefCell<Option<Box<dyn Fn(&str, &str, &str)>>>>; // (plugin_id, item_id, action_id)
type AmbientDismissCallback = Rc<RefCell<Option<Box<dyn Fn(&str, &str)>>>>; // (plugin_id, item_id)

/// A container for ambient items with diffing support
pub struct AmbientItemsContainer {
    container: gtk4::Box,
    widgets: Rc<RefCell<HashMap<String, AmbientItemWidget>>>,
    on_action: AmbientActionCallback,
    on_dismiss: AmbientDismissCallback,
}

impl AmbientItemsContainer {
    /// Create a new ambient items container
    pub fn new() -> Self {
        let container = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(4)
            .css_classes(["ambient-items-container"])
            .build();

        Self {
            container,
            widgets: Rc::new(RefCell::new(HashMap::new())),
            on_action: Rc::new(RefCell::new(None)),
            on_dismiss: Rc::new(RefCell::new(None)),
        }
    }

    /// Get the underlying GTK widget
    pub fn widget(&self) -> &gtk4::Box {
        &self.container
    }

    /// Check if there are any items
    pub fn is_empty(&self) -> bool {
        self.widgets.borrow().is_empty()
    }

    /// Set ambient items with diffing (only update changed, add new, remove old)
    pub fn set_items(&self, items: &[AmbientItemWithPlugin]) {
        use std::collections::HashSet;

        let mut widgets = self.widgets.borrow_mut();

        let new_keys: HashSet<String> = items
            .iter()
            .map(|i| format!("{}:{}", i.plugin_id, i.item.id))
            .collect();

        // Remove items that are no longer present
        let keys_to_remove: Vec<String> = widgets
            .keys()
            .filter(|k| !new_keys.contains(*k))
            .cloned()
            .collect();

        for key in keys_to_remove {
            if let Some(widget) = widgets.remove(&key) {
                self.container.remove(widget.widget());
            }
        }

        // Update existing or add new items
        for item_with_plugin in items {
            let key = format!(
                "{}:{}",
                item_with_plugin.plugin_id, item_with_plugin.item.id
            );

            if let Some(widget) = widgets.get(&key) {
                // Update existing widget
                widget.update(&item_with_plugin.item);
            } else {
                // Create new widget
                let widget =
                    AmbientItemWidget::new(&item_with_plugin.item, &item_with_plugin.plugin_id);

                // Connect action callback
                let plugin_id = item_with_plugin.plugin_id.clone();
                let on_action = self.on_action.clone();
                widget.connect_action(move |item_id, action_id| {
                    if let Some(ref cb) = *on_action.borrow() {
                        cb(&plugin_id, item_id, action_id);
                    }
                });

                // Connect dismiss callback
                let plugin_id = item_with_plugin.plugin_id.clone();
                let on_dismiss = self.on_dismiss.clone();
                widget.connect_dismiss(move |item_id| {
                    if let Some(ref cb) = *on_dismiss.borrow() {
                        cb(&plugin_id, item_id);
                    }
                });

                self.container.append(widget.widget());
                widgets.insert(key, widget);
            }
        }
    }

    /// Clear all ambient items
    #[allow(dead_code)]
    pub fn clear(&self) {
        let mut widgets = self.widgets.borrow_mut();
        for (_, widget) in widgets.drain() {
            self.container.remove(widget.widget());
        }
    }

    /// Connect ambient action callback (`plugin_id`, `item_id`, `action_id`)
    pub fn connect_action<F: Fn(&str, &str, &str) + 'static>(&self, f: F) {
        *self.on_action.borrow_mut() = Some(Box::new(f));
    }

    /// Connect ambient dismiss callback (`plugin_id`, `item_id`)
    pub fn connect_dismiss<F: Fn(&str, &str) + 'static>(&self, f: F) {
        *self.on_dismiss.borrow_mut() = Some(Box::new(f));
    }
}

impl Default for AmbientItemsContainer {
    fn default() -> Self {
        Self::new()
    }
}
