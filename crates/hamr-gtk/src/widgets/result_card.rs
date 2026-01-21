//! Result card widget for displaying plugin card responses.
//!
//! This widget renders `CardData` from plugins, including:
//! - Title
//! - Markdown content
//! - Card blocks (pills, messages, notes)
//! - Action buttons

use gtk4::prelude::*;
use gtk4::{Label, Orientation};
use hamr_types::{Action, CardBlock, CardData};
use std::cell::RefCell;
use std::rc::Rc;

use super::design;
use super::markdown::MarkdownView;
use super::ripple_button::RippleButton;

/// Callback type for action button clicks
/// Callback type for action button clicks: (`context`, `action_id`)
type ActionCallback = Rc<RefCell<Option<Box<dyn Fn(&str, &str)>>>>;

/// A widget that displays plugin card responses.
pub struct ResultCard {
    container: gtk4::Box,
    title_label: Label,
    markdown_view: MarkdownView,
    blocks_container: gtk4::Box,
    actions_container: gtk4::Box,
    on_action: ActionCallback,
    /// Current context (item ID) for action handling
    context: Rc<RefCell<String>>,
}

impl ResultCard {
    /// Create a new `ResultCard` widget.
    pub fn new() -> Self {
        let container = gtk4::Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(8)
            .vexpand(true)
            .css_classes(["result-card"])
            .build();

        // Title
        let title_label = Label::builder()
            .css_classes(["result-card-title"])
            .halign(gtk4::Align::Start)
            .wrap(true)
            .build();
        container.append(&title_label);

        // Blocks container (for Pill, Separator, Message, Note blocks)
        let blocks_container = gtk4::Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(4)
            .build();
        container.append(&blocks_container);

        // Markdown view (wrapped in scrolled window)
        let markdown_view = MarkdownView::new();
        container.append(markdown_view.widget());

        // Actions container
        let actions_container = gtk4::Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(8)
            .halign(gtk4::Align::End)
            .margin_top(8)
            .margin_end(8)
            .margin_bottom(8)
            .build();
        container.append(&actions_container);

        Self {
            container,
            title_label,
            markdown_view,
            blocks_container,
            actions_container,
            on_action: Rc::new(RefCell::new(None)),
            context: Rc::new(RefCell::new(String::new())),
        }
    }

    /// Set the card data and context to display.
    // Card max_height is usize from config, GTK uses i32 for height
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    pub fn set_card_with_context(&self, card: &CardData, context: Option<&str>) {
        // Store context for action handling
        *self.context.borrow_mut() = context.unwrap_or_default().to_string();

        // Title
        self.title_label.set_text(&card.title);
        self.title_label.set_visible(!card.title.is_empty());

        // Clear previous blocks
        while let Some(child) = self.blocks_container.first_child() {
            self.blocks_container.remove(&child);
        }

        // Render blocks if present
        if card.blocks.is_empty() {
            self.blocks_container.set_visible(false);

            // Markdown content (prefer markdown over plain content)
            if let Some(md) = &card.markdown {
                tracing::debug!("Setting markdown content: {} chars", md.len());
                self.markdown_view.set_content(md);
                self.markdown_view.widget().set_visible(true);
            } else if let Some(content) = &card.content {
                // Treat plain content as markdown too
                tracing::debug!("Setting plain content: {} chars", content.len());
                self.markdown_view.set_content(content);
                self.markdown_view.widget().set_visible(true);
            } else {
                tracing::debug!("No content to display");
                self.markdown_view.clear();
                self.markdown_view.widget().set_visible(false);
            }
        } else {
            for block in &card.blocks {
                let block_widget = Self::render_block(block);
                self.blocks_container.append(&block_widget);
            }
            self.blocks_container.set_visible(true);
            // Hide markdown view when using blocks
            self.markdown_view.widget().set_visible(false);
        }

        // Set max height if specified
        if let Some(max_height) = card.max_height {
            self.markdown_view.set_max_content_height(max_height as i32);
        }

        // Render actions
        self.render_actions(&card.actions);
    }

    /// Connect a callback for action button clicks.
    /// Callback receives (`context`, `action_id`).
    pub fn connect_action<F>(&self, callback: F)
    where
        F: Fn(&str, &str) + 'static,
    {
        *self.on_action.borrow_mut() = Some(Box::new(callback));
    }

    /// Get the root widget for embedding.
    pub fn widget(&self) -> &gtk4::Box {
        &self.container
    }

    /// Set the maximum height for the card's scrollable content area.
    pub fn set_max_height(&self, height: i32) {
        tracing::debug!("ResultCard::set_max_height({})", height);
        self.markdown_view.set_max_content_height(height);
    }

    /// Clear the card content.
    pub fn clear(&self) {
        self.title_label.set_text("");
        self.markdown_view.clear();

        while let Some(child) = self.blocks_container.first_child() {
            self.blocks_container.remove(&child);
        }

        while let Some(child) = self.actions_container.first_child() {
            self.actions_container.remove(&child);
        }
    }

    fn render_block(block: &CardBlock) -> gtk4::Widget {
        match block {
            CardBlock::Pill { text } => {
                let label = Label::builder()
                    .label(text)
                    .css_classes(["card-pill"])
                    .halign(gtk4::Align::Center)
                    .build();
                label.upcast()
            }
            CardBlock::Separator => {
                let separator = gtk4::Separator::builder()
                    .orientation(Orientation::Horizontal)
                    .css_classes(["card-separator"])
                    .build();
                separator.upcast()
            }
            CardBlock::Message { role, content } => {
                let msg_box = gtk4::Box::builder()
                    .orientation(Orientation::Vertical)
                    .spacing(4)
                    .css_classes(["card-message", &format!("message-{role}")])
                    .build();

                // Role label (optional, for assistant/user distinction)
                let role_label = Label::builder()
                    .label(role)
                    .css_classes(["message-role"])
                    .halign(gtk4::Align::Start)
                    .build();
                msg_box.append(&role_label);

                // Message content as markdown
                let content_view = MarkdownView::new();
                content_view.set_content(content);
                // Don't use scrolled window for inline messages
                content_view
                    .widget()
                    .set_vscrollbar_policy(gtk4::PolicyType::Never);
                msg_box.append(content_view.widget());

                msg_box.upcast()
            }
            CardBlock::Note { content } => {
                let note_box = gtk4::Box::builder()
                    .orientation(Orientation::Vertical)
                    .css_classes(["card-note"])
                    .build();

                let content_view = MarkdownView::new();
                content_view.set_content(content);
                content_view
                    .widget()
                    .set_vscrollbar_policy(gtk4::PolicyType::Never);
                note_box.append(content_view.widget());

                note_box.upcast()
            }
        }
    }

    fn render_actions(&self, actions: &[Action]) {
        // Clear previous actions
        while let Some(child) = self.actions_container.first_child() {
            self.actions_container.remove(&child);
        }

        if actions.is_empty() {
            self.actions_container.set_visible(false);
            return;
        }

        self.actions_container.set_visible(true);

        for action in actions {
            let button = RippleButton::from_action(action, None);

            let on_action = self.on_action.clone();
            let context = self.context.clone();
            button.connect_clicked(move |action_id| {
                let ctx = context.borrow().clone();
                tracing::debug!("Card action clicked: {} (context: {})", action_id, ctx);
                if let Some(ref cb) = *on_action.borrow() {
                    cb(&ctx, action_id);
                }
            });

            self.actions_container.append(button.widget());
        }
    }
}

impl Default for ResultCard {
    fn default() -> Self {
        Self::new()
    }
}

/// CSS styles for result card (uses same styling as `result_list`)
// CSS template - splitting would scatter related style rules
#[allow(clippy::too_many_lines)]
pub fn result_card_css(theme: &crate::config::Theme) -> String {
    use design::{icon, radius, rounding, spacing};

    let colors = &theme.colors;

    // Pre-compute scaled values for cleaner format string
    let margin_top = theme.scaled(spacing::XS); // 4
    let margin_side = theme.scaled(spacing::SM); // 8 (mapped from 6)
    let padding = theme.scaled(design::result_list::CONTAINER_PADDING);
    let border = theme.scaled(1);
    let card_radius = theme.scaled(rounding::SMALL);
    let margin_sm = theme.scaled(spacing::SM); // 8
    let margin_xs = theme.scaled(spacing::XS); // 4
    let margin_xxl = theme.scaled(spacing::XXL); // 24
    let padding_xs = theme.scaled(spacing::XS); // 4
    let padding_sm = theme.scaled(spacing::SM); // 8
    let padding_md = theme.scaled(spacing::MD); // 12
    let padding_lg = theme.scaled(spacing::LG); // 16
    let padding_6 = theme.scaled(spacing::SM - spacing::XXXS); // 6
    let radius_xs = theme.scaled(radius::XS); // 4
    let radius_sm = theme.scaled(radius::SM); // 8
    let radius_md = theme.scaled(radius::MD); // 12
    let border_left = theme.scaled(3);
    let icon_lg = theme.scaled(icon::LG); // 24
    let padding_xxxs = theme.scaled(spacing::XXXS); // 2
    let min_height_1 = theme.scaled(1);

    format!(
        r"
        .result-card {{
            margin-top: {margin_top}px;
            margin-left: {margin_side}px;
            margin-right: {margin_side}px;
            margin-bottom: {margin_side}px;
            padding: {padding}px;
            background: {surface_container_low};
            background-color: {surface_container_low};
            border: {border}px solid {outline_variant};
            border-radius: {card_radius}px;
        }}

        .result-card-title {{
            font-size: {font_title}px;
            font-weight: 600;
            margin-bottom: {margin_sm}px;
            color: {on_surface};
        }}

        .card-pill {{
            background: {surface_container_high};
            border-radius: {radius_md}px;
            padding: {padding_xs}px {padding_md}px;
            font-size: {font_pill}px;
            color: {on_surface_variant};
        }}

        .card-separator {{
            margin: {margin_sm}px 0;
            background-color: {outline_variant};
        }}

        .card-message {{
            padding: {padding_sm}px;
            border-radius: {radius_sm}px;
            margin: {margin_xs}px 0;
        }}

        .message-user {{
            background: alpha({primary}, 0.15);
            margin-left: {margin_xxl}px;
        }}

        .message-assistant {{
            background: {surface_container};
            margin-right: {margin_xxl}px;
        }}

        .message-system {{
            background: alpha({secondary}, 0.15);
            font-style: italic;
        }}

        .message-role {{
            font-size: {font_small}px;
            font-weight: 600;
            color: {on_surface_variant};
            text-transform: uppercase;
        }}

        .card-note {{
            background: alpha({secondary}, 0.1);
            border-left: {border_left}px solid {secondary};
            padding: {padding_sm}px;
            margin: {margin_xs}px 0;
        }}

        .markdown-view {{
            background: transparent;
            background-color: transparent;
        }}

        .markdown-view text {{
            background: transparent;
            background-color: transparent;
            color: {on_surface};
        }}

        .markdown-table {{
            border: {border}px solid {outline_variant};
            border-radius: {card_radius}px;
        }}

        .markdown-table-header {{
            font-weight: 600;
            font-size: 0.9em;
            background: {surface_container};
            border-bottom: {border}px solid {outline_variant};
        }}

        .markdown-table-cell {{
            font-size: 0.9em;
            border-bottom: {border}px solid alpha({outline_variant}, 0.3);
        }}

        .markdown-hr {{
            min-height: {min_height_1}px;
            background: {outline_variant};
            margin-top: {margin_sm}px;
            margin-bottom: {margin_sm}px;
        }}

        .markdown-view text selection {{
            background-color: alpha({primary}, 0.3);
        }}

        /* Code blocks */
        .markdown-code-block {{
            background: {surface_container};
            border: {border}px solid {outline_variant};
            border-radius: {radius_sm}px;
        }}

        .markdown-code-header {{
            background: {surface_container_high};
            padding: {padding_6}px {padding_md}px;
            border-bottom: {border}px solid {outline_variant};
        }}

        .markdown-code-lang {{
            font-size: {font_small}px;
            font-weight: 500;
            color: {on_surface_variant};
            font-family: monospace;
        }}

        .markdown-code-copy {{
            min-width: {icon_lg}px;
            min-height: {icon_lg}px;
            padding: {padding_xxxs}px;
            border-radius: {radius_xs}px;
            background: transparent;
            border: none;
        }}

        .markdown-code-copy:hover {{
            background: alpha({on_surface}, 0.1);
        }}

        .markdown-code-copy .material-icon {{
            font-size: {font_copy_icon}px;
            color: {on_surface_variant};
        }}

        .markdown-code-content {{
            font-family: monospace;
            font-size: {font_code}px;
            padding: {padding_md}px;
            color: {on_surface};
        }}

        /* Blockquotes */
        .markdown-blockquote {{
            background: alpha({on_surface}, 0.03);
            border-left: {border_left}px solid {outline_variant};
            border-radius: 0 {radius_sm}px {radius_sm}px 0;
            padding: {padding_md}px {padding_lg}px;
        }}

        .markdown-blockquote-content {{
            font-style: italic;
            color: {on_surface_variant};
        }}

        /* Alerts */
        .markdown-alert-note {{
            background: alpha(#58a6ff, 0.08);
            border-left: {border_left}px solid #58a6ff;
            border-radius: 0 {radius_sm}px {radius_sm}px 0;
            padding: {padding_md}px {padding_lg}px;
        }}

        .markdown-alert-note .markdown-alert-icon,
        .markdown-alert-note .markdown-alert-title {{
            color: #58a6ff;
        }}

        .markdown-alert-tip {{
            background: alpha(#3fb950, 0.08);
            border-left: {border_left}px solid #3fb950;
            border-radius: 0 {radius_sm}px {radius_sm}px 0;
            padding: {padding_md}px {padding_lg}px;
        }}

        .markdown-alert-tip .markdown-alert-icon,
        .markdown-alert-tip .markdown-alert-title {{
            color: #3fb950;
        }}

        .markdown-alert-important {{
            background: alpha(#a371f7, 0.08);
            border-left: {border_left}px solid #a371f7;
            border-radius: 0 {radius_sm}px {radius_sm}px 0;
            padding: {padding_md}px {padding_lg}px;
        }}

        .markdown-alert-important .markdown-alert-icon,
        .markdown-alert-important .markdown-alert-title {{
            color: #a371f7;
        }}

        .markdown-alert-warning {{
            background: alpha(#d29922, 0.08);
            border-left: {border_left}px solid #d29922;
            border-radius: 0 {radius_sm}px {radius_sm}px 0;
            padding: {padding_md}px {padding_lg}px;
        }}

        .markdown-alert-warning .markdown-alert-icon,
        .markdown-alert-warning .markdown-alert-title {{
            color: #d29922;
        }}

        .markdown-alert-caution {{
            background: alpha(#f85149, 0.08);
            border-left: {border_left}px solid #f85149;
            border-radius: 0 {radius_sm}px {radius_sm}px 0;
            padding: {padding_md}px {padding_lg}px;
        }}

        .markdown-alert-caution .markdown-alert-icon,
        .markdown-alert-caution .markdown-alert-title {{
            color: #f85149;
        }}

        .markdown-alert-icon {{
            font-size: {font_alert_icon}px;
        }}

        .markdown-alert-title {{
            font-weight: 600;
            font-size: {font_code}px;
        }}
        ",
        surface_container_low = colors.surface_container_low,
        surface_container = colors.surface_container,
        surface_container_high = colors.surface_container_high,
        outline_variant = colors.outline_variant,
        on_surface = colors.on_surface,
        on_surface_variant = colors.on_surface_variant,
        primary = colors.primary,
        secondary = colors.secondary,
        font_title = theme.scaled_font(design::font::LG), // 15 (mapped from 16)
        font_pill = theme.scaled_font(design::font::MD - 1), // 12
        font_small = theme.scaled_font(design::font::SM), // 11
        font_code = theme.scaled_font(design::font::MD),  // 13
        font_copy_icon = theme.scaled_font(design::font::LG), // 15 (mapped from 16)
        font_alert_icon = theme.scaled_font(design::font::XL), // 17 (mapped from 18)
    )
}
