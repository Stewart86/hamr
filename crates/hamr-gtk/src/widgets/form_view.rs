#![allow(dead_code)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Orientation};

use hamr_types::{FormData, FormField, FormFieldType};

type OnChangeCallback = Rc<dyn Fn(String, String, HashMap<String, String>)>;

pub struct FormView {
    container: GtkBox,
    fields: Rc<RefCell<HashMap<String, FieldWidget>>>,
    live_update: bool,
    on_change: RefCell<Option<OnChangeCallback>>,
}

#[derive(Clone)]
enum FieldWidget {
    Text(gtk4::Entry),
    Password(gtk4::PasswordEntry),
    TextArea(gtk4::TextView),
    Select(gtk4::DropDown, Vec<String>),
    Checkbox(gtk4::CheckButton),
    Switch(gtk4::Switch),
    Slider(gtk4::Scale),
    Hidden,
}

impl FormView {
    pub fn new(form: &FormData) -> Self {
        let container = GtkBox::new(Orientation::Vertical, 12);
        container.add_css_class("form-fields");
        let mut fields = HashMap::new();

        for field in &form.fields {
            let (widget, field_widget) = Self::build_field(field);
            if let Some(w) = widget {
                container.append(&w);
            }
            fields.insert(field.id.clone(), field_widget);
        }

        Self {
            container,
            fields: Rc::new(std::cell::RefCell::new(fields)),
            live_update: form.live_update,
            on_change: RefCell::new(None),
        }
    }

    // 1:1 FormFieldType variant mapping - each arm creates appropriate GTK widget
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::too_many_lines
    )]
    fn build_field(field: &FormField) -> (Option<gtk4::Widget>, FieldWidget) {
        let container = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(4)
            .build();

        let label = gtk4::Label::builder()
            .label(&field.label)
            .halign(gtk4::Align::Start)
            .css_classes(["form-field-label"])
            .build();
        container.append(&label);

        let (widget, field_widget) = match field.field_type {
            FormFieldType::Text
            | FormFieldType::Email
            | FormFieldType::Url
            | FormFieldType::Phone => {
                let entry = gtk4::Entry::new();
                entry.add_css_class("form-entry");
                entry.set_placeholder_text(field.placeholder.as_deref());
                if let Some(default_value) = field.default_value.as_deref() {
                    entry.set_text(default_value);
                }
                (
                    Some(entry.clone().upcast::<gtk4::Widget>()),
                    FieldWidget::Text(entry),
                )
            }
            FormFieldType::Password => {
                let entry = gtk4::PasswordEntry::new();
                entry.add_css_class("form-entry");
                entry.set_placeholder_text(field.placeholder.as_deref());
                if let Some(default_value) = field.default_value.as_deref() {
                    entry.set_text(default_value);
                }
                (
                    Some(entry.clone().upcast::<gtk4::Widget>()),
                    FieldWidget::Password(entry),
                )
            }
            FormFieldType::Number => {
                let entry = gtk4::Entry::new();
                entry.add_css_class("form-entry");
                entry.set_input_purpose(gtk4::InputPurpose::Digits);
                entry.set_placeholder_text(field.placeholder.as_deref());
                if let Some(default_value) = field.default_value.as_deref() {
                    entry.set_text(default_value);
                }
                (
                    Some(entry.clone().upcast::<gtk4::Widget>()),
                    FieldWidget::Text(entry),
                )
            }
            FormFieldType::TextArea => {
                let text = gtk4::TextView::new();
                text.add_css_class("form-textarea");
                text.set_wrap_mode(gtk4::WrapMode::Word);
                if let Some(default_value) = field.default_value.as_deref() {
                    text.buffer().set_text(default_value);
                }
                (
                    Some(text.clone().upcast::<gtk4::Widget>()),
                    FieldWidget::TextArea(text),
                )
            }
            FormFieldType::Select => {
                let option_labels: Vec<&str> =
                    field.options.iter().map(|o| o.label.as_str()).collect();
                let model = gtk4::StringList::new(&option_labels);
                let dropdown = gtk4::DropDown::new(Some(model), None::<&gtk4::Expression>);
                let mut selected_index = 0u32;
                if let Some(default_value) = field.default_value.as_deref()
                    && let Some(index) = field.options.iter().position(|o| o.value == default_value)
                {
                    selected_index = index as u32;
                }
                dropdown.set_selected(selected_index);
                (
                    Some(dropdown.clone().upcast::<gtk4::Widget>()),
                    FieldWidget::Select(
                        dropdown,
                        field.options.iter().map(|o| o.value.clone()).collect(),
                    ),
                )
            }
            FormFieldType::Checkbox => {
                let check = gtk4::CheckButton::new();
                if let Some(default_value) = field.default_value.as_deref() {
                    let active = matches!(default_value, "true" | "1" | "yes" | "on");
                    check.set_active(active);
                }
                (
                    Some(check.clone().upcast::<gtk4::Widget>()),
                    FieldWidget::Checkbox(check),
                )
            }
            FormFieldType::Switch => {
                let switch = gtk4::Switch::new();
                if let Some(default_value) = field.default_value.as_deref() {
                    let active = matches!(default_value, "true" | "1" | "yes" | "on");
                    switch.set_active(active);
                }
                (
                    Some(switch.clone().upcast::<gtk4::Widget>()),
                    FieldWidget::Switch(switch),
                )
            }
            FormFieldType::Slider => {
                let min = field.min.unwrap_or(0.0);
                let max = field.max.unwrap_or(100.0);
                let step = field.step.unwrap_or(1.0);
                let scale = gtk4::Scale::with_range(Orientation::Horizontal, min, max, step);
                if let Some(default_value) = field.default_value.as_deref()
                    && let Ok(value) = default_value.parse::<f64>()
                {
                    scale.set_value(value);
                }
                scale.set_draw_value(true);
                (
                    Some(scale.clone().upcast::<gtk4::Widget>()),
                    FieldWidget::Slider(scale),
                )
            }
            FormFieldType::Hidden | FormFieldType::Date | FormFieldType::Time => {
                (None, FieldWidget::Hidden)
            }
        };

        if let Some(widget) = widget {
            container.append(&widget);
            (Some(container.upcast()), field_widget)
        } else {
            (None, field_widget)
        }
    }

    pub fn widget(&self) -> &GtkBox {
        &self.container
    }

    pub fn collect_values(&self) -> HashMap<String, String> {
        collect_values(&self.fields)
    }

    pub fn set_on_change<F>(&self, f: F)
    where
        F: Fn(String, String, HashMap<String, String>) + 'static,
    {
        let callback: OnChangeCallback = Rc::new(f);
        *self.on_change.borrow_mut() = Some(callback.clone());
        self.attach_live_update_handlers(&callback);
    }

    fn attach_live_update_handlers(&self, callback: &OnChangeCallback) {
        for (id, field) in self.fields.borrow().iter() {
            match field {
                FieldWidget::Text(entry) => {
                    let id = id.clone();
                    let cb = Rc::clone(callback);
                    let fields = self.fields.clone();
                    entry.connect_changed(move |e| {
                        let value = e.text().to_string();
                        let all = collect_values(&fields);
                        cb(id.clone(), value, all);
                    });
                }
                FieldWidget::Password(entry) => {
                    let id = id.clone();
                    let cb = Rc::clone(callback);
                    let fields = self.fields.clone();
                    entry.connect_changed(move |e| {
                        let value = e.text().to_string();
                        let all = collect_values(&fields);
                        cb(id.clone(), value, all);
                    });
                }
                FieldWidget::TextArea(text) => {
                    let id = id.clone();
                    let cb = Rc::clone(callback);
                    let fields = self.fields.clone();
                    let buffer = text.buffer();
                    buffer.connect_changed(move |buf: &gtk4::TextBuffer| {
                        let value = buf
                            .text(&buf.start_iter(), &buf.end_iter(), true)
                            .to_string();
                        let all = collect_values(&fields);
                        cb(id.clone(), value, all);
                    });
                }
                FieldWidget::Select(dropdown, options) => {
                    let id = id.clone();
                    let cb = Rc::clone(callback);
                    let fields = self.fields.clone();
                    let options = options.clone();
                    dropdown.connect_selected_notify(move |dd| {
                        let index = dd.selected() as usize;
                        let value = options.get(index).cloned().unwrap_or_default();
                        let all = collect_values(&fields);
                        cb(id.clone(), value, all);
                    });
                }
                FieldWidget::Checkbox(check) => {
                    let id = id.clone();
                    let cb = Rc::clone(callback);
                    let fields = self.fields.clone();
                    check.connect_toggled(move |c| {
                        let value = c.is_active().to_string();
                        let all = collect_values(&fields);
                        cb(id.clone(), value, all);
                    });
                }
                FieldWidget::Switch(switch) => {
                    let id = id.clone();
                    let cb = Rc::clone(callback);
                    let fields = self.fields.clone();
                    switch.connect_state_set(move |s, _| {
                        let value = s.is_active().to_string();
                        let all = collect_values(&fields);
                        cb(id.clone(), value, all);
                        false.into()
                    });
                }
                FieldWidget::Slider(scale) => {
                    let id = id.clone();
                    let cb = Rc::clone(callback);
                    let fields = self.fields.clone();
                    scale.connect_value_changed(move |s| {
                        let value = s.value().to_string();
                        let all = collect_values(&fields);
                        cb(id.clone(), value, all);
                    });
                }
                FieldWidget::Hidden => {}
            }
        }
    }
}

fn collect_values(
    fields: &Rc<std::cell::RefCell<HashMap<String, FieldWidget>>>,
) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for (id, field) in fields.borrow().iter() {
        let value = match field {
            FieldWidget::Text(e) => e.text().to_string(),
            FieldWidget::Password(e) => e.text().to_string(),
            FieldWidget::TextArea(t) => {
                let buf = t.buffer();
                buf.text(&buf.start_iter(), &buf.end_iter(), true)
                    .to_string()
            }
            FieldWidget::Select(dropdown, options) => {
                let index = dropdown.selected() as usize;
                options.get(index).cloned().unwrap_or_default()
            }
            FieldWidget::Checkbox(c) => c.is_active().to_string(),
            FieldWidget::Switch(s) => s.is_active().to_string(),
            FieldWidget::Slider(s) => s.value().to_string(),
            FieldWidget::Hidden => String::new(),
        };
        map.insert(id.clone(), value);
    }
    map
}
