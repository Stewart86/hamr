//! Reusable GTK4 widgets matching the QML design spec from the original
//! Hamr implementation.

pub mod action_bar;
pub mod ambient_container;
pub mod ambient_item;
pub mod badge;
pub mod chip;
pub mod confirm_dialog;
pub mod design;
pub mod error_dialog;
pub mod form_view;
pub mod gauge;
pub mod graph;
pub mod grid_item;
pub mod kbd;
pub mod keybinding_map;
pub mod markdown;
pub mod pinned_panel;
pub mod preview_panel;
pub mod result_card;
pub mod result_grid;
pub mod result_item;
pub mod result_list;
pub mod result_object;
pub mod result_view;
pub mod result_visual;
pub mod ripple_button;

pub use action_bar::{ActionBar, ActionBarAction, ActionBarMode};
pub use ambient_container::{AmbientItemWithPlugin, AmbientItemsContainer};
pub use ambient_item::AmbientItemWidget;
pub use badge::BadgeWidget;
pub use chip::ChipWidget;
pub use confirm_dialog::ConfirmDialog;
pub use error_dialog::ErrorDialog;
pub use keybinding_map::KeybindingMap;
#[allow(unused_imports)]
pub use markdown::MarkdownView;
pub use preview_panel::PreviewPanel;
pub use result_card::ResultCard;
pub use result_item::ResultItem;
pub use result_view::ResultView;
