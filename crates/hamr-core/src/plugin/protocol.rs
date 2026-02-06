use hamr_types::{
    Action, Badge, Chip, DisplayHint, FormFieldType, FormOption, GridItem, ImageItem, PluginAction,
    ResultItem as HamrResultItem,
};
use serde::{Deserialize, Serialize};

/// Index mode for plugin index updates
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IndexMode {
    Full,
    Incremental,
}

/// Source of an action sent to a plugin
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActionSource {
    Normal,
    Ambient,
}

/// Input sent to plugin handler (stdin)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginInput {
    pub step: Step,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected: Option<SelectedItem>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub form_data: Option<serde_json::Value>,

    /// Source of the action (normal, ambient)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<ActionSource>,
}

impl Default for PluginInput {
    fn default() -> Self {
        Self {
            step: Step::Initial,
            query: None,
            selected: None,
            action: None,
            session: None,
            context: None,
            value: None,
            form_data: None,
            source: None,
        }
    }
}

impl PluginInput {
    #[must_use]
    pub fn initial() -> Self {
        Self {
            step: Step::Initial,
            ..Default::default()
        }
    }

    #[must_use]
    pub fn search(query: impl Into<String>) -> Self {
        Self {
            step: Step::Search,
            query: Some(query.into()),
            ..Default::default()
        }
    }

    #[must_use]
    pub fn action(item_id: impl Into<String>) -> Self {
        Self {
            step: Step::Action,
            selected: Some(SelectedItem {
                id: item_id.into(),
                extra: None,
            }),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Step {
    Initial,
    Search,
    Action,
    Form,
    Match,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectedItem {
    pub id: String,

    #[serde(flatten)]
    pub extra: Option<serde_json::Value>,
}

/// Response from plugin handler (stdout)
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
// Deserialization type: boxing would add allocation overhead for every plugin response
#[allow(clippy::large_enum_variant)]
pub enum PluginResponse {
    Results {
        #[serde(alias = "results")]
        items: Vec<HamrResultItem>,

        #[serde(default)]
        prepend: bool,

        #[serde(default, rename = "inputMode")]
        input_mode: Option<String>,

        #[serde(default)]
        status: Option<StatusData>,

        #[serde(default)]
        context: Option<String>,

        #[serde(default)]
        placeholder: Option<String>,

        #[serde(default, rename = "clearInput")]
        clear_input: bool,

        #[serde(default, rename = "navigateForward")]
        navigate_forward: Option<bool>,

        #[serde(default, rename = "pluginActions")]
        plugin_actions: Vec<PluginAction>,

        #[serde(default, rename = "navigationDepth")]
        navigation_depth: Option<u32>,

        #[serde(
            default,
            rename = "displayHint",
            deserialize_with = "deserialize_display_hint"
        )]
        display_hint: Option<DisplayHint>,

        /// When true, activates this plugin for multi-step flow
        /// Used when an indexed item needs to enter a search/form flow from main search
        #[serde(default)]
        activate: bool,
    },

    Execute(ExecuteData),

    Card {
        card: CardResponseData,

        #[serde(default)]
        status: Option<StatusData>,

        #[serde(default)]
        context: Option<String>,
    },

    Form {
        form: FormResponseData,

        #[serde(default)]
        context: Option<String>,

        #[serde(default, rename = "navigateForward")]
        navigate_forward: Option<bool>,
    },

    Index {
        items: Vec<HamrResultItem>,

        #[serde(default)]
        mode: Option<IndexMode>,

        #[serde(default)]
        remove: Option<Vec<String>>,

        #[serde(default)]
        status: Option<StatusData>,
    },

    Status {
        status: StatusData,
    },

    Update {
        #[serde(default)]
        items: Option<Vec<UpdateItem>>,

        #[serde(default)]
        status: Option<StatusData>,
    },

    Error {
        message: String,

        #[serde(default)]
        details: Option<String>,
    },

    #[serde(rename = "imageBrowser")]
    ImageBrowser {
        #[serde(default)]
        images: Vec<ImageItem>,

        #[serde(default)]
        title: Option<String>,

        #[serde(default)]
        directory: Option<String>,

        #[serde(default, rename = "imageBrowser")]
        image_browser: Option<ImageBrowserInner>,
    },

    #[serde(rename = "gridBrowser")]
    GridBrowser {
        items: Vec<GridItem>,

        #[serde(default)]
        title: Option<String>,

        #[serde(default)]
        columns: Option<u32>,

        #[serde(default)]
        actions: Vec<Action>,
    },

    Prompt {
        prompt: PromptData,
    },

    Match {
        result: Option<HamrResultItem>,
    },

    /// No operation - plugin handled the action but has nothing to return
    Noop,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PromptData {
    pub text: String,

    #[serde(default)]
    pub placeholder: Option<String>,
}

/// Card response data (nested under "card" key in JSON)
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardResponseData {
    #[serde(default)]
    pub title: String,

    #[serde(default)]
    pub content: Option<String>,

    /// When true, content should be rendered as markdown
    /// When this is a string, it contains the markdown content directly
    #[serde(default, deserialize_with = "deserialize_markdown")]
    pub markdown: Option<String>,

    #[serde(default)]
    pub actions: Vec<Action>,

    #[serde(default)]
    pub kind: Option<String>,

    #[serde(default)]
    pub blocks: Vec<CardBlockData>,

    #[serde(default)]
    pub max_height: Option<u32>,

    #[serde(default)]
    pub show_details: Option<bool>,

    #[serde(default)]
    pub allow_toggle_details: Option<bool>,
}

/// Custom deserializer for markdown field that can be either bool or string
fn deserialize_markdown<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de;

    struct MarkdownVisitor;

    impl de::Visitor<'_> for MarkdownVisitor {
        type Value = Option<String>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a boolean or string")
        }

        fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            if v {
                Ok(Some(String::new())) // Marker that markdown=true was set
            } else {
                Ok(None)
            }
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Some(v.to_string()))
        }

        fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Some(v))
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }
    }

    deserializer.deserialize_any(MarkdownVisitor)
}

fn default_submit() -> String {
    "Submit".to_string()
}

/// Form response data (nested under "form" key in JSON)
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FormResponseData {
    pub title: String,
    pub fields: Vec<FormField>,

    #[serde(default = "default_submit")]
    pub submit_label: String,

    #[serde(default)]
    pub cancel_label: Option<String>,

    /// When true, changes are applied immediately without submit button
    #[serde(default)]
    pub live_update: bool,
}

/// Execute action data
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecuteData {
    #[serde(default)]
    pub launch: Option<String>,

    #[serde(default)]
    pub copy: Option<String>,

    #[serde(default)]
    pub type_text: Option<String>,

    #[serde(default)]
    pub open_url: Option<String>,

    #[serde(default)]
    pub open: Option<String>,

    #[serde(default)]
    pub notify: Option<String>,

    #[serde(default)]
    pub sound: Option<String>,

    #[serde(default)]
    pub close: Option<bool>,

    #[serde(default)]
    pub keep_open: bool,
}

/// Re-export `ResultItem` from `hamr_types` as `IndexItem` for compatibility
pub type IndexItem = HamrResultItem;

/// Update item (partial update)
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateItem {
    pub id: String,

    #[serde(flatten)]
    pub fields: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusData {
    #[serde(default)]
    pub badges: Vec<Badge>,

    #[serde(default)]
    pub chips: Vec<Chip>,

    #[serde(default)]
    pub description: Option<String>,

    #[serde(default)]
    pub fab: Option<FabData>,

    #[serde(default, deserialize_with = "deserialize_nullable_ambient")]
    pub ambient: Option<Vec<AmbientItemData>>,
}

fn deserialize_nullable_ambient<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<Vec<AmbientItemData>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    // Deserialize as raw Value to distinguish between:
    // - Field missing (will use #[serde(default)] → None)
    // - Field present with null → Some(vec![]) (clear items)
    // - Field present with array → Some(items)
    let value: serde_json::Value = serde_json::Value::deserialize(deserializer)?;
    match value {
        serde_json::Value::Null => Ok(Some(vec![])),
        serde_json::Value::Array(arr) => {
            let items: std::result::Result<Vec<AmbientItemData>, _> =
                arr.into_iter().map(serde_json::from_value).collect();
            items.map(Some).map_err(serde::de::Error::custom)
        }
        other => Err(serde::de::Error::custom(format!(
            "expected null or array for ambient, got {other:?}"
        ))),
    }
}

/// FAB override data from plugin
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FabData {
    #[serde(default)]
    pub badges: Vec<Badge>,

    #[serde(default)]
    pub chips: Vec<Chip>,

    #[serde(default)]
    pub priority: i32,

    #[serde(default)]
    pub show_fab: bool,
}

/// Ambient item data from plugin
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AmbientItemData {
    pub id: String,
    pub name: String,

    #[serde(default)]
    pub description: Option<String>,

    #[serde(default)]
    pub icon: Option<String>,

    #[serde(default)]
    pub badges: Vec<Badge>,

    #[serde(default)]
    pub chips: Vec<Chip>,

    #[serde(default)]
    pub actions: Vec<Action>,

    /// Duration in ms before auto-removal (0 = permanent)
    #[serde(default)]
    pub duration: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FormField {
    pub id: String,
    pub label: String,

    #[serde(
        default,
        rename = "type",
        deserialize_with = "deserialize_form_field_type"
    )]
    pub field_type: Option<FormFieldType>,

    #[serde(default)]
    pub placeholder: Option<String>,

    #[serde(default, alias = "default")]
    pub default_value: Option<String>,

    #[serde(default)]
    pub required: bool,

    #[serde(default)]
    pub options: Vec<FormOption>,

    /// Help text displayed below the field
    #[serde(default)]
    pub hint: Option<String>,

    /// Number of rows for textarea fields
    #[serde(default)]
    pub rows: Option<u32>,

    /// Minimum value for slider fields
    #[serde(default)]
    pub min: Option<f64>,

    /// Maximum value for slider fields
    #[serde(default)]
    pub max: Option<f64>,

    /// Step value for slider fields
    #[serde(default)]
    pub step: Option<f64>,
}

/// Inner imageBrowser object (QML protocol compatibility)
#[derive(Debug, Clone, Deserialize)]
pub struct ImageBrowserInner {
    #[serde(default)]
    pub directory: Option<String>,

    #[serde(default)]
    pub images: Vec<ImageItem>,
}

/// Deserialize `DisplayHint` with alias support for plugin protocol values.
///
/// Handles `"largegrid"` (no underscore) in addition to the standard `"large_grid"`.
fn deserialize_display_hint<'de, D>(deserializer: D) -> Result<Option<DisplayHint>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt: Option<String> = Option::deserialize(deserializer)?;
    match opt {
        None => Ok(None),
        Some(s) => match s.to_lowercase().as_str() {
            "auto" => Ok(Some(DisplayHint::Auto)),
            "list" => Ok(Some(DisplayHint::List)),
            "grid" => Ok(Some(DisplayHint::Grid)),
            "large_grid" | "largegrid" => Ok(Some(DisplayHint::LargeGrid)),
            _ => Ok(None),
        },
    }
}

/// Deserialize `FormFieldType` with alias support for plugin protocol values.
///
/// Handles `"textarea"` (no underscore) in addition to the standard `"text_area"`,
/// and `"toggle"` as an alias for `"switch"`.
fn deserialize_form_field_type<'de, D>(deserializer: D) -> Result<Option<FormFieldType>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt: Option<String> = Option::deserialize(deserializer)?;
    match opt {
        None => Ok(None),
        Some(s) => match s.as_str() {
            "password" => Ok(Some(FormFieldType::Password)),
            "number" => Ok(Some(FormFieldType::Number)),
            "textarea" | "text_area" => Ok(Some(FormFieldType::TextArea)),
            "select" => Ok(Some(FormFieldType::Select)),
            "checkbox" => Ok(Some(FormFieldType::Checkbox)),
            "switch" | "toggle" => Ok(Some(FormFieldType::Switch)),
            "slider" => Ok(Some(FormFieldType::Slider)),
            "hidden" => Ok(Some(FormFieldType::Hidden)),
            "date" => Ok(Some(FormFieldType::Date)),
            "time" => Ok(Some(FormFieldType::Time)),
            "email" => Ok(Some(FormFieldType::Email)),
            "url" => Ok(Some(FormFieldType::Url)),
            "phone" => Ok(Some(FormFieldType::Phone)),
            // "text" and unknown types default to Text
            _ => Ok(Some(FormFieldType::Text)),
        },
    }
}

/// Block data for rich cards (from plugin response)
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum CardBlockData {
    Pill { text: String },
    Separator,
    Message { role: String, content: String },
    Note { content: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_step_serializes_lowercase() {
        let json = serde_json::to_string(&Step::Initial).unwrap();
        assert_eq!(json, "\"initial\"");

        let json = serde_json::to_string(&Step::Search).unwrap();
        assert_eq!(json, "\"search\"");

        let json = serde_json::to_string(&Step::Action).unwrap();
        assert_eq!(json, "\"action\"");

        let json = serde_json::to_string(&Step::Form).unwrap();
        assert_eq!(json, "\"form\"");
    }

    #[test]
    fn test_plugin_input_serializes_camel_case() {
        let input = PluginInput::search("test");
        let json = serde_json::to_string(&input).unwrap();
        assert!(json.contains("\"step\":\"search\""));
        assert!(json.contains("\"query\":\"test\""));
        assert!(!json.contains("selected")); // None fields skipped
    }

    #[test]
    fn test_plugin_input_with_selected_item() {
        let input = PluginInput {
            selected: Some(SelectedItem {
                id: "item-1".into(),
                extra: Some(json!({"foo": "bar"})),
            }),
            action: Some("open".into()),
            ..PluginInput::action("item-1")
        };
        let json = serde_json::to_string(&input).unwrap();
        assert!(json.contains("\"id\":\"item-1\""));
        assert!(json.contains("\"action\":\"open\""));
    }

    #[test]
    fn test_plugin_input_default() {
        let input = PluginInput::default();
        let json = serde_json::to_string(&input).unwrap();
        assert!(json.contains("\"step\":\"initial\""));
        assert!(!json.contains("query"));
        assert!(!json.contains("selected"));
    }

    #[test]
    fn test_plugin_input_factory_initial() {
        let input = PluginInput::initial();
        let json = serde_json::to_string(&input).unwrap();
        assert!(json.contains("\"step\":\"initial\""));
    }

    #[test]
    fn test_plugin_input_factory_search() {
        let input = PluginInput::search("hello");
        let json = serde_json::to_string(&input).unwrap();
        assert!(json.contains("\"step\":\"search\""));
        assert!(json.contains("\"query\":\"hello\""));
    }

    #[test]
    fn test_plugin_input_factory_action() {
        let input = PluginInput::action("my-item");
        let json = serde_json::to_string(&input).unwrap();
        assert!(json.contains("\"step\":\"action\""));
        assert!(json.contains("\"id\":\"my-item\""));
    }

    #[test]
    fn test_selected_item_flattens_extra() {
        let item = SelectedItem {
            id: "test-id".into(),
            extra: Some(json!({"custom": "value", "num": 42})),
        };
        let json = serde_json::to_string(&item).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["id"], "test-id");
        assert_eq!(parsed["custom"], "value");
        assert_eq!(parsed["num"], 42);
    }

    #[test]
    fn test_plugin_response_results() {
        let json = json!({
            "type": "results",
            "items": [{"id": "1", "name": "Item 1"}],
            "prepend": true,
            "clearInput": true
        });
        let response: PluginResponse = serde_json::from_value(json).unwrap();
        match response {
            PluginResponse::Results {
                items,
                prepend,
                clear_input,
                ..
            } => {
                assert_eq!(items.len(), 1);
                assert!(prepend);
                assert!(clear_input);
            }
            _ => panic!("Expected Results"),
        }
    }

    #[test]
    fn test_plugin_response_results_alias() {
        let json = json!({
            "type": "results",
            "results": [{"id": "1", "name": "Item 1"}]
        });
        let response: PluginResponse = serde_json::from_value(json).unwrap();
        match response {
            PluginResponse::Results { items, .. } => {
                assert_eq!(items.len(), 1);
            }
            _ => panic!("Expected Results"),
        }
    }

    #[test]
    fn test_plugin_response_execute() {
        let json = json!({
            "type": "execute",
            "launch": "/usr/bin/firefox",
            "close": true
        });
        let response: PluginResponse = serde_json::from_value(json).unwrap();
        match response {
            PluginResponse::Execute(data) => {
                assert_eq!(data.launch, Some("/usr/bin/firefox".into()));
                assert_eq!(data.close, Some(true));
            }
            _ => panic!("Expected Execute"),
        }
    }

    #[test]
    fn test_plugin_response_card() {
        let json = json!({
            "type": "card",
            "card": {
                "title": "Test Card",
                "content": "Some content",
                "actions": []
            }
        });
        let response: PluginResponse = serde_json::from_value(json).unwrap();
        match response {
            PluginResponse::Card { card, .. } => {
                assert_eq!(card.title, "Test Card");
                assert_eq!(card.content, Some("Some content".into()));
            }
            _ => panic!("Expected Card"),
        }
    }

    #[test]
    fn test_card_markdown_bool_true() {
        let json = json!({
            "title": "Test",
            "markdown": true
        });
        let card: CardResponseData = serde_json::from_value(json).unwrap();
        assert_eq!(card.markdown, Some(String::new()));
    }

    #[test]
    fn test_card_markdown_bool_false() {
        let json = json!({
            "title": "Test",
            "markdown": false
        });
        let card: CardResponseData = serde_json::from_value(json).unwrap();
        assert_eq!(card.markdown, None);
    }

    #[test]
    fn test_card_markdown_string() {
        let json = json!({
            "title": "Test",
            "markdown": "# Heading\n\nContent"
        });
        let card: CardResponseData = serde_json::from_value(json).unwrap();
        assert_eq!(card.markdown, Some("# Heading\n\nContent".into()));
    }

    #[test]
    fn test_card_markdown_missing() {
        let json = json!({
            "title": "Test"
        });
        let card: CardResponseData = serde_json::from_value(json).unwrap();
        assert_eq!(card.markdown, None);
    }

    #[test]
    fn test_plugin_response_form() {
        let json = json!({
            "type": "form",
            "form": {
                "title": "Settings",
                "fields": [
                    {"id": "name", "label": "Name", "type": "text"}
                ]
            }
        });
        let response: PluginResponse = serde_json::from_value(json).unwrap();
        match response {
            PluginResponse::Form { form, .. } => {
                assert_eq!(form.title, "Settings");
                assert_eq!(form.fields.len(), 1);
                assert_eq!(form.submit_label, "Submit"); // default
            }
            _ => panic!("Expected Form"),
        }
    }

    #[test]
    fn test_form_live_update() {
        let json = json!({
            "type": "form",
            "form": {
                "title": "Live Form",
                "fields": [],
                "liveUpdate": true,
                "submitLabel": "Apply"
            }
        });
        let response: PluginResponse = serde_json::from_value(json).unwrap();
        match response {
            PluginResponse::Form { form, .. } => {
                assert!(form.live_update);
                assert_eq!(form.submit_label, "Apply");
            }
            _ => panic!("Expected Form"),
        }
    }

    #[test]
    fn test_plugin_response_index() {
        let json = json!({
            "type": "index",
            "items": [{"id": "1", "name": "App"}],
            "mode": "incremental",
            "remove": ["old-id"]
        });
        let response: PluginResponse = serde_json::from_value(json).unwrap();
        match response {
            PluginResponse::Index {
                items,
                mode,
                remove,
                ..
            } => {
                assert_eq!(items.len(), 1);
                assert_eq!(mode, Some(IndexMode::Incremental));
                assert_eq!(remove, Some(vec!["old-id".into()]));
            }
            _ => panic!("Expected Index"),
        }
    }

    #[test]
    fn test_plugin_response_status() {
        let json = json!({
            "type": "status",
            "status": {
                "badges": [{"text": "OK", "color": "green"}],
                "description": "All good"
            }
        });
        let response: PluginResponse = serde_json::from_value(json).unwrap();
        match response {
            PluginResponse::Status { status } => {
                assert_eq!(status.badges.len(), 1);
                assert_eq!(status.description, Some("All good".into()));
            }
            _ => panic!("Expected Status"),
        }
    }

    #[test]
    fn test_status_ambient_array() {
        let json = json!({
            "badges": [],
            "ambient": [
                {"id": "notif-1", "name": "Notification"}
            ]
        });
        let status: StatusData = serde_json::from_value(json).unwrap();
        let ambient = status.ambient.unwrap();
        assert_eq!(ambient.len(), 1);
        assert_eq!(ambient[0].id, "notif-1");
    }

    #[test]
    fn test_status_ambient_null_clears() {
        let json = json!({
            "badges": [],
            "ambient": null
        });
        let status: StatusData = serde_json::from_value(json).unwrap();
        let ambient = status.ambient.unwrap();
        assert!(ambient.is_empty());
    }

    #[test]
    fn test_status_ambient_missing() {
        let json = json!({
            "badges": []
        });
        let status: StatusData = serde_json::from_value(json).unwrap();
        assert!(status.ambient.is_none());
    }

    #[test]
    fn test_plugin_response_update() {
        let json = json!({
            "type": "update",
            "items": [
                {"id": "1", "name": "Updated Name"}
            ]
        });
        let response: PluginResponse = serde_json::from_value(json).unwrap();
        match response {
            PluginResponse::Update { items, .. } => {
                let items = items.unwrap();
                assert_eq!(items.len(), 1);
                assert_eq!(items[0].id, "1");
            }
            _ => panic!("Expected Update"),
        }
    }

    #[test]
    fn test_plugin_response_error() {
        let json = json!({
            "type": "error",
            "message": "Something went wrong",
            "details": "Stack trace..."
        });
        let response: PluginResponse = serde_json::from_value(json).unwrap();
        match response {
            PluginResponse::Error { message, details } => {
                assert_eq!(message, "Something went wrong");
                assert_eq!(details, Some("Stack trace...".into()));
            }
            _ => panic!("Expected Error"),
        }
    }

    #[test]
    fn test_plugin_response_image_browser() {
        let json = json!({
            "type": "imageBrowser",
            "directory": "/home/user/pictures",
            "title": "Gallery"
        });
        let response: PluginResponse = serde_json::from_value(json).unwrap();
        match response {
            PluginResponse::ImageBrowser {
                directory, title, ..
            } => {
                assert_eq!(directory, Some("/home/user/pictures".into()));
                assert_eq!(title, Some("Gallery".into()));
            }
            _ => panic!("Expected ImageBrowser"),
        }
    }

    #[test]
    fn test_plugin_response_grid_browser() {
        let json = json!({
            "type": "gridBrowser",
            "items": [{"id": "1", "name": "Item", "icon": "test"}],
            "columns": 4
        });
        let response: PluginResponse = serde_json::from_value(json).unwrap();
        match response {
            PluginResponse::GridBrowser { items, columns, .. } => {
                assert_eq!(items.len(), 1);
                assert_eq!(columns, Some(4));
            }
            _ => panic!("Expected GridBrowser"),
        }
    }

    #[test]
    fn test_plugin_response_prompt() {
        let json = json!({
            "type": "prompt",
            "prompt": {
                "text": "Enter your name",
                "placeholder": "John Doe"
            }
        });
        let response: PluginResponse = serde_json::from_value(json).unwrap();
        match response {
            PluginResponse::Prompt { prompt } => {
                assert_eq!(prompt.text, "Enter your name");
                assert_eq!(prompt.placeholder, Some("John Doe".into()));
            }
            _ => panic!("Expected Prompt"),
        }
    }

    #[test]
    fn test_plugin_response_match() {
        let json = json!({
            "type": "match",
            "result": {"id": "1", "name": "Matched"}
        });
        let response: PluginResponse = serde_json::from_value(json).unwrap();
        match response {
            PluginResponse::Match { result } => {
                assert!(result.is_some());
                assert_eq!(result.unwrap().name, "Matched");
            }
            _ => panic!("Expected Match"),
        }
    }

    #[test]
    fn test_plugin_response_match_empty() {
        let json = json!({
            "type": "match",
            "result": null
        });
        let response: PluginResponse = serde_json::from_value(json).unwrap();
        match response {
            PluginResponse::Match { result } => {
                assert!(result.is_none());
            }
            _ => panic!("Expected Match"),
        }
    }

    #[test]
    fn test_plugin_response_noop() {
        let json = json!({"type": "noop"});
        let response: PluginResponse = serde_json::from_value(json).unwrap();
        assert!(matches!(response, PluginResponse::Noop));
    }

    #[test]
    fn test_card_block_pill() {
        let json = json!({"type": "pill", "text": "Python"});
        let block: CardBlockData = serde_json::from_value(json).unwrap();
        match block {
            CardBlockData::Pill { text } => assert_eq!(text, "Python"),
            _ => panic!("Expected Pill"),
        }
    }

    #[test]
    fn test_card_block_separator() {
        let json = json!({"type": "separator"});
        let block: CardBlockData = serde_json::from_value(json).unwrap();
        assert!(matches!(block, CardBlockData::Separator));
    }

    #[test]
    fn test_card_block_message() {
        let json = json!({"type": "message", "role": "user", "content": "Hello"});
        let block: CardBlockData = serde_json::from_value(json).unwrap();
        match block {
            CardBlockData::Message { role, content } => {
                assert_eq!(role, "user");
                assert_eq!(content, "Hello");
            }
            _ => panic!("Expected Message"),
        }
    }

    #[test]
    fn test_card_block_note() {
        let json = json!({"type": "note", "content": "Important info"});
        let block: CardBlockData = serde_json::from_value(json).unwrap();
        match block {
            CardBlockData::Note { content } => assert_eq!(content, "Important info"),
            _ => panic!("Expected Note"),
        }
    }

    #[test]
    fn test_form_field_all_options() {
        let json = json!({
            "id": "volume",
            "label": "Volume",
            "type": "slider",
            "placeholder": "Set volume",
            "default": "50",
            "required": true,
            "hint": "0-100",
            "min": 0,
            "max": 100,
            "step": 5
        });
        let field: FormField = serde_json::from_value(json).unwrap();
        assert_eq!(field.id, "volume");
        assert_eq!(field.label, "Volume");
        assert_eq!(field.field_type, Some(FormFieldType::Slider));
        assert_eq!(field.default_value, Some("50".into()));
        assert!(field.required);
        assert_eq!(field.min, Some(0.0));
        assert_eq!(field.max, Some(100.0));
        assert_eq!(field.step, Some(5.0));
    }

    #[test]
    fn test_form_field_with_options() {
        let json = json!({
            "id": "color",
            "label": "Color",
            "type": "select",
            "options": [
                {"value": "red", "label": "Red"},
                {"value": "blue", "label": "Blue"}
            ]
        });
        let field: FormField = serde_json::from_value(json).unwrap();
        assert_eq!(field.options.len(), 2);
        assert_eq!(field.options[0].value, "red");
    }

    #[test]
    fn test_fab_data() {
        let json = json!({
            "badges": [{"text": "3"}],
            "chips": [],
            "priority": 10,
            "showFab": true
        });
        let fab: FabData = serde_json::from_value(json).unwrap();
        assert_eq!(fab.badges.len(), 1);
        assert_eq!(fab.priority, 10);
        assert!(fab.show_fab);
    }

    #[test]
    fn test_ambient_item_data() {
        let json = json!({
            "id": "music-1",
            "name": "Now Playing",
            "description": "Song Title",
            "icon": "music",
            "duration": 5000
        });
        let item: AmbientItemData = serde_json::from_value(json).unwrap();
        assert_eq!(item.id, "music-1");
        assert_eq!(item.name, "Now Playing");
        assert_eq!(item.duration, 5000);
    }

    #[test]
    fn test_execute_data_all_fields() {
        let json = json!({
            "launch": "/usr/bin/app",
            "copy": "copied text",
            "typeText": "typed",
            "openUrl": "https://example.com",
            "open": "/path/to/file",
            "notify": "Notification",
            "sound": "notification.wav",
            "close": true,
            "keepOpen": false
        });
        let data: ExecuteData = serde_json::from_value(json).unwrap();
        assert_eq!(data.launch, Some("/usr/bin/app".into()));
        assert_eq!(data.copy, Some("copied text".into()));
        assert_eq!(data.type_text, Some("typed".into()));
        assert_eq!(data.open_url, Some("https://example.com".into()));
        assert_eq!(data.open, Some("/path/to/file".into()));
        assert_eq!(data.notify, Some("Notification".into()));
        assert_eq!(data.sound, Some("notification.wav".into()));
        assert_eq!(data.close, Some(true));
        assert!(!data.keep_open);
    }

    #[test]
    fn test_image_browser_inner() {
        let json = json!({
            "type": "imageBrowser",
            "imageBrowser": {
                "directory": "/home/pics",
                "images": [{"path": "/home/pics/1.jpg"}]
            }
        });
        let response: PluginResponse = serde_json::from_value(json).unwrap();
        match response {
            PluginResponse::ImageBrowser { image_browser, .. } => {
                let inner = image_browser.unwrap();
                assert_eq!(inner.directory, Some("/home/pics".into()));
                assert_eq!(inner.images.len(), 1);
            }
            _ => panic!("Expected ImageBrowser"),
        }
    }
}
