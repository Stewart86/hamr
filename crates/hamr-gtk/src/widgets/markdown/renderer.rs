//! Markdown to `TextBuffer` renderer using `pulldown-cmark`.

use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{Align, Orientation, TextBuffer, TextIter, TextTagTable, TextView};
use pulldown_cmark::{
    BlockQuoteKind, CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd,
};
use std::fmt::Write;
use std::sync::OnceLock;

use super::images;
use super::syntax::SyntaxHighlighter;
use super::tags::{self, names};

/// Global syntax highlighter (expensive to create, reuse across renders)
fn highlighter() -> &'static SyntaxHighlighter {
    static HIGHLIGHTER: OnceLock<SyntaxHighlighter> = OnceLock::new();
    HIGHLIGHTER.get_or_init(SyntaxHighlighter::new)
}

/// Render markdown content into a `TextBuffer`
pub fn render_markdown(content: &str, buffer: &TextBuffer, text_view: &TextView) {
    buffer.set_text("");

    // Combine options for full GFM support
    let options = Options::ENABLE_GFM
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TABLES
        | Options::ENABLE_TASKLISTS;

    let parser = Parser::new_ext(content, options);
    let tag_table = buffer.tag_table();

    let mut renderer = MarkdownRenderer::new(buffer, &tag_table, text_view);
    renderer.render(parser);
}

/// Alert type for GFM alerts
#[derive(Debug, Clone, Copy)]
enum AlertKind {
    Note,
    Tip,
    Important,
    Warning,
    Caution,
}

impl AlertKind {
    fn icon(self) -> &'static str {
        match self {
            AlertKind::Note => "info",
            AlertKind::Tip => "lightbulb",
            AlertKind::Important => "error",
            AlertKind::Warning => "warning",
            AlertKind::Caution => "dangerous",
        }
    }

    fn title(self) -> &'static str {
        match self {
            AlertKind::Note => "Note",
            AlertKind::Tip => "Tip",
            AlertKind::Important => "Important",
            AlertKind::Warning => "Warning",
            AlertKind::Caution => "Caution",
        }
    }

    fn css_class(self) -> &'static str {
        match self {
            AlertKind::Note => "markdown-alert-note",
            AlertKind::Tip => "markdown-alert-tip",
            AlertKind::Important => "markdown-alert-important",
            AlertKind::Warning => "markdown-alert-warning",
            AlertKind::Caution => "markdown-alert-caution",
        }
    }
}

#[allow(clippy::struct_excessive_bools)] // Parser state machine flags track independent parsing contexts
struct MarkdownRenderer<'a> {
    buffer: &'a TextBuffer,
    tag_table: &'a TextTagTable,
    text_view: &'a TextView,
    /// Stack of active tag names (owned strings for dynamic link tags)
    tag_stack: Vec<String>,
    /// Current list nesting depth
    list_depth: usize,
    /// Whether we're in an ordered list (with indices)
    ordered_list_indices: Vec<Option<u64>>,
    /// Whether we're inside a code block (collecting content)
    in_code_block: bool,
    /// Code block content being collected
    code_block_content: String,
    /// Code block language hint
    code_block_lang: Option<String>,
    /// Counter for unique link tag names
    link_counter: usize,
    /// Track if we have bold active (for bold+italic combo)
    has_bold: bool,
    /// Track if we have italic active (for bold+italic combo)
    has_italic: bool,
    /// Table state: collecting rows
    in_table: bool,
    in_table_head: bool,
    in_table_row: bool,
    table_rows: Vec<Vec<String>>,
    current_row: Vec<String>,
    current_cell: String,
    /// Whether current item is a task list item (has checkbox)
    pending_task_marker: bool,
    /// Blockquote state: collecting content
    in_blockquote: bool,
    blockquote_content: String,
    /// Alert state
    alert_kind: Option<AlertKind>,
}

impl<'a> MarkdownRenderer<'a> {
    fn new(buffer: &'a TextBuffer, tag_table: &'a TextTagTable, text_view: &'a TextView) -> Self {
        Self {
            buffer,
            tag_table,
            text_view,
            tag_stack: Vec::new(),
            list_depth: 0,
            ordered_list_indices: Vec::new(),
            in_code_block: false,
            code_block_content: String::new(),
            code_block_lang: None,
            link_counter: 0,
            has_bold: false,
            has_italic: false,
            in_table: false,
            in_table_head: false,
            in_table_row: false,
            table_rows: Vec::new(),
            current_row: Vec::new(),
            current_cell: String::new(),
            pending_task_marker: false,
            in_blockquote: false,
            blockquote_content: String::new(),
            alert_kind: None,
        }
    }

    fn render(&mut self, parser: Parser) {
        for event in parser {
            self.handle_event(event);
        }
    }

    fn handle_event(&mut self, event: Event) {
        match event {
            Event::Start(tag) => self.start_tag(tag),
            Event::End(tag) => self.end_tag(tag),
            Event::Text(text) => self.append_text(&text),
            Event::Code(code) => self.append_inline_code(&code),
            Event::SoftBreak => self.append_text(" "),
            Event::HardBreak => self.append_text("\n"),
            Event::Rule => self.append_rule(),
            Event::TaskListMarker(checked) => self.append_task_marker(checked),
            Event::Html(html) | Event::InlineHtml(html) => self.handle_html(&html),
            _ => {}
        }
    }

    fn handle_html(&mut self, html: &str) {
        let html_lower = html.trim().to_lowercase();

        match html_lower.as_str() {
            "<sub>" => self.tag_stack.push(names::SUBSCRIPT.to_string()),
            "<sup>" => self.tag_stack.push(names::SUPERSCRIPT.to_string()),
            "<ins>" | "<u>" => self.tag_stack.push(names::INSERTED.to_string()),
            "</sub>" | "</sup>" | "</ins>" | "</u>" => self.pop_tag(),
            "<br>" | "<br/>" | "<br />" => self.append_text("\n"),
            _ => {}
        }
    }

    fn handle_image(&mut self, dest_url: &str) {
        self.ensure_newline();

        if images::is_http_url(dest_url) {
            let offset = self.buffer.end_iter().offset();
            images::load_http_image_async(dest_url.to_string(), self.buffer.clone(), offset);
        } else if let Some(texture) = images::load_local_image(dest_url) {
            let mut iter = self.buffer.end_iter();
            self.buffer.insert_paintable(&mut iter, &texture);
        } else {
            self.append_text("[Image]");
        }

        self.ensure_newline();
    }

    fn start_tag(&mut self, tag: Tag) {
        match tag {
            Tag::Heading { level, .. } => {
                self.ensure_newline();
                let tag_name = tags::heading_tag_name(heading_level_to_u8(level));
                self.tag_stack.push(tag_name.to_string());
            }
            Tag::Paragraph => {
                self.ensure_blank_line();
            }
            Tag::Strong => {
                self.has_bold = true;
                self.update_bold_italic_tag();
            }
            Tag::Emphasis => {
                self.has_italic = true;
                self.update_bold_italic_tag();
            }
            Tag::Strikethrough => {
                self.tag_stack.push(names::STRIKETHROUGH.to_string());
            }
            Tag::CodeBlock(kind) => {
                self.ensure_newline();
                self.in_code_block = true;
                self.code_block_content.clear();
                self.code_block_lang = match kind {
                    CodeBlockKind::Fenced(lang) if !lang.is_empty() => Some(lang.to_string()),
                    _ => None,
                };
            }
            Tag::BlockQuote(kind) => {
                self.ensure_newline();
                self.in_blockquote = true;
                self.blockquote_content.clear();
                self.alert_kind = match kind {
                    Some(BlockQuoteKind::Note) => Some(AlertKind::Note),
                    Some(BlockQuoteKind::Tip) => Some(AlertKind::Tip),
                    Some(BlockQuoteKind::Important) => Some(AlertKind::Important),
                    Some(BlockQuoteKind::Warning) => Some(AlertKind::Warning),
                    Some(BlockQuoteKind::Caution) => Some(AlertKind::Caution),
                    None => None,
                };
            }
            Tag::List(first_item) => {
                self.list_depth += 1;
                self.ordered_list_indices.push(first_item);
            }
            Tag::Item => {
                self.ensure_newline();
                // Don't append marker yet - wait to see if there's a task marker
                self.pending_task_marker = true;
            }
            Tag::Link { dest_url, .. } => {
                let tag_name =
                    tags::create_link_tag_with_url(self.tag_table, &dest_url, self.link_counter);
                self.link_counter += 1;
                self.tag_stack.push(tag_name);
            }
            Tag::Image { dest_url, .. } => {
                self.handle_image(&dest_url);
            }
            Tag::Table(_) => {
                self.ensure_newline();
                self.in_table = true;
                self.table_rows.clear();
            }
            Tag::TableHead => {
                self.in_table_head = true;
                self.in_table_row = true; // Header cells come directly, treat as a row
                self.current_row.clear();
            }
            Tag::TableRow => {
                self.in_table_row = true;
                self.current_row.clear();
            }
            Tag::TableCell => {
                self.current_cell.clear();
            }
            _ => {}
        }
    }

    fn end_tag(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Heading(_) => {
                self.pop_tag();
                self.append_text("\n");
            }
            TagEnd::Paragraph => {
                self.append_text("\n");
            }
            TagEnd::Strong => {
                self.pop_tag();
                self.has_bold = false;
                self.update_bold_italic_tag();
            }
            TagEnd::Emphasis => {
                self.pop_tag();
                self.has_italic = false;
                self.update_bold_italic_tag();
            }
            TagEnd::Strikethrough | TagEnd::Link => {
                self.pop_tag();
            }
            TagEnd::CodeBlock => {
                self.in_code_block = false;
                let content = std::mem::take(&mut self.code_block_content);
                let lang = self.code_block_lang.take();
                self.render_code_block(&content, lang.as_deref());
            }
            TagEnd::BlockQuote(_) => {
                self.in_blockquote = false;
                let content = std::mem::take(&mut self.blockquote_content);
                let alert = self.alert_kind.take();
                self.render_blockquote(&content, alert);
            }
            TagEnd::List(_) => {
                self.list_depth = self.list_depth.saturating_sub(1);
                self.ordered_list_indices.pop();
            }
            TagEnd::Item => {
                // Increment ordered list counter if applicable
                if let Some(Some(idx)) = self.ordered_list_indices.last_mut() {
                    *idx += 1;
                }
            }
            TagEnd::Table => {
                self.render_table();
                self.in_table = false;
                self.table_rows.clear();
            }
            TagEnd::TableHead => {
                // TableHead contains cells directly (no TableRow wrapper)
                // So we need to push the header row here
                self.in_table_row = false;
                if !self.current_row.is_empty() {
                    self.table_rows.push(std::mem::take(&mut self.current_row));
                }
                self.in_table_head = false;
            }
            TagEnd::TableRow => {
                self.in_table_row = false;
                if !self.current_row.is_empty() {
                    self.table_rows.push(std::mem::take(&mut self.current_row));
                }
            }
            TagEnd::TableCell => {
                self.current_row
                    .push(std::mem::take(&mut self.current_cell));
            }
            _ => {}
        }
    }

    fn append_text(&mut self, text: &str) {
        // If collecting table cells, store instead of rendering
        if self.in_table && self.in_table_row {
            self.current_cell.push_str(text);
            return;
        }

        // If collecting code block content
        if self.in_code_block {
            self.code_block_content.push_str(text);
            return;
        }

        // If collecting blockquote content
        if self.in_blockquote {
            self.blockquote_content.push_str(text);
            return;
        }

        // If we have a pending list marker (non-task item), render it now
        if self.pending_task_marker {
            self.pending_task_marker = false;
            self.append_list_marker();
        }

        let mut end_iter = self.buffer.end_iter();

        if self.tag_stack.is_empty() {
            self.buffer.insert(&mut end_iter, text);
        } else {
            let tags: Vec<&str> = self
                .tag_stack
                .iter()
                .map(std::string::String::as_str)
                .collect();
            self.insert_with_tags(&mut end_iter, text, &tags);
        }
    }

    fn append_inline_code(&mut self, code: &str) {
        let mut end_iter = self.buffer.end_iter();
        self.insert_with_tags(&mut end_iter, code, &[names::CODE_INLINE]);
    }

    fn append_rule(&mut self) {
        self.ensure_newline();

        let separator = gtk4::Separator::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .margin_top(12)
            .margin_bottom(12)
            .css_classes(["markdown-hr"])
            .build();

        let mut end_iter = self.buffer.end_iter();
        let anchor = self.buffer.create_child_anchor(&mut end_iter);
        self.text_view.add_child_at_anchor(&separator, &anchor);

        // Update width after widget is mapped
        let sep_weak = separator.downgrade();
        let tv_weak = self.text_view.downgrade();
        glib::idle_add_local_once(move || {
            if let (Some(sep), Some(tv)) = (sep_weak.upgrade(), tv_weak.upgrade()) {
                let margins = tv.left_margin() + tv.right_margin();
                let available = tv.width() - margins;
                sep.set_width_request(available.max(200));
            }
        });

        self.buffer.insert(&mut self.buffer.end_iter(), "\n");
    }

    fn append_task_marker(&mut self, checked: bool) {
        // Task marker replaces the bullet, so clear pending
        self.pending_task_marker = false;

        // Add indent for nested lists
        let indent = "  ".repeat(self.list_depth.saturating_sub(1));
        if !indent.is_empty() {
            self.buffer.insert(&mut self.buffer.end_iter(), &indent);
        }

        let (icon, tag_name) = if checked {
            ("check_box", names::TASK_CHECKED)
        } else {
            ("check_box_outline_blank", names::TASK_UNCHECKED)
        };

        let mut end_iter = self.buffer.end_iter();
        self.insert_with_tags(&mut end_iter, icon, &[tag_name]);
        self.buffer.insert(&mut end_iter, " ");
    }

    fn append_list_marker(&mut self) {
        let indent = "  ".repeat(self.list_depth.saturating_sub(1));

        let marker = if let Some(Some(idx)) = self.ordered_list_indices.last() {
            format!("{indent}{idx}. ")
        } else {
            format!("{indent}\u{2022} ") // bullet point
        };

        self.append_text(&marker);
    }

    fn insert_with_tags(&self, iter: &mut TextIter, text: &str, tag_names: &[&str]) {
        let start_offset = iter.offset();
        self.buffer.insert(iter, text);

        let start_iter = self.buffer.iter_at_offset(start_offset);
        let end_iter = self.buffer.end_iter();

        for tag_name in tag_names {
            if let Some(tag) = self.tag_table.lookup(tag_name) {
                self.buffer.apply_tag(&tag, &start_iter, &end_iter);
            }
        }
    }

    fn pop_tag(&mut self) {
        self.tag_stack.pop();
    }

    fn update_bold_italic_tag(&mut self) {
        // Remove any existing bold/italic tags from stack
        self.tag_stack
            .retain(|t| t != names::BOLD && t != names::ITALIC && t != names::BOLD_ITALIC);

        // Add appropriate tag based on current state
        if self.has_bold && self.has_italic {
            self.tag_stack.push(names::BOLD_ITALIC.to_string());
        } else if self.has_bold {
            self.tag_stack.push(names::BOLD.to_string());
        } else if self.has_italic {
            self.tag_stack.push(names::ITALIC.to_string());
        }
    }

    // Grid row/col indices are usize, GTK attach requires i32
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    fn render_table(&mut self) {
        if self.table_rows.is_empty() {
            return;
        }

        let num_cols = self
            .table_rows
            .iter()
            .map(std::vec::Vec::len)
            .max()
            .unwrap_or(0);
        if num_cols == 0 {
            return;
        }

        // Create a Grid widget for the table
        let grid = gtk4::Grid::builder()
            .column_spacing(0)
            .row_spacing(0)
            .column_homogeneous(true)
            .margin_top(12)
            .margin_bottom(12)
            .hexpand(true)
            .css_classes(["markdown-table"])
            .build();

        for (row_idx, row) in self.table_rows.iter().enumerate() {
            for (col_idx, cell) in row.iter().enumerate() {
                let is_header = row_idx == 0;
                let label = gtk4::Label::builder()
                    .label(cell.trim())
                    .halign(Align::Start)
                    .valign(Align::Center)
                    .hexpand(true)
                    .margin_start(12)
                    .margin_end(12)
                    .margin_top(6)
                    .margin_bottom(6)
                    .build();

                if is_header {
                    label.add_css_class("markdown-table-header");
                } else {
                    label.add_css_class("markdown-table-cell");
                }

                grid.attach(&label, col_idx as i32, row_idx as i32, 1, 1);
            }
        }

        // Insert anchor in buffer
        let mut end_iter = self.buffer.end_iter();
        let anchor = self.buffer.create_child_anchor(&mut end_iter);

        // Add widget to text view and make it visible
        self.text_view.add_child_at_anchor(&grid, &anchor);
        grid.set_visible(true);

        // Set width after widget is mapped
        let grid_weak = grid.downgrade();
        let tv_weak = self.text_view.downgrade();
        glib::idle_add_local_once(move || {
            if let (Some(grid), Some(tv)) = (grid_weak.upgrade(), tv_weak.upgrade()) {
                let margins = tv.left_margin() + tv.right_margin() + 4;
                let available = tv.width() - margins;
                if available > 100 {
                    grid.set_width_request(available);
                }
            }
        });

        self.buffer.insert(&mut self.buffer.end_iter(), "\n");
    }

    fn render_code_block(&mut self, content: &str, lang: Option<&str>) {
        let content = content.trim_end();
        if content.is_empty() {
            return;
        }

        // Main container with rounded corners
        let container = gtk4::Box::builder()
            .orientation(Orientation::Vertical)
            .css_classes(["markdown-code-block"])
            .margin_top(8)
            .margin_bottom(8)
            .hexpand(false)
            .build();

        // Header with language label and copy button
        let header = gtk4::Box::builder()
            .orientation(Orientation::Horizontal)
            .css_classes(["markdown-code-header"])
            .build();

        // Language label (left side)
        let lang_label = gtk4::Label::builder()
            .label(lang.unwrap_or(""))
            .css_classes(["markdown-code-lang"])
            .halign(Align::Start)
            .hexpand(true)
            .build();
        header.append(&lang_label);

        // Copy button (right side)
        let copy_btn = gtk4::Button::builder()
            .css_classes(["markdown-code-copy"])
            .build();
        let copy_icon = gtk4::Label::builder()
            .label("content_copy")
            .css_classes(["material-icon"])
            .build();
        copy_btn.set_child(Some(&copy_icon));

        // Copy to clipboard on click
        let content_clone = content.to_string();
        let icon_weak = copy_icon.downgrade();
        copy_btn.connect_clicked(move |btn| {
            let clipboard = btn.clipboard();
            clipboard.set_text(&content_clone);

            // Show feedback
            if let Some(icon) = icon_weak.upgrade() {
                icon.set_text("check");
                let icon_weak2 = icon.downgrade();
                glib::timeout_add_local_once(std::time::Duration::from_millis(1500), move || {
                    if let Some(icon) = icon_weak2.upgrade() {
                        icon.set_text("content_copy");
                    }
                });
            }
        });
        header.append(&copy_btn);

        container.append(&header);

        // Code content with syntax highlighting using Pango markup
        let code_box = gtk4::Box::builder()
            .orientation(Orientation::Vertical)
            .css_classes(["markdown-code-content"])
            .build();

        // Apply syntax highlighting
        let highlighted = highlighter().highlight(content, lang);
        for line_spans in highlighted {
            // Build Pango markup for the entire line
            let mut markup = String::new();
            for span in &line_spans {
                let escaped = glib::markup_escape_text(&span.text);
                let _ = write!(markup, "<span foreground=\"{}\">", span.color);
                markup.push_str(&escaped);
                markup.push_str("</span>");
            }

            let label = gtk4::Label::builder()
                .use_markup(true)
                .label(&markup)
                .css_classes(["markdown-code-line"])
                .halign(Align::Start)
                .selectable(true)
                .build();

            code_box.append(&label);
        }

        // Wrap code content in horizontal ScrolledWindow for overflow
        let code_scroll = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Automatic)
            .vscrollbar_policy(gtk4::PolicyType::Never)
            .propagate_natural_width(false)
            .propagate_natural_height(true)
            .hexpand(false)
            .child(&code_box)
            .build();

        container.append(&code_scroll);

        // Insert widget
        let mut end_iter = self.buffer.end_iter();
        let anchor = self.buffer.create_child_anchor(&mut end_iter);
        self.text_view.add_child_at_anchor(&container, &anchor);
        container.set_visible(true);

        // Set width after widget is mapped
        let container_weak = container.downgrade();
        let tv_weak = self.text_view.downgrade();
        glib::idle_add_local_once(move || {
            if let (Some(container), Some(tv)) = (container_weak.upgrade(), tv_weak.upgrade()) {
                let margins = tv.left_margin() + tv.right_margin() + 4;
                let available = tv.width() - margins;
                if available > 100 {
                    container.set_width_request(available);
                }
            }
        });

        self.buffer.insert(&mut self.buffer.end_iter(), "\n");
    }

    fn render_blockquote(&mut self, content: &str, alert: Option<AlertKind>) {
        let content = content.trim();
        if content.is_empty() {
            return;
        }

        let css_class = alert.map_or("markdown-blockquote", |a| a.css_class());

        // Container with left border
        let container = gtk4::Box::builder()
            .orientation(Orientation::Horizontal)
            .css_classes([css_class])
            .margin_top(8)
            .margin_bottom(8)
            .hexpand(true)
            .build();

        // Content area
        let content_box = gtk4::Box::builder()
            .orientation(Orientation::Vertical)
            .hexpand(true)
            .spacing(4)
            .build();

        // For alerts, add icon and title
        if let Some(alert) = alert {
            let title_box = gtk4::Box::builder()
                .orientation(Orientation::Horizontal)
                .spacing(6)
                .build();

            let icon = gtk4::Label::builder()
                .label(alert.icon())
                .css_classes(["material-icon", "markdown-alert-icon"])
                .build();
            title_box.append(&icon);

            let title = gtk4::Label::builder()
                .label(alert.title())
                .css_classes(["markdown-alert-title"])
                .halign(Align::Start)
                .build();
            title_box.append(&title);

            content_box.append(&title_box);
        }

        // Quote content - strip alert title if present
        let display_content = if alert.is_some() {
            // Remove the [!TYPE] line from content
            content
                .lines()
                .skip_while(|line| {
                    let trimmed = line.trim();
                    trimmed.starts_with("[!") && trimmed.ends_with(']')
                })
                .collect::<Vec<_>>()
                .join("\n")
                .trim()
                .to_string()
        } else {
            content.to_string()
        };

        let content_label = gtk4::Label::builder()
            .label(&display_content)
            .css_classes(["markdown-blockquote-content"])
            .halign(Align::Start)
            .valign(Align::Start)
            .wrap(true)
            .wrap_mode(gtk4::pango::WrapMode::Word)
            .selectable(true)
            .build();
        content_box.append(&content_label);

        container.append(&content_box);

        // Insert widget
        let mut end_iter = self.buffer.end_iter();
        let anchor = self.buffer.create_child_anchor(&mut end_iter);
        self.text_view.add_child_at_anchor(&container, &anchor);
        container.set_visible(true);

        // Set width after widget is mapped
        let container_weak = container.downgrade();
        let tv_weak = self.text_view.downgrade();
        glib::idle_add_local_once(move || {
            if let (Some(container), Some(tv)) = (container_weak.upgrade(), tv_weak.upgrade()) {
                let margins = tv.left_margin() + tv.right_margin() + 4;
                let available = tv.width() - margins;
                if available > 100 {
                    container.set_width_request(available);
                }
            }
        });

        self.buffer.insert(&mut self.buffer.end_iter(), "\n");
    }

    fn ensure_newline(&mut self) {
        let end_iter = self.buffer.end_iter();
        if end_iter.offset() > 0 {
            let mut check_iter = end_iter;
            check_iter.backward_char();
            if check_iter.char() != '\n' {
                self.buffer.insert(&mut self.buffer.end_iter(), "\n");
            }
        }
    }

    fn ensure_blank_line(&mut self) {
        let end_iter = self.buffer.end_iter();
        let offset = end_iter.offset();

        if offset == 0 {
            return;
        }

        // Check last two characters for double newline
        let text = self
            .buffer
            .text(&self.buffer.start_iter(), &end_iter, false);
        let text_str = text.as_str();

        if !text_str.ends_with("\n\n") {
            if text_str.ends_with('\n') {
                self.buffer.insert(&mut self.buffer.end_iter(), "\n");
            } else {
                self.buffer.insert(&mut self.buffer.end_iter(), "\n\n");
            }
        }
    }
}

fn heading_level_to_u8(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}
