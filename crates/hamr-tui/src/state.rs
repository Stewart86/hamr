//! View state types for the TUI.

use crate::compositor::Window as CompositorWindow;
use hamr_rpc::{
    CardData, FormData, FormFieldType, GridBrowserData, GridItem, ImageBrowserData, ImageItem,
};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ErrorState {
    pub title: String,
    pub message: String,
    pub details: Option<String>,
    pub plugin_id: Option<String>,
}

impl ErrorState {
    pub fn new(
        title: String,
        message: String,
        details: Option<String>,
        plugin_id: Option<String>,
    ) -> Self {
        Self {
            title,
            message,
            details,
            plugin_id,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub enum ViewMode {
    #[default]
    Results,
    Form(FormState),
    Card(CardState),
    GridBrowser(GridBrowserState),
    ImageBrowser(ImageBrowserState),
    WindowPicker(WindowPickerState),
    Error(ErrorState),
}

#[derive(Debug, Clone)]
pub struct CardState {
    pub card: CardData,
    pub scroll_offset: usize,
    pub selected_action: usize,
}

impl CardState {
    pub fn new(card: CardData) -> Self {
        Self {
            card,
            scroll_offset: 0,
            selected_action: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GridBrowserState {
    pub data: GridBrowserData,
    pub selected: usize,
    pub columns: usize,
}

impl GridBrowserState {
    pub fn new(data: GridBrowserData) -> Self {
        let columns = data.columns.unwrap_or(4) as usize;
        Self {
            data,
            selected: 0,
            columns,
        }
    }

    pub fn move_left(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_right(&mut self) {
        if self.selected < self.data.items.len().saturating_sub(1) {
            self.selected += 1;
        }
    }

    pub fn move_up(&mut self) {
        if self.selected >= self.columns {
            self.selected -= self.columns;
        }
    }

    pub fn move_down(&mut self) {
        let new_pos = self.selected + self.columns;
        if new_pos < self.data.items.len() {
            self.selected = new_pos;
        }
    }

    pub fn get_selected_item(&self) -> Option<&GridItem> {
        self.data.items.get(self.selected)
    }
}

#[derive(Debug, Clone)]
pub struct ImageBrowserState {
    pub data: ImageBrowserData,
    pub selected: usize,
}

impl ImageBrowserState {
    pub fn new(data: ImageBrowserData) -> Self {
        Self { data, selected: 0 }
    }

    pub fn get_selected_image(&self) -> Option<&ImageItem> {
        self.data.images.get(self.selected)
    }
}

#[derive(Debug, Clone)]
pub struct WindowPickerState {
    pub windows: Vec<CompositorWindow>,
    pub selected: usize,
    pub app_name: String,
}

impl WindowPickerState {
    pub fn new(windows: Vec<CompositorWindow>, app_name: String) -> Self {
        Self {
            windows,
            selected: 0,
            app_name,
        }
    }

    pub fn select_next(&mut self) {
        if self.selected < self.windows.len().saturating_sub(1) {
            self.selected += 1;
        }
    }

    pub fn select_previous(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn get_selected_window(&self) -> Option<&CompositorWindow> {
        self.windows.get(self.selected)
    }
}

#[derive(Debug, Clone)]
pub struct FormState {
    pub form: FormData,
    pub field_values: HashMap<String, String>,
    pub focused_field: usize,
    pub cursor_position: usize,
    pub context: Option<String>,
    pub textarea_scroll: usize,
    original_values: HashMap<String, String>,
    pub show_cancel_confirm: bool,
}

impl FormState {
    pub fn new(form: FormData, context: Option<String>) -> Self {
        let mut field_values = HashMap::new();
        for field in &form.fields {
            let default_val = field.default_value.clone().unwrap_or_default();
            field_values.insert(field.id.clone(), default_val.clone());
        }
        let cursor_pos = form
            .fields
            .first()
            .and_then(|f| f.default_value.as_ref())
            .map_or(0, std::string::String::len);
        let original_values = field_values.clone();
        Self {
            form,
            field_values,
            focused_field: 0,
            cursor_position: cursor_pos,
            context,
            textarea_scroll: 0,
            original_values,
            show_cancel_confirm: false,
        }
    }

    pub fn is_dirty(&self) -> bool {
        for (id, current_value) in &self.field_values {
            let original = self
                .original_values
                .get(id)
                .map_or("", std::string::String::as_str);
            if current_value != original {
                return true;
            }
        }
        false
    }

    pub fn current_field(&self) -> Option<&hamr_rpc::FormField> {
        self.form.fields.get(self.focused_field)
    }

    pub fn current_value(&self) -> String {
        self.current_field()
            .map(|f| self.field_values.get(&f.id).cloned().unwrap_or_default())
            .unwrap_or_default()
    }

    pub fn set_current_value(&mut self, value: String) {
        if let Some(field) = self.current_field() {
            self.field_values.insert(field.id.clone(), value);
        }
    }

    pub fn focus_next(&mut self) {
        if self.focused_field < self.form.fields.len() + 1 {
            self.focused_field += 1;
            self.cursor_position = self.current_value().len();
            self.textarea_scroll = 0;
        }
    }

    pub fn focus_prev(&mut self) {
        if self.focused_field > 0 {
            self.focused_field -= 1;
            self.cursor_position = self.current_value().len();
            self.textarea_scroll = 0;
        }
    }

    pub fn is_on_submit(&self) -> bool {
        self.focused_field == self.form.fields.len()
    }

    pub fn is_on_cancel(&self) -> bool {
        self.focused_field == self.form.fields.len() + 1
    }

    pub fn get_form_data(&self) -> HashMap<String, String> {
        self.field_values.clone()
    }

    pub fn insert_char(&mut self, c: char) {
        if let Some(field) = self.current_field() {
            if !matches!(
                field.field_type,
                FormFieldType::Text
                    | FormFieldType::Password
                    | FormFieldType::Number
                    | FormFieldType::TextArea
                    | FormFieldType::Date
                    | FormFieldType::Time
                    | FormFieldType::Email
                    | FormFieldType::Url
                    | FormFieldType::Phone
            ) {
                return;
            }

            if matches!(field.field_type, FormFieldType::Number)
                && !c.is_ascii_digit()
                && c != '.'
                && c != '-'
            {
                return;
            }
            if matches!(field.field_type, FormFieldType::Date) && !c.is_ascii_digit() && c != '-' {
                return;
            }
            if matches!(field.field_type, FormFieldType::Time) && !c.is_ascii_digit() && c != ':' {
                return;
            }

            let mut value = self.current_value();
            value.insert(self.cursor_position, c);
            self.set_current_value(value);
            self.cursor_position += 1;
        }
    }

    pub fn delete_char(&mut self) {
        if self.cursor_position > 0 {
            let mut value = self.current_value();
            if self.cursor_position <= value.len() {
                value.remove(self.cursor_position - 1);
                self.set_current_value(value);
                self.cursor_position -= 1;
            }
        }
    }

    pub fn toggle_bool_field(&mut self) {
        if let Some(field) = self.current_field()
            && matches!(
                field.field_type,
                FormFieldType::Checkbox | FormFieldType::Switch
            )
        {
            let current = self.current_value();
            let new_val = if current == "true" || current == "1" || current == "on" {
                "false"
            } else {
                "true"
            };
            self.set_current_value(new_val.to_string());
        }
    }

    pub fn cycle_select_prev(&mut self) {
        if let Some(field) = self.current_field()
            && matches!(field.field_type, FormFieldType::Select)
        {
            let value = self.current_value();
            let options = &field.options;
            if !options.is_empty() {
                let current_idx = options.iter().position(|o| o.value == value).unwrap_or(0);
                let new_idx = if current_idx == 0 {
                    options.len() - 1
                } else {
                    current_idx - 1
                };
                self.set_current_value(options[new_idx].value.clone());
            }
        }
    }

    pub fn cycle_select_next(&mut self) {
        if let Some(field) = self.current_field()
            && matches!(field.field_type, FormFieldType::Select)
        {
            let value = self.current_value();
            let options = &field.options;
            if !options.is_empty() {
                let current_idx = options.iter().position(|o| o.value == value).unwrap_or(0);
                let new_idx = (current_idx + 1) % options.len();
                self.set_current_value(options[new_idx].value.clone());
            }
        }
    }

    pub fn adjust_slider(&mut self, increase: bool) {
        if let Some(field) = self.current_field()
            && matches!(field.field_type, FormFieldType::Slider)
        {
            let current: f64 = self.current_value().parse().unwrap_or(0.0);
            let min = field.min.unwrap_or(0.0);
            let max = field.max.unwrap_or(100.0);
            let step = field.step.unwrap_or(1.0);
            let new_val = if increase {
                (current + step).min(max)
            } else {
                (current - step).max(min)
            };
            self.set_current_value(format!("{new_val:.1}"));
        }
    }

    pub fn get_missing_required(&self) -> Vec<&str> {
        self.form
            .fields
            .iter()
            .filter(|f| f.required)
            .filter(|f| {
                let val = self.field_values.get(&f.id).cloned().unwrap_or_default();
                val.trim().is_empty()
            })
            .map(|f| f.label.as_str())
            .collect()
    }

    pub fn get_validation_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();

        for field in &self.form.fields {
            let val = self
                .field_values
                .get(&field.id)
                .cloned()
                .unwrap_or_default();

            if val.trim().is_empty() {
                continue;
            }

            match field.field_type {
                FormFieldType::Email => {
                    if !is_valid_email(&val) {
                        errors.push(format!("{}: invalid email format", field.label));
                    }
                }
                FormFieldType::Url => {
                    if !is_valid_url(&val) {
                        errors.push(format!("{}: invalid URL format", field.label));
                    }
                }
                FormFieldType::Phone => {
                    if !is_valid_phone(&val) {
                        errors.push(format!("{}: invalid phone format", field.label));
                    }
                }
                _ => {}
            }
        }

        errors
    }
}

pub fn is_valid_email(email: &str) -> bool {
    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 {
        return false;
    }
    let local = parts[0];
    let domain = parts[1];

    !local.is_empty() && !domain.is_empty() && domain.contains('.')
}

pub fn is_valid_url(url: &str) -> bool {
    let url_lower = url.to_lowercase();
    (url_lower.starts_with("http://") || url_lower.starts_with("https://")) && url.len() > 10
}

pub fn is_valid_phone(phone: &str) -> bool {
    let cleaned: String = phone.chars().filter(char::is_ascii_digit).collect();
    cleaned.len() >= 7
        && phone
            .chars()
            .all(|c| c.is_ascii_digit() || " ()-+".contains(c))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_state_creation() {
        let error = ErrorState::new(
            "Test Error".to_string(),
            "Something went wrong".to_string(),
            Some("Details here".to_string()),
            Some("test-plugin".to_string()),
        );

        assert_eq!(error.title, "Test Error");
        assert_eq!(error.message, "Something went wrong");
        assert_eq!(error.details, Some("Details here".to_string()));
        assert_eq!(error.plugin_id, Some("test-plugin".to_string()));
    }

    #[test]
    fn test_error_state_minimal() {
        let error = ErrorState::new("Error".to_string(), "Failed".to_string(), None, None);

        assert_eq!(error.title, "Error");
        assert_eq!(error.message, "Failed");
        assert_eq!(error.details, None);
        assert_eq!(error.plugin_id, None);
    }
}
