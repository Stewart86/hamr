//! Shared types for Hamr launcher components.
//!
//! This crate provides the core types used across hamr-core, hamr-rpc,
//! hamr-daemon, and hamr-tui. All types are serializable for RPC transport.

use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::path::PathBuf;

/// Deserialize a Vec that may be null or missing (both become empty vec)
fn deserialize_null_as_empty_vec<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    let opt: Option<Vec<T>> = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

/// Events sent from UI to core
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CoreEvent {
    /// Query text changed (realtime search)
    QueryChanged { query: String },

    /// Query submitted (enter pressed) - context is for multi-step flows
    QuerySubmitted {
        query: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        context: Option<String>,
    },

    /// Item selected with optional action
    ItemSelected {
        id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        action: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        plugin_id: Option<String>,
    },

    /// Ambient item action triggered
    AmbientAction {
        plugin_id: String,
        item_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        action: Option<String>,
    },

    /// Slider value changed
    SliderChanged {
        id: String,
        value: f64,
        #[serde(skip_serializing_if = "Option::is_none")]
        plugin_id: Option<String>,
    },

    /// Switch toggled
    SwitchToggled {
        id: String,
        value: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        plugin_id: Option<String>,
    },

    /// Navigate back in plugin
    Back,

    /// Cancel/close current context
    Cancel,

    /// Open a specific plugin
    OpenPlugin { plugin_id: String },

    /// Close current plugin
    ClosePlugin,

    /// Launcher window opened
    LauncherOpened,

    /// Launcher window closed
    LauncherClosed,

    /// Request to refresh plugin index
    RefreshIndex { plugin_id: String },

    /// Dismiss an ambient item
    DismissAmbient { plugin_id: String, item_id: String },

    /// Form submitted with field values
    FormSubmitted {
        form_data: HashMap<String, String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        context: Option<String>,
    },

    /// Form cancelled
    FormCancelled,

    /// Set the active plugin context (for multi-step flows)
    SetContext {
        #[serde(skip_serializing_if = "Option::is_none")]
        context: Option<String>,
    },

    /// Form field changed (for live update forms)
    FormFieldChanged {
        field_id: String,
        value: String,
        form_data: HashMap<String, String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        context: Option<String>,
    },

    /// Plugin action triggered (Ctrl+1-6 from toolbar)
    PluginActionTriggered { action_id: String },
}

/// Updates sent from core to UI
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CoreUpdate {
    /// Search results to display (full replacement)
    Results {
        results: Vec<SearchResult>,
        /// Optional placeholder for the search input
        #[serde(skip_serializing_if = "Option::is_none")]
        placeholder: Option<String>,
        /// Optional flag to clear input
        #[serde(rename = "clearInput", skip_serializing_if = "Option::is_none")]
        clear_input: Option<bool>,
        /// Optional input mode (realtime/submit)
        #[serde(rename = "inputMode", skip_serializing_if = "Option::is_none")]
        input_mode: Option<String>,
        /// Optional context for multi-step flows
        #[serde(skip_serializing_if = "Option::is_none")]
        context: Option<String>,
        /// Optional flag to navigate forward (increment depth)
        #[serde(rename = "navigateForward", skip_serializing_if = "Option::is_none")]
        navigate_forward: Option<bool>,
        /// Display hint for preferred view mode (list vs grid)
        #[serde(
            rename = "displayHint",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        display_hint: Option<DisplayHint>,
    },

    /// Partial update to existing results (patch by id)
    ResultsUpdate { patches: Vec<ResultPatch> },

    /// Card content to display
    Card {
        card: CardData,
        #[serde(skip_serializing_if = "Option::is_none")]
        context: Option<String>,
    },

    /// Form to display
    Form { form: FormData },

    /// Plugin activated
    PluginActivated {
        id: String,
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        icon: Option<String>,
    },

    /// Request plugin activation (internal, processed by daemon to call `core.activate_plugin`)
    /// Used when a plugin response signals it needs multi-step flow
    #[serde(skip)]
    ActivatePlugin { plugin_id: String },

    /// Plugin deactivated
    PluginDeactivated,

    /// Loading state changed
    Busy { busy: bool },

    /// Error occurred
    Error { message: String },

    /// Prompt text changed
    Prompt { prompt: String },

    /// Placeholder text changed
    Placeholder { placeholder: String },

    /// Execute an action
    Execute { action: ExecuteAction },

    /// Request to close/hide launcher
    Close,

    /// Request to show launcher
    Show,

    /// Request to toggle launcher (respects intuitive mode on client)
    Toggle,

    /// Clear the search input
    ClearInput,

    /// Input mode changed (realtime/submit)
    InputModeChanged { mode: String },

    /// Context changed for multi-step plugin flows
    ContextChanged {
        #[serde(skip_serializing_if = "Option::is_none")]
        context: Option<String>,
    },

    /// Plugin status updated (badges, chips, description)
    PluginStatusUpdate {
        plugin_id: String,
        status: PluginStatus,
    },

    /// Ambient items updated for a plugin
    AmbientUpdate {
        plugin_id: String,
        items: Vec<AmbientItem>,
    },

    /// FAB override updated
    FabUpdate {
        #[serde(skip_serializing_if = "Option::is_none")]
        fab: Option<FabOverride>,
    },

    /// Image browser to display
    ImageBrowser { browser: ImageBrowserData },

    /// Grid browser to display
    GridBrowser { browser: GridBrowserData },

    /// Plugin actions toolbar updated
    PluginActionsUpdate { actions: Vec<PluginAction> },

    /// Navigation depth changed (for breadcrumb display)
    NavigationDepthChanged { depth: u32 },

    /// Navigate forward - increment depth by 1 (relative change)
    NavigateForward,

    /// Navigate back - decrement depth by 1 (relative change)
    NavigateBack,

    /// Configuration reloaded at runtime
    ConfigReloaded,

    /// Plugin management mode changed (for "/" prefix - shows only plugins)
    PluginManagementChanged { active: bool },

    /// Internal: Index update from plugin (not forwarded to UI)
    /// Used by non-socket daemon plugins that emit index data via stdio
    #[serde(skip)]
    IndexUpdate {
        plugin_id: String,
        /// Serialized `IndexItem` array (we use `Value` to avoid circular deps)
        items: serde_json::Value,
        mode: Option<String>,
        remove: Option<Vec<String>>,
    },
}

/// Partial update to a result item - only specified fields are updated
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ResultPatch {
    /// ID of the item to update (required)
    pub id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_type: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub verb: Option<String>,

    /// **Deprecated**: Use `widget` field instead
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<SliderValue>,

    /// **Deprecated**: Use `widget` field instead
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gauge: Option<GaugeData>,

    /// **Deprecated**: Use `widget` field instead
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<ProgressData>,

    /// **Deprecated**: Use `widget` field instead
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph: Option<GraphData>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub badges: Option<Vec<Badge>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub chips: Option<Vec<Chip>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_ocr: Option<bool>,

    /// Strongly typed widget data for patches
    #[serde(skip_serializing_if = "Option::is_none")]
    pub widget: Option<WidgetData>,
}

/// Unified result item used throughout hamr.
///
/// This is the single source of truth for result items across:
/// - Plugin search responses
/// - Plugin index items  
/// - UI display
/// - RPC transport
///
/// All fields use serde defaults for flexible deserialization from plugins.
/// Deserialization is handled via `ResultItemRaw` to support both modern `widget`
/// field and legacy flat fields (value/min/max/step/gauge/progress/graph).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", try_from = "ResultItemRaw")]
pub struct ResultItem {
    pub id: String,
    pub name: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_type: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verb: Option<String>,

    /// Result/item type - accepts "type", "resultType", or "itemType" from JSON
    #[serde(default, alias = "type", alias = "itemType")]
    pub result_type: ResultType,

    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        deserialize_with = "deserialize_null_as_empty_vec"
    )]
    pub badges: Vec<Badge>,

    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        deserialize_with = "deserialize_null_as_empty_vec"
    )]
    pub chips: Vec<Chip>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview: Option<PreviewData>,

    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        deserialize_with = "deserialize_null_as_empty_vec"
    )]
    pub actions: Vec<Action>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin_id: Option<String>,

    /// Primary app ID for window matching - `StartupWMClass` from `.desktop` file
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app_id: Option<String>,

    /// Fallback app ID - desktop filename without .desktop extension
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app_id_fallback: Option<String>,

    /// Keywords for enhanced searchability (index items only)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<String>>,

    /// Entry point data for replaying actions (index items only)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry_point: Option<serde_json::Value>,

    /// Whether selecting this item should keep the launcher open
    #[serde(default, skip_serializing_if = "is_false")]
    pub keep_open: bool,

    /// Whether this item is a smart suggestion based on context
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_suggestion: bool,

    /// Reason why this item was suggested (e.g., "Often used around 9am")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggestion_reason: Option<String>,

    /// Whether this item has OCR-searchable text (for images/screenshots)
    #[serde(default, skip_serializing_if = "is_false")]
    pub has_ocr: bool,

    /// Display hint for preferred view mode (list vs grid)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_hint: Option<DisplayHint>,

    /// Strongly typed widget data (slider, switch, gauge, progress, graph)
    /// Populated during deserialization from flat fields for backward compatibility.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub widget: Option<WidgetData>,

    /// Immediate URL to open when this result is selected (for match responses)
    #[serde(default, rename = "openUrl", skip_serializing_if = "Option::is_none")]
    pub open_url: Option<String>,

    /// Immediate text to copy when this result is selected (for match responses)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy: Option<String>,

    /// Notification message to show when this result is selected (for match responses)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notify: Option<String>,

    /// Whether to close the launcher after executing this result (for match responses)
    #[serde(default, rename = "close", skip_serializing_if = "Option::is_none")]
    pub should_close: Option<bool>,

    /// Composite score for ranking (not serialized over RPC)
    #[serde(skip)]
    pub composite_score: f64,
}

// Serde skip_serializing_if requires &bool signature
#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_false(b: &bool) -> bool {
    !*b
}

impl Default for ResultItem {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            description: None,
            icon: None,
            icon_type: None,
            verb: None,
            result_type: ResultType::default(),
            badges: Vec::new(),
            chips: Vec::new(),
            thumbnail: None,
            preview: None,
            actions: Vec::new(),
            plugin_id: None,
            app_id: None,
            app_id_fallback: None,
            keywords: None,
            entry_point: None,
            keep_open: false,
            is_suggestion: false,
            suggestion_reason: None,
            has_ocr: false,
            display_hint: None,
            widget: None,
            open_url: None,
            copy: None,
            notify: None,
            should_close: None,
            composite_score: 0.0,
        }
    }
}

/// Build `WidgetData` from flat fields during deserialization.
/// Returns `Some(widget)` if any widget data is present.
#[allow(clippy::too_many_arguments)] // Flat fields from legacy protocol
fn build_widget_from_flat(
    result_type: ResultType,
    value: Option<f64>,
    min: Option<f64>,
    max: Option<f64>,
    step: Option<f64>,
    display_value: Option<String>,
    gauge: Option<&GaugeData>,
    progress: Option<&ProgressData>,
    graph: Option<&GraphData>,
) -> Option<WidgetData> {
    // Priority: explicit widget types first, then infer from data
    match result_type {
        ResultType::Slider => value.map(|v| WidgetData::Slider {
            value: v,
            min: min.unwrap_or(0.0),
            max: max.unwrap_or(100.0),
            step: step.unwrap_or(1.0),
            display_value,
        }),
        ResultType::Switch => value.map(|v| WidgetData::Switch { value: v != 0.0 }),
        _ => {
            // Check for other widget types
            if let Some(g) = gauge {
                return Some(WidgetData::Gauge {
                    value: g.value,
                    min: g.min,
                    max: g.max,
                    label: g.label.clone(),
                    color: g.color.clone(),
                });
            }
            if let Some(p) = progress {
                return Some(WidgetData::Progress {
                    value: p.value,
                    max: p.max,
                    label: p.label.clone(),
                    color: p.color.clone(),
                });
            }
            if let Some(gr) = graph {
                return Some(WidgetData::Graph {
                    data: gr.data.clone(),
                    min: gr.min,
                    max: gr.max,
                });
            }
            None
        }
    }
}

/// Internal helper for deserializing slider value which can be a number or object.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SliderValueInternal {
    value: Option<f64>,
    #[serde(default)]
    min: Option<f64>,
    #[serde(default)]
    max: Option<f64>,
    #[serde(default)]
    step: Option<f64>,
    #[serde(default)]
    display_value: Option<String>,
}

/// Raw deserialization target for `ResultItem`.
/// Handles both modern `widget` field and legacy flat fields for backward compatibility.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResultItemRaw {
    id: String,
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    icon: Option<String>,
    #[serde(default)]
    icon_type: Option<String>,
    #[serde(default)]
    verb: Option<String>,
    #[serde(default, alias = "type", alias = "itemType")]
    result_type: Option<ResultType>,
    #[serde(default, deserialize_with = "deserialize_null_as_empty_vec")]
    badges: Vec<Badge>,
    #[serde(default, deserialize_with = "deserialize_null_as_empty_vec")]
    chips: Vec<Chip>,
    #[serde(default)]
    value: serde_json::Value,
    #[serde(default)]
    min: Option<f64>,
    #[serde(default)]
    max: Option<f64>,
    #[serde(default)]
    step: Option<f64>,
    #[serde(default)]
    display_value: Option<String>,
    #[serde(default)]
    gauge: Option<GaugeData>,
    #[serde(default, deserialize_with = "deserialize_progress_data")]
    progress: Option<ProgressData>,
    #[serde(default)]
    graph: Option<GraphData>,
    #[serde(default)]
    thumbnail: Option<String>,
    #[serde(default)]
    preview: Option<PreviewData>,
    #[serde(default, deserialize_with = "deserialize_null_as_empty_vec")]
    actions: Vec<Action>,
    #[serde(default)]
    plugin_id: Option<String>,
    #[serde(default)]
    app_id: Option<String>,
    #[serde(default)]
    app_id_fallback: Option<String>,
    #[serde(default)]
    keywords: Option<Vec<String>>,
    #[serde(default)]
    entry_point: Option<serde_json::Value>,
    #[serde(default)]
    keep_open: bool,
    #[serde(default)]
    is_suggestion: bool,
    #[serde(default)]
    suggestion_reason: Option<String>,
    #[serde(default)]
    has_ocr: bool,
    #[serde(default)]
    display_hint: Option<DisplayHint>,
    #[serde(default)]
    widget: Option<WidgetData>,
    #[serde(default, rename = "openUrl")]
    open_url: Option<String>,
    #[serde(default)]
    copy: Option<String>,
    #[serde(default)]
    notify: Option<String>,
    #[serde(default, rename = "close")]
    should_close: Option<bool>,
}

/// Parsed slider/switch value data tuple.
type ParsedValueData = (
    Option<f64>,
    Option<f64>,
    Option<f64>,
    Option<f64>,
    Option<String>,
);

/// Parse the `value` field which can be a number, boolean, or `SliderValue` object.
fn parse_value_field(
    value: serde_json::Value,
    result_type: Option<ResultType>,
    min: Option<f64>,
    max: Option<f64>,
    step: Option<f64>,
    display_value: Option<String>,
) -> Result<ParsedValueData, String> {
    if value.is_object() {
        let slider_val: SliderValueInternal =
            serde_json::from_value(value).unwrap_or(SliderValueInternal {
                value: None,
                min: None,
                max: None,
                step: None,
                display_value: None,
            });
        Ok((
            slider_val.value,
            slider_val.min.or(min),
            slider_val.max.or(max),
            slider_val.step.or(step),
            slider_val.display_value.or(display_value),
        ))
    } else if value.is_number() {
        let num = value
            .as_f64()
            .ok_or_else(|| "expected f64 for value".to_string())?;
        Ok((Some(num), min, max, step, display_value))
    } else if value.is_boolean() {
        if result_type != Some(ResultType::Switch) {
            return Err("boolean value is only valid for switch type, not slider".to_string());
        }
        let num = if value.as_bool().unwrap_or(false) {
            1.0
        } else {
            0.0
        };
        Ok((Some(num), min, max, step, display_value))
    } else if value.is_null() {
        Ok((None, min, max, step, display_value))
    } else {
        Err("value must be a number, boolean, or SliderValue object".to_string())
    }
}

impl TryFrom<ResultItemRaw> for ResultItem {
    type Error = String;

    fn try_from(raw: ResultItemRaw) -> Result<Self, Self::Error> {
        let (value, min, max, step, display_value) = parse_value_field(
            raw.value,
            raw.result_type,
            raw.min,
            raw.max,
            raw.step,
            raw.display_value,
        )?;

        let result_type = raw.result_type.unwrap_or_default();

        let widget = raw.widget.or_else(|| {
            build_widget_from_flat(
                result_type,
                value,
                min,
                max,
                step,
                display_value,
                raw.gauge.as_ref(),
                raw.progress.as_ref(),
                raw.graph.as_ref(),
            )
        });

        Ok(ResultItem {
            id: raw.id,
            name: raw.name,
            description: raw.description,
            icon: raw.icon,
            icon_type: raw.icon_type,
            verb: raw.verb,
            result_type,
            badges: raw.badges,
            chips: raw.chips,
            thumbnail: raw.thumbnail,
            preview: raw.preview,
            actions: raw.actions,
            plugin_id: raw.plugin_id,
            app_id: raw.app_id,
            app_id_fallback: raw.app_id_fallback,
            keywords: raw.keywords,
            entry_point: raw.entry_point,
            keep_open: raw.keep_open,
            is_suggestion: raw.is_suggestion,
            suggestion_reason: raw.suggestion_reason,
            has_ocr: raw.has_ocr,
            display_hint: raw.display_hint,
            widget,
            open_url: raw.open_url,
            copy: raw.copy,
            notify: raw.notify,
            should_close: raw.should_close,
            composite_score: 0.0,
        })
    }
}

impl ResultItem {
    /// Get icon with default fallback
    #[must_use]
    pub fn icon_or_default(&self) -> &str {
        self.icon.as_deref().unwrap_or("extension")
    }

    /// Get verb with default fallback
    #[must_use]
    pub fn verb_or_default(&self) -> &str {
        self.verb.as_deref().unwrap_or("Select")
    }

    /// Get slider value as a `SliderValue` struct (for UI components)
    #[must_use]
    pub fn slider_value(&self) -> Option<SliderValue> {
        match &self.widget {
            Some(WidgetData::Slider {
                value,
                min,
                max,
                step,
                display_value,
            }) => Some(SliderValue {
                value: *value,
                min: *min,
                max: *max,
                step: *step,
                display_value: display_value.clone(),
            }),
            _ => None,
        }
    }

    /// Check if this is a slider type (derived from widget field)
    #[must_use]
    pub fn is_slider(&self) -> bool {
        matches!(self.widget, Some(WidgetData::Slider { .. }))
    }

    /// Check if this is a switch type (derived from widget field)
    #[must_use]
    pub fn is_switch(&self) -> bool {
        matches!(self.widget, Some(WidgetData::Switch { .. }))
    }

    /// Set slider widget data
    #[must_use]
    pub fn with_slider(
        mut self,
        value: f64,
        min: f64,
        max: f64,
        step: f64,
        display_value: Option<String>,
    ) -> Self {
        self.widget = Some(WidgetData::Slider {
            value,
            min,
            max,
            step,
            display_value,
        });
        self
    }

    /// Set switch widget data
    #[must_use]
    pub fn with_switch(mut self, value: bool) -> Self {
        self.widget = Some(WidgetData::Switch { value });
        self
    }

    /// Set gauge widget data
    #[must_use]
    pub fn with_gauge(mut self, data: GaugeData) -> Self {
        self.widget = Some(WidgetData::Gauge {
            value: data.value,
            min: data.min,
            max: data.max,
            label: data.label,
            color: data.color,
        });
        self
    }

    /// Set progress widget data
    #[must_use]
    pub fn with_progress(mut self, data: ProgressData) -> Self {
        self.widget = Some(WidgetData::Progress {
            value: data.value,
            max: data.max,
            label: data.label,
            color: data.color,
        });
        self
    }

    /// Set graph widget data
    #[must_use]
    pub fn with_graph(mut self, data: GraphData) -> Self {
        self.widget = Some(WidgetData::Graph {
            data: data.data,
            min: data.min,
            max: data.max,
        });
        self
    }
}

/// Type alias for backward compatibility
pub type SearchResult = ResultItem;

/// Icon specification (internal use only)
///
/// For wire format, use `SearchResult.icon` (`String`) + `SearchResult.icon_type` (`Option<String>`).
/// This enum is used internally by TUI and other UIs to convert from the simple protocol format.
#[derive(Debug, Clone)]
pub enum IconSpec {
    /// Freedesktop/system icon name
    System(String),

    /// Material symbol name
    Material(String),

    /// Text/emoji
    Text(String),

    /// File path
    Path(PathBuf),
}

impl IconSpec {
    /// Create `IconSpec` from the simple wire format used in protocol.
    /// Auto-detects type if not specified based on icon name patterns.
    #[must_use]
    pub fn from_wire(icon: String, icon_type: Option<&str>) -> Self {
        match icon_type {
            Some("system") => IconSpec::System(icon),
            Some("material") => IconSpec::Material(icon),
            Some("text") => IconSpec::Text(icon),
            Some("path") => IconSpec::Path(PathBuf::from(icon)),
            None => {
                if icon.contains('.') || icon.contains('-') {
                    IconSpec::System(icon)
                } else {
                    IconSpec::Material(icon)
                }
            }
            Some(_other) => IconSpec::Material(icon),
        }
    }
}

impl Serialize for IconSpec {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;

        let mut map = serializer.serialize_map(Some(2))?;

        match self {
            IconSpec::System(s) => {
                map.serialize_entry("type", "system")?;
                map.serialize_entry("value", s)?;
            }
            IconSpec::Material(s) => {
                map.serialize_entry("type", "material")?;
                map.serialize_entry("value", s)?;
            }
            IconSpec::Text(s) => {
                map.serialize_entry("type", "text")?;
                map.serialize_entry("value", s)?;
            }
            IconSpec::Path(p) => {
                map.serialize_entry("type", "path")?;
                map.serialize_entry("value", p)?;
            }
        }

        map.end()
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum IconSpecRaw {
    PlainString(String),
    Tagged {
        #[serde(rename = "type")]
        icon_type: String,
        value: serde_json::Value,
    },
}

impl<'de> Deserialize<'de> for IconSpec {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        IconSpecRaw::deserialize(deserializer)
            .and_then(|raw| raw.try_into().map_err(serde::de::Error::custom))
    }
}

impl TryFrom<IconSpecRaw> for IconSpec {
    type Error = String;

    fn try_from(raw: IconSpecRaw) -> Result<Self, Self::Error> {
        match raw {
            IconSpecRaw::PlainString(s) => Ok(IconSpec::Material(s)),
            IconSpecRaw::Tagged { icon_type, value } => match icon_type.as_str() {
                "system" => {
                    let s = serde_json::from_value::<String>(value)
                        .map_err(|e| format!("Invalid system icon value: {e}"))?;
                    Ok(IconSpec::System(s))
                }
                "material" => {
                    let s = serde_json::from_value::<String>(value)
                        .map_err(|e| format!("Invalid material icon value: {e}"))?;
                    Ok(IconSpec::Material(s))
                }
                "text" => {
                    let s = serde_json::from_value::<String>(value)
                        .map_err(|e| format!("Invalid text icon value: {e}"))?;
                    Ok(IconSpec::Text(s))
                }
                "path" => {
                    let p = serde_json::from_value::<PathBuf>(value)
                        .map_err(|e| format!("Invalid path icon value: {e}"))?;
                    Ok(IconSpec::Path(p))
                }
                other => Err(format!("Unknown icon type: {other}")),
            },
        }
    }
}

impl Default for IconSpec {
    fn default() -> Self {
        Self::Material("extension".to_string())
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResultType {
    #[default]
    Normal,
    App,
    Plugin,
    IndexedItem,
    Slider,
    Switch,
    WebSearch,
    Suggestion,
    Recent,
    PatternMatch,
}

/// Display hint for result rendering
///
/// Plugins can provide a hint about the preferred view mode for their results.
/// This allows plugins like emoji or wallpaper to request grid view while
/// users can still toggle with Ctrl+G.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DisplayHint {
    /// Use user's default preference
    #[default]
    Auto,
    /// Force list view
    List,
    /// Force grid view
    Grid,
    /// Force large grid view (image browser style)
    LargeGrid,
}

/// Badge on a result
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Badge {
    #[serde(default)]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

/// Chip (similar to badge but different styling)
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Chip {
    /// Text to display (accepts both "text" and "label" from JSON)
    #[serde(alias = "label", default)]
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Custom text/icon color (CSS color string like "#ffffff")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SliderValue {
    pub value: f64,
    #[serde(default)]
    pub min: f64,
    #[serde(default = "default_max")]
    pub max: f64,
    #[serde(default = "default_step")]
    pub step: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_value: Option<String>,
}

fn default_max() -> f64 {
    100.0
}
fn default_step() -> f64 {
    1.0
}

/// Interactive and display widgets for result items.
///
/// This tagged enum replaces the flat `value`/`min`/`max`/`step` fields with
/// explicit widget types. Each variant contains only the fields relevant to
/// that widget type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum WidgetData {
    /// Adjustable slider (volume, brightness)
    Slider {
        value: f64,
        #[serde(default)]
        min: f64,
        #[serde(default = "default_max")]
        max: f64,
        #[serde(default = "default_step")]
        step: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        display_value: Option<String>,
    },
    /// Toggle switch (on/off)
    Switch { value: bool },
    /// Circular gauge display (memory, battery, temperature)
    Gauge {
        value: f64,
        #[serde(default)]
        min: f64,
        #[serde(default = "default_max")]
        max: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        color: Option<String>,
    },
    /// Linear progress bar (download, sync, playback)
    Progress {
        value: f64,
        #[serde(default = "default_max")]
        max: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        color: Option<String>,
    },
    /// Sparkline graph (CPU, network, stock price)
    Graph {
        data: Vec<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        min: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max: Option<f64>,
    },
}

impl WidgetData {
    /// Get the current value for interactive widgets (slider/switch)
    #[must_use]
    pub fn value(&self) -> Option<f64> {
        match self {
            WidgetData::Slider { value, .. }
            | WidgetData::Gauge { value, .. }
            | WidgetData::Progress { value, .. } => Some(*value),
            WidgetData::Switch { value } => Some(if *value { 1.0 } else { 0.0 }),
            WidgetData::Graph { .. } => None,
        }
    }

    /// Check if this is an interactive widget (can be modified by user)
    #[must_use]
    pub fn is_interactive(&self) -> bool {
        matches!(self, WidgetData::Slider { .. } | WidgetData::Switch { .. })
    }
}

/// Unified frecency data for index entries.
///
/// This struct consolidates all frecency-related fields that were previously
/// stored as flat underscore-prefixed fields (`_count`, `_lastUsed`, etc.)
/// into a single nested structure.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Frecency {
    /// Total usage count
    #[serde(default)]
    pub count: u32,

    /// Last used timestamp in milliseconds since epoch
    #[serde(default)]
    pub last_used: u64,

    /// Recent search terms that led to this item (max 10)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recent_search_terms: Vec<String>,

    /// Usage count per hour of day (0-23)
    #[serde(default, skip_serializing_if = "is_zero_array_24")]
    pub hour_slot_counts: [u32; 24],

    /// Usage count per day of week (0=Sunday, 6=Saturday)
    #[serde(default, skip_serializing_if = "is_zero_array_7")]
    pub day_of_week_counts: [u32; 7],

    /// Consecutive days of usage (streak)
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    pub consecutive_days: u32,

    /// Last date of consecutive usage (YYYY-MM-DD format)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_consecutive_date: Option<String>,

    /// Count of launches from empty query
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    pub launch_from_empty_count: u32,

    /// Count of launches at session start
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    pub session_start_count: u32,

    /// Usage count per workspace
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub workspace_counts: HashMap<String, u32>,

    /// Usage count per monitor
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub monitor_counts: HashMap<String, u32>,

    /// Items launched after this item (sequence tracking)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub launched_after: HashMap<String, u32>,

    /// Count of launches after resuming from idle
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    pub resume_from_idle_count: u32,

    /// Usage count per display configuration
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub display_count_counts: HashMap<String, u32>,

    /// Usage count per session duration bucket
    /// [0]: 0-5min, [1]: 5-15min, [2]: 15-30min, [3]: 30-60min, [4]: 60+min
    #[serde(default, skip_serializing_if = "is_zero_array_5")]
    pub session_duration_counts: [u32; 5],
}

// Serde skip_serializing_if requires &u32 signature
#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_zero_u32(v: &u32) -> bool {
    *v == 0
}

fn is_zero_array_24(arr: &[u32; 24]) -> bool {
    arr.iter().all(|&x| x == 0)
}

fn is_zero_array_7(arr: &[u32; 7]) -> bool {
    arr.iter().all(|&x| x == 0)
}

fn is_zero_array_5(arr: &[u32; 5]) -> bool {
    arr.iter().all(|&x| x == 0)
}

impl Frecency {
    /// Create a new Frecency with initial usage
    #[must_use]
    pub fn new_with_usage(count: u32, last_used: u64) -> Self {
        Self {
            count,
            last_used,
            ..Default::default()
        }
    }

    /// Check if this item has any usage data
    #[must_use]
    pub fn has_usage(&self) -> bool {
        self.count > 0
    }

    /// Get the age in milliseconds since last use
    #[must_use]
    pub fn age_ms(&self, now: u64) -> u64 {
        now.saturating_sub(self.last_used)
    }
}

/// Gauge data for circular progress
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GaugeData {
    pub value: f64,
    #[serde(default)]
    pub min: f64,
    #[serde(default = "default_max")]
    pub max: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

/// Progress bar data
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProgressData {
    pub value: f64,
    #[serde(default = "default_max")]
    pub max: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

/// Custom deserializer for progress that accepts either a number or `ProgressData` object
fn deserialize_progress_data<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<ProgressData>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;

    let value: Option<serde_json::Value> = Option::deserialize(deserializer)?;
    match value {
        None => Ok(None),
        Some(serde_json::Value::Number(n)) => Ok(Some(ProgressData {
            value: n.as_f64().unwrap_or(0.0),
            max: 100.0,
            label: None,
            color: None,
        })),
        Some(serde_json::Value::Object(obj)) => {
            let progress: ProgressData = serde_json::from_value(serde_json::Value::Object(obj))
                .map_err(|e| D::Error::custom(format!("failed to parse progress object: {e}")))?;
            Ok(Some(progress))
        }
        Some(other) => Err(D::Error::custom(format!(
            "expected number or object for progress, got {other:?}"
        ))),
    }
}

/// Preview data for side panel
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreviewData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markdown: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub metadata: Vec<MetadataItem>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<Action>,
}

/// Metadata key-value pair
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetadataItem {
    pub label: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

/// Action on a result
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Action {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_type: Option<String>,
    #[serde(default)]
    pub keep_open: bool,
}

/// Plugin action for toolbar (Ctrl+1-6 shortcuts)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginAction {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Keyboard shortcut hint (e.g., "Ctrl+1")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shortcut: Option<String>,
    /// Confirmation message for dangerous actions
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confirm: Option<String>,
    /// Whether this action is currently active/highlighted
    #[serde(default)]
    pub active: bool,
}

/// Card data for display
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardData {
    #[serde(default)]
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markdown: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<Action>,
    /// Card kind - "blocks" for rich block-based cards
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// Block content for rich cards
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<CardBlock>,
    /// Maximum height for the card content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_height: Option<u32>,
    /// Whether to show details section expanded
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_details: Option<bool>,
    /// Allow toggling details visibility
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_toggle_details: Option<bool>,
}

/// Form data for input
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormData {
    pub title: String,
    pub fields: Vec<FormField>,
    #[serde(default = "default_submit_label")]
    pub submit_label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancel_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    /// When true, changes are applied immediately without submit button
    #[serde(default)]
    pub live_update: bool,
}

fn default_submit_label() -> String {
    "Submit".to_string()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormField {
    pub id: String,
    pub label: String,
    #[serde(default, rename = "type")]
    pub field_type: FormFieldType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_value: Option<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<FormOption>,
    /// Help text displayed below the field
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    /// Number of rows for textarea fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rows: Option<u32>,
    /// Minimum value for slider fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    /// Maximum value for slider fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    /// Step value for slider fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FormFieldType {
    #[default]
    Text,
    Password,
    Number,
    TextArea,
    Select,
    Checkbox,
    Switch,
    Slider,
    Hidden,
    Date,
    Time,
    Email,
    Url,
    Phone,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormOption {
    pub value: String,
    pub label: String,
}

/// Execute action from core
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ExecuteAction {
    /// Launch a desktop application (via gio launch on Linux, open -a on macOS)
    #[serde(rename = "launch")]
    Launch { desktop_file: String },

    /// Open a URL in default browser
    #[serde(rename = "open_url")]
    OpenUrl { url: String },

    /// Open a file/folder with default application
    #[serde(rename = "open")]
    Open { path: String },

    /// Copy text to clipboard
    #[serde(rename = "copy")]
    Copy { text: String },

    /// Type text (simulate keyboard)
    #[serde(rename = "type_text")]
    TypeText { text: String },

    /// Play a sound effect
    #[serde(rename = "sound")]
    PlaySound { sound: String },

    /// Show notification
    #[serde(rename = "notify")]
    Notify { message: String },
}

/// Ambient item - persistent status items shown in action bar
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AmbientItem {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        deserialize_with = "deserialize_null_as_empty_vec"
    )]
    pub badges: Vec<Badge>,
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        deserialize_with = "deserialize_null_as_empty_vec"
    )]
    pub chips: Vec<Chip>,
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        deserialize_with = "deserialize_null_as_empty_vec"
    )]
    pub actions: Vec<Action>,
    /// Duration in ms before auto-removal (0 = permanent)
    #[serde(default)]
    pub duration: u64,
    /// Plugin that owns this ambient item
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin_id: Option<String>,
}

/// FAB (Floating Action Button) override data
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FabOverride {
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        deserialize_with = "deserialize_null_as_empty_vec"
    )]
    pub badges: Vec<Badge>,
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        deserialize_with = "deserialize_null_as_empty_vec"
    )]
    pub chips: Vec<Chip>,
    /// Higher priority wins when multiple plugins set FAB
    #[serde(default)]
    pub priority: i32,
    /// Force FAB visible when launcher is closed
    #[serde(default)]
    pub show_fab: bool,
}

/// Plugin status update - badges/chips/description for plugin entry in main list
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginStatus {
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        deserialize_with = "deserialize_null_as_empty_vec"
    )]
    pub badges: Vec<Badge>,
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        deserialize_with = "deserialize_null_as_empty_vec"
    )]
    pub chips: Vec<Chip>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fab: Option<FabOverride>,
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        deserialize_with = "deserialize_null_as_empty_vec"
    )]
    pub ambient: Vec<AmbientItem>,
}

/// Source of an action (where it was triggered from)
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActionSource {
    #[default]
    Normal,
    Ambient,
    Fab,
}

/// Graph/sparkline data for visual display
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphData {
    /// Y-axis data points
    pub data: Vec<f64>,
    /// Minimum Y value (auto-calculated if not specified)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    /// Maximum Y value (auto-calculated if not specified)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
}

/// Image browser data for displaying image grids
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageBrowserData {
    /// Directory path being browsed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub directory: Option<String>,
    /// List of images to display
    #[serde(default)]
    pub images: Vec<ImageItem>,
    /// Optional title for the browser
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// Single image item in image browser
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageItem {
    /// File path to the image
    pub path: String,
    /// Optional unique identifier
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Optional display name
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Grid browser data for displaying item grids
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GridBrowserData {
    /// Items to display in the grid
    pub items: Vec<GridItem>,
    /// Optional title for the browser
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Number of columns (default varies by UI)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub columns: Option<u32>,
    /// Custom actions available in the grid
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<Action>,
}

/// Single item in grid browser
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GridItem {
    /// Unique identifier
    pub id: String,
    /// Display name
    pub name: String,
    /// Optional icon name
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Optional thumbnail path
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<String>,
    /// Optional description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Block type for rich cards
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "block_type", rename_all = "lowercase")]
pub enum CardBlock {
    /// Pill/badge separator (e.g., date markers)
    Pill { text: String },
    /// Horizontal line separator
    Separator,
    /// Chat message bubble
    Message {
        /// Role: "user", "assistant", "system"
        role: String,
        /// Message content (supports markdown)
        content: String,
    },
    /// Note/info block
    Note { content: String },
}

/// Plugin manifest for registration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    #[serde(default)]
    pub priority: i32,
}

#[cfg(test)]
#[allow(clippy::float_cmp)] // Exact float comparisons are intentional in tests
mod icon_spec_tests {
    use super::{
        CoreEvent, CoreUpdate, IconSpec, ResultItem, ResultType, SearchResult, WidgetData,
    };
    use serde_json::json;

    #[test]
    fn auto_detect_material_icon() {
        let icon = IconSpec::from_wire("timer".to_string(), None);
        match icon {
            IconSpec::Material(name) => assert_eq!(name, "timer"),
            _ => panic!("Expected Material variant"),
        }
    }

    #[test]
    fn auto_detect_system_icon_with_dots() {
        let icon = IconSpec::from_wire("org.gnome.Nautilus".to_string(), None);
        match icon {
            IconSpec::System(name) => assert_eq!(name, "org.gnome.Nautilus"),
            _ => panic!("Expected System variant"),
        }
    }

    #[test]
    fn auto_detect_system_icon_with_dashes() {
        let icon = IconSpec::from_wire("document-open".to_string(), None);
        match icon {
            IconSpec::System(name) => assert_eq!(name, "document-open"),
            _ => panic!("Expected System variant"),
        }
    }

    #[test]
    fn explicit_system_type() {
        let icon = IconSpec::from_wire("firefox".to_string(), Some("system"));
        match icon {
            IconSpec::System(name) => assert_eq!(name, "firefox"),
            _ => panic!("Expected System variant"),
        }
    }

    #[test]
    fn explicit_material_type() {
        let icon = IconSpec::from_wire("org.gnome.Nautilus".to_string(), Some("material"));
        match icon {
            IconSpec::Material(name) => assert_eq!(name, "org.gnome.Nautilus"),
            _ => panic!("Expected Material variant"),
        }
    }

    #[test]
    fn explicit_text_type() {
        let icon = IconSpec::from_wire("🚀".to_string(), Some("text"));
        match icon {
            IconSpec::Text(value) => assert_eq!(value, "🚀"),
            _ => panic!("Expected Text variant"),
        }
    }

    #[test]
    fn search_result_with_icon() {
        let json = json!({
            "id": "test",
            "name": "Test Item",
            "icon": "timer"
        });
        let result: SearchResult = serde_json::from_value(json).expect("Failed to deserialize");
        assert_eq!(result.icon_or_default(), "timer");
        assert_eq!(result.icon_type, None);
    }

    #[test]
    fn search_result_with_icon_type() {
        let json = json!({
            "id": "test",
            "name": "Test Item",
            "icon": "org.gnome.Nautilus",
            "iconType": "system"
        });
        let result: SearchResult = serde_json::from_value(json).expect("Failed to deserialize");
        assert_eq!(result.icon_or_default(), "org.gnome.Nautilus");
        assert_eq!(result.icon_type, Some("system".to_string()));
    }

    #[test]
    fn search_result_serialize_simple() {
        let result = SearchResult {
            id: "test".to_string(),
            name: "Test".to_string(),
            icon: Some("timer".to_string()),
            icon_type: None,
            ..Default::default()
        };
        let json = serde_json::to_value(&result).expect("Failed to serialize");
        assert_eq!(json["icon"], "timer");
        assert!(json.get("iconType").is_none());
    }

    #[test]
    fn search_result_serialize_with_type() {
        let result = SearchResult {
            id: "test".to_string(),
            name: "Test".to_string(),
            icon: Some("firefox".to_string()),
            icon_type: Some("system".to_string()),
            ..Default::default()
        };
        let json = serde_json::to_value(&result).expect("Failed to serialize");
        assert_eq!(json["icon"], "firefox");
        assert_eq!(json["iconType"], "system");
    }

    #[test]
    fn default_icon_is_extension() {
        let result = SearchResult::default();
        assert_eq!(result.icon_or_default(), "extension");
        assert_eq!(result.icon_type, None);
    }

    #[test]
    fn qml_protocol_format_simple() {
        let json = json!({
            "id": "timer_id",
            "name": "Timer",
            "icon": "timer"
        });
        let result: SearchResult = serde_json::from_value(json).expect("Failed to deserialize");
        assert_eq!(result.icon_or_default(), "timer");
        assert_eq!(result.icon_type, None);
    }

    #[test]
    fn qml_protocol_format_with_type() {
        let json = json!({
            "id": "nautilus_id",
            "name": "Files",
            "icon": "org.gnome.Nautilus",
            "iconType": "system"
        });
        let result: SearchResult = serde_json::from_value(json).expect("Failed to deserialize");
        assert_eq!(result.icon_or_default(), "org.gnome.Nautilus");
        assert_eq!(result.icon_type, Some("system".to_string()));
    }

    #[test]
    fn auto_detect_logic_matches_qml() {
        let system_icons = vec![
            "org.gnome.Nautilus",
            "com.google.Chrome",
            "document-open",
            "folder-open",
        ];
        for icon_name in system_icons {
            let spec = IconSpec::from_wire(icon_name.to_string(), None);
            match spec {
                IconSpec::System(name) => assert_eq!(name, icon_name),
                _ => panic!("Expected System for {icon_name}, got {spec:?}"),
            }
        }

        let material_icons = vec!["timer", "settings", "play", "pause"];
        for icon_name in material_icons {
            let spec = IconSpec::from_wire(icon_name.to_string(), None);
            match spec {
                IconSpec::Material(name) => assert_eq!(name, icon_name),
                _ => panic!("Expected Material for {icon_name}, got {spec:?}"),
            }
        }
    }

    #[test]
    fn test_set_context_event_serialization() {
        let event = CoreEvent::SetContext {
            context: Some("__edit__:0".to_string()),
        };

        let json = serde_json::to_value(&event).expect("Failed to serialize");
        assert_eq!(json["type"], "set_context");
        assert_eq!(json["context"], "__edit__:0");
    }

    #[test]
    fn test_set_context_event_deserialization() {
        let json = json!({
            "type": "set_context",
            "context": "__edit__:5"
        });

        let event: CoreEvent = serde_json::from_value(json).expect("Failed to deserialize");
        match event {
            CoreEvent::SetContext { context } => {
                assert_eq!(context, Some("__edit__:5".to_string()));
            }
            _ => panic!("Expected SetContext event"),
        }
    }

    #[test]
    fn test_set_context_event_null_context() {
        let event = CoreEvent::SetContext { context: None };

        let json = serde_json::to_value(&event).expect("Failed to serialize");
        assert_eq!(json["type"], "set_context");
        assert!(json.get("context").is_none() || json["context"].is_null());
    }

    #[test]
    fn test_results_update_with_all_optional_fields() {
        let update = CoreUpdate::Results {
            results: vec![SearchResult {
                id: "item-1".to_string(),
                name: "Test Item".to_string(),
                ..Default::default()
            }],
            placeholder: Some("Edit task...".to_string()),
            clear_input: Some(true),
            input_mode: Some("submit".to_string()),
            context: Some("__edit__:0".to_string()),
            navigate_forward: Some(true),
            display_hint: None,
        };

        let json = serde_json::to_value(&update).expect("Failed to serialize");
        assert_eq!(json["type"], "results");
        assert!(json["results"].is_array());
        assert_eq!(json["placeholder"], "Edit task...");
        assert_eq!(json["clearInput"], true);
        assert_eq!(json["inputMode"], "submit");
        assert_eq!(json["context"], "__edit__:0");
        assert_eq!(json["navigateForward"], true);
    }

    #[test]
    fn test_results_update_without_optional_fields() {
        let update = CoreUpdate::Results {
            results: vec![],
            placeholder: None,
            clear_input: None,
            input_mode: None,
            context: None,
            navigate_forward: None,
            display_hint: None,
        };

        let json = serde_json::to_value(&update).expect("Failed to serialize");
        assert_eq!(json["type"], "results");
        assert!(json["results"].is_array());
        // Optional fields should be absent (not null)
        assert!(json.get("placeholder").is_none());
        assert!(json.get("clearInput").is_none());
        assert!(json.get("inputMode").is_none());
        assert!(json.get("context").is_none());
        assert!(json.get("navigateForward").is_none());
    }

    #[test]
    fn test_results_update_deserialization_with_optional_fields() {
        let json = json!({
            "type": "results",
            "results": [{"id": "item-1", "name": "Item 1"}],
            "placeholder": "Search...",
            "clearInput": true,
            "inputMode": "realtime",
            "context": "__add_mode__"
        });

        let update: CoreUpdate = serde_json::from_value(json).expect("Failed to deserialize");
        match update {
            CoreUpdate::Results {
                results,
                placeholder,
                clear_input,
                input_mode,
                context,
                navigate_forward,
                ..
            } => {
                assert_eq!(results.len(), 1);
                assert_eq!(placeholder, Some("Search...".to_string()));
                assert_eq!(clear_input, Some(true));
                assert_eq!(input_mode, Some("realtime".to_string()));
                assert_eq!(context, Some("__add_mode__".to_string()));
                assert!(navigate_forward.is_none());
            }
            _ => panic!("Expected Results update"),
        }
    }

    #[test]
    fn test_results_update_deserialization_with_navigate_forward() {
        let json = json!({
            "type": "results",
            "results": [{"id": "item-1", "name": "Item 1"}],
            "navigateForward": true
        });

        let update: CoreUpdate = serde_json::from_value(json).expect("Failed to deserialize");
        match update {
            CoreUpdate::Results {
                navigate_forward, ..
            } => {
                assert_eq!(
                    navigate_forward,
                    Some(true),
                    "navigateForward should be Some(true)"
                );
            }
            _ => panic!("Expected Results update"),
        }
    }

    #[test]
    fn test_results_update_deserialization_without_optional_fields() {
        let json = json!({
            "type": "results",
            "results": []
        });

        let update: CoreUpdate = serde_json::from_value(json).expect("Failed to deserialize");
        match update {
            CoreUpdate::Results {
                placeholder,
                clear_input,
                input_mode,
                context,
                navigate_forward,
                ..
            } => {
                assert!(placeholder.is_none());
                assert!(clear_input.is_none());
                assert!(input_mode.is_none());
                assert!(context.is_none());
                assert!(navigate_forward.is_none());
            }
            _ => panic!("Expected Results update"),
        }
    }

    #[test]
    fn test_slider_value_correct_struct_format() {
        // Correct format: SliderValue struct with value, min, max, step for a slider
        let json = json!({
            "id": "slider_id",
            "name": "Volume",
            "resultType": "slider",
            "value": {
                "value": 50.0,
                "min": 0.0,
                "max": 100.0,
                "step": 1.0
            }
        });

        let result: SearchResult = serde_json::from_value(json).expect("Failed to deserialize");
        assert_eq!(result.id, "slider_id");
        assert_eq!(result.result_type, ResultType::Slider);
        assert!(result.widget.is_some());
        let slider_value = result.slider_value().expect("Failed to get slider value");
        assert_eq!(slider_value.value, 50.0);
        assert_eq!(slider_value.min, 0.0);
        assert_eq!(slider_value.max, 100.0);
        assert_eq!(slider_value.step, 1.0);
    }

    #[test]
    fn test_slider_value_boolean_format_fails() {
        // WRONG format: boolean value for slider should fail
        // Boolean values are ONLY valid for Switch type, not Slider
        let json = json!({
            "id": "slider_id",
            "name": "Volume",
            "resultType": "slider",
            "value": true  // THIS IS WRONG - sliders need numeric values
        });

        let result: Result<SearchResult, _> = serde_json::from_value(json);
        assert!(
            result.is_err(),
            "Boolean value should NOT deserialize for Slider type"
        );
    }

    #[test]
    fn test_switch_value_boolean_format_supported() {
        // Boolean format is supported for switches
        let json_true = json!({
            "id": "switch_id",
            "name": "Power",
            "resultType": "switch",
            "value": true
        });

        let result: SearchResult = serde_json::from_value(json_true).unwrap();
        assert_eq!(result.result_type, ResultType::Switch);
        assert!(matches!(
            result.widget,
            Some(WidgetData::Switch { value: true })
        ));

        let json_false = json!({
            "id": "switch_id",
            "name": "Power",
            "resultType": "switch",
            "value": false
        });

        let result: SearchResult = serde_json::from_value(json_false).unwrap();
        assert!(matches!(
            result.widget,
            Some(WidgetData::Switch { value: false })
        ));
    }

    #[test]
    fn test_slider_value_with_display_value() {
        // SliderValue can have optional display_value field
        let json = json!({
            "id": "slider_id",
            "name": "Volume",
            "resultType": "slider",
            "value": {
                "value": 75.0,
                "min": 0.0,
                "max": 100.0,
                "step": 5.0,
                "displayValue": "75%"
            }
        });

        let result: SearchResult = serde_json::from_value(json).expect("Failed to deserialize");
        assert_eq!(result.id, "slider_id");
        assert!(result.widget.is_some());
        let slider_value = result.slider_value().expect("Failed to get slider value");
        assert_eq!(slider_value.value, 75.0);
        assert_eq!(slider_value.display_value, Some("75%".to_string()));
    }

    #[test]
    fn test_switch_value_number_format_accepted() {
        // Numeric 1.0 for switch is treated as true
        let json = json!({
            "id": "switch_id",
            "name": "Power",
            "resultType": "switch",
            "value": 1.0
        });

        let result: SearchResult = serde_json::from_value(json).expect("Failed to deserialize");
        assert_eq!(result.id, "switch_id");
        // For switch with numeric value, widget is populated as Switch (1.0 -> true)
        assert!(matches!(
            result.widget,
            Some(WidgetData::Switch { value: true })
        ));
    }

    #[test]
    fn test_result_type_alias_accepts_type_field() {
        // Plugins send "type" instead of "resultType" - verify the alias works
        let json = json!({
            "id": "switch_id",
            "name": "Enable Feature",
            "type": "switch",  // Using "type" instead of "resultType"
            "value": {
                "value": 1.0,
                "min": 0.0,
                "max": 1.0,
                "step": 1.0
            }
        });

        let result: SearchResult = serde_json::from_value(json).expect("Failed to deserialize");
        assert_eq!(result.id, "switch_id");
        assert_eq!(
            result.result_type,
            ResultType::Switch,
            "\"type\" field should map to result_type via alias"
        );
    }

    #[test]
    fn test_slider_value_serialize_deserialize_roundtrip() {
        // Create a slider using builder method
        let original = ResultItem {
            id: "volume".to_string(),
            name: "Volume Control".to_string(),
            ..Default::default()
        }
        .with_slider(75.0, 0.0, 100.0, 5.0, Some("75%".to_string()));

        // Serialize to JSON
        let json = serde_json::to_string(&original).expect("Failed to serialize");

        // Verify JSON contains the widget field with slider data
        assert!(json.contains("\"widget\""), "JSON should contain widget");

        // Deserialize back
        let parsed: ResultItem = serde_json::from_str(&json).expect("Failed to deserialize");

        // Verify slider values via slider_value() method
        let slider_value = parsed.slider_value().expect("slider_value should exist");
        assert_eq!(slider_value.value, 75.0, "value should roundtrip");
        assert_eq!(slider_value.min, 0.0, "min should roundtrip");
        assert_eq!(slider_value.max, 100.0, "max should roundtrip");
        assert_eq!(slider_value.step, 5.0, "step should roundtrip");
        assert_eq!(
            slider_value.display_value,
            Some("75%".to_string()),
            "display_value should roundtrip"
        );
    }

    #[test]
    fn test_slider_type_from_json_with_type_field() {
        // This tests that "type": "slider" is correctly parsed (using alias)
        let json = r#"{"id":"setting:sizes.searchWidth","name":"searchWidth","description":"test","icon":"tune","type":"slider","value":580.0,"min":400,"max":1000,"step":10}"#;

        let result: SearchResult = serde_json::from_str(json).expect("Failed to deserialize");
        assert_eq!(result.id, "setting:sizes.searchWidth");
        assert_eq!(
            result.result_type,
            ResultType::Slider,
            "type field should parse to ResultType::Slider"
        );
        assert!(result.is_slider(), "is_slider() should return true");
        let slider_value = result.slider_value().expect("slider_value should exist");
        assert_eq!(slider_value.value, 580.0);
        assert_eq!(slider_value.min, 400.0);
        assert_eq!(slider_value.max, 1000.0);
        assert_eq!(slider_value.step, 10.0);
    }

    #[test]
    fn test_plugin_status_null_arrays_become_empty() {
        use super::PluginStatus;

        let json = serde_json::json!({
            "badges": null,
            "chips": null,
            "ambient": null,
            "fab": null
        });

        let status: PluginStatus =
            serde_json::from_value(json).expect("Failed to deserialize with null arrays");
        assert!(
            status.badges.is_empty(),
            "null badges should become empty vec"
        );
        assert!(
            status.chips.is_empty(),
            "null chips should become empty vec"
        );
        assert!(
            status.ambient.is_empty(),
            "null ambient should become empty vec"
        );
        assert!(status.fab.is_none(), "null fab should be None");
    }

    #[test]
    fn test_search_result_null_arrays_become_empty() {
        let json = serde_json::json!({
            "id": "test",
            "name": "Test Item",
            "badges": null,
            "chips": null,
            "actions": null
        });

        let result: SearchResult =
            serde_json::from_value(json).expect("Failed to deserialize with null arrays");
        assert!(
            result.badges.is_empty(),
            "null badges should become empty vec"
        );
        assert!(
            result.chips.is_empty(),
            "null chips should become empty vec"
        );
        assert!(
            result.actions.is_empty(),
            "null actions should become empty vec"
        );
    }

    #[test]
    fn test_ambient_item_null_arrays_become_empty() {
        use super::AmbientItem;

        let json = serde_json::json!({
            "id": "timer:123",
            "name": "Timer",
            "badges": null,
            "chips": null,
            "actions": null
        });

        let item: AmbientItem =
            serde_json::from_value(json).expect("Failed to deserialize with null arrays");
        assert!(
            item.badges.is_empty(),
            "null badges should become empty vec"
        );
        assert!(item.chips.is_empty(), "null chips should become empty vec");
        assert!(
            item.actions.is_empty(),
            "null actions should become empty vec"
        );
    }

    #[test]
    fn test_fab_override_null_arrays_become_empty() {
        use super::FabOverride;

        let json = serde_json::json!({
            "badges": null,
            "chips": null,
            "priority": 10
        });

        let fab: FabOverride =
            serde_json::from_value(json).expect("Failed to deserialize with null arrays");
        assert!(fab.badges.is_empty(), "null badges should become empty vec");
        assert!(fab.chips.is_empty(), "null chips should become empty vec");
        assert_eq!(fab.priority, 10);
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)] // Exact float comparisons are intentional in tests
mod widget_data_tests {
    use super::*;

    #[test]
    fn test_widget_slider_serialization_roundtrip() {
        let widget = WidgetData::Slider {
            value: 75.0,
            min: 0.0,
            max: 100.0,
            step: 1.0,
            display_value: Some("75%".to_string()),
        };
        let json = serde_json::to_string(&widget).unwrap();
        assert!(json.contains("\"type\":\"slider\""));
        assert!(json.contains("\"value\":75"));
        assert!(json.contains("\"displayValue\":\"75%\""));

        let parsed: WidgetData = serde_json::from_str(&json).unwrap();
        assert_eq!(widget, parsed);
    }

    #[test]
    fn test_widget_switch_serialization_roundtrip() {
        let widget = WidgetData::Switch { value: true };
        let json = serde_json::to_string(&widget).unwrap();
        assert!(json.contains("\"type\":\"switch\""));
        assert!(json.contains("\"value\":true"));

        let parsed: WidgetData = serde_json::from_str(&json).unwrap();
        assert_eq!(widget, parsed);

        // Also test false value
        let widget_false = WidgetData::Switch { value: false };
        let json_false = serde_json::to_string(&widget_false).unwrap();
        assert!(json_false.contains("\"value\":false"));
    }

    #[test]
    fn test_widget_gauge_with_optional_fields() {
        // Minimal gauge - only required fields
        let json = r#"{"type":"gauge","value":50.0}"#;
        let widget: WidgetData = serde_json::from_str(json).unwrap();
        match widget {
            WidgetData::Gauge {
                value,
                min,
                max,
                label,
                color,
            } => {
                assert_eq!(value, 50.0);
                assert_eq!(min, 0.0); // default
                assert_eq!(max, 100.0); // default
                assert!(label.is_none());
                assert!(color.is_none());
            }
            _ => panic!("Expected Gauge variant"),
        }

        // Full gauge with all fields
        let json_full = r##"{"type":"gauge","value":75.0,"min":0.0,"max":100.0,"label":"75%","color":"#4caf50"}"##;
        let widget_full: WidgetData = serde_json::from_str(json_full).unwrap();
        match widget_full {
            WidgetData::Gauge { label, color, .. } => {
                assert_eq!(label, Some("75%".to_string()));
                assert_eq!(color, Some("#4caf50".to_string()));
            }
            _ => panic!("Expected Gauge variant"),
        }
    }

    #[test]
    fn test_widget_progress_serialization() {
        let widget = WidgetData::Progress {
            value: 2.8,
            max: 4.7,
            label: Some("2.8 GB / 4.7 GB".to_string()),
            color: None,
        };
        let json = serde_json::to_string(&widget).unwrap();

        // color should not be present (skip_serializing_if)
        assert!(!json.contains("color"));
        assert!(json.contains("\"label\":\"2.8 GB / 4.7 GB\""));

        let parsed: WidgetData = serde_json::from_str(&json).unwrap();
        assert_eq!(widget, parsed);
    }

    #[test]
    fn test_widget_graph_serialization() {
        let widget = WidgetData::Graph {
            data: vec![10.0, 20.0, 15.0, 30.0],
            min: None,
            max: Some(100.0),
        };
        let json = serde_json::to_string(&widget).unwrap();

        // min should not be present (skip_serializing_if)
        assert!(!json.contains("\"min\""));
        assert!(json.contains("\"max\":100"));

        let parsed: WidgetData = serde_json::from_str(&json).unwrap();
        match parsed {
            WidgetData::Graph { data, min, max } => {
                assert_eq!(data, vec![10.0, 20.0, 15.0, 30.0]);
                assert!(min.is_none());
                assert_eq!(max, Some(100.0));
            }
            _ => panic!("Expected Graph variant"),
        }
    }

    #[test]
    fn test_widget_slider_defaults() {
        // Minimal slider - only value required
        let json = r#"{"type":"slider","value":50.0}"#;
        let widget: WidgetData = serde_json::from_str(json).unwrap();
        match widget {
            WidgetData::Slider {
                value,
                min,
                max,
                step,
                display_value,
            } => {
                assert_eq!(value, 50.0);
                assert_eq!(min, 0.0); // default
                assert_eq!(max, 100.0); // default
                assert_eq!(step, 1.0); // default
                assert!(display_value.is_none());
            }
            _ => panic!("Expected Slider variant"),
        }
    }

    #[test]
    fn test_widget_value_method() {
        let slider = WidgetData::Slider {
            value: 75.0,
            min: 0.0,
            max: 100.0,
            step: 1.0,
            display_value: None,
        };
        assert_eq!(slider.value(), Some(75.0));

        let switch_on = WidgetData::Switch { value: true };
        assert_eq!(switch_on.value(), Some(1.0));

        let switch_off = WidgetData::Switch { value: false };
        assert_eq!(switch_off.value(), Some(0.0));

        let gauge = WidgetData::Gauge {
            value: 50.0,
            min: 0.0,
            max: 100.0,
            label: None,
            color: None,
        };
        assert_eq!(gauge.value(), Some(50.0));

        let graph = WidgetData::Graph {
            data: vec![1.0, 2.0],
            min: None,
            max: None,
        };
        assert_eq!(graph.value(), None);
    }

    #[test]
    fn test_widget_is_interactive() {
        let slider = WidgetData::Slider {
            value: 50.0,
            min: 0.0,
            max: 100.0,
            step: 1.0,
            display_value: None,
        };
        assert!(slider.is_interactive());

        let switch = WidgetData::Switch { value: true };
        assert!(switch.is_interactive());

        let gauge = WidgetData::Gauge {
            value: 50.0,
            min: 0.0,
            max: 100.0,
            label: None,
            color: None,
        };
        assert!(!gauge.is_interactive());

        let progress = WidgetData::Progress {
            value: 50.0,
            max: 100.0,
            label: None,
            color: None,
        };
        assert!(!progress.is_interactive());

        let graph = WidgetData::Graph {
            data: vec![1.0],
            min: None,
            max: None,
        };
        assert!(!graph.is_interactive());
    }

    #[test]
    fn test_result_item_widget_field_populated_for_slider() {
        let json = r#"{"id":"vol","name":"Volume","resultType":"slider","value":75.0,"min":0,"max":100,"step":5,"displayValue":"75%"}"#;
        let result: ResultItem = serde_json::from_str(json).unwrap();

        // widget field should be populated from flat JSON fields
        let widget = result.widget.expect("widget field should be populated");
        match widget {
            WidgetData::Slider {
                value,
                min,
                max,
                step,
                display_value,
            } => {
                assert_eq!(value, 75.0);
                assert_eq!(min, 0.0);
                assert_eq!(max, 100.0);
                assert_eq!(step, 5.0);
                assert_eq!(display_value, Some("75%".to_string()));
            }
            _ => panic!("Expected Slider widget"),
        }
    }

    #[test]
    fn test_result_item_widget_field_populated_for_switch() {
        let json = r#"{"id":"wifi","name":"WiFi","resultType":"switch","value":true}"#;
        let result: ResultItem = serde_json::from_str(json).unwrap();

        // widget field should have proper bool
        let widget = result.widget.expect("widget field should be populated");
        match widget {
            WidgetData::Switch { value } => {
                assert!(value, "switch value should be true");
            }
            _ => panic!("Expected Switch widget"),
        }

        // Test false value
        let json_off = r#"{"id":"wifi","name":"WiFi","resultType":"switch","value":false}"#;
        let result_off: ResultItem = serde_json::from_str(json_off).unwrap();
        let widget_off = result_off.widget.expect("widget field should be populated");
        match widget_off {
            WidgetData::Switch { value } => {
                assert!(!value, "switch value should be false");
            }
            _ => panic!("Expected Switch widget"),
        }
    }

    #[test]
    fn test_result_item_widget_field_populated_for_gauge() {
        let json = r##"{"id":"mem","name":"Memory","gauge":{"value":75.5,"min":0,"max":100,"label":"12GB/16GB","color":"#4caf50"}}"##;
        let result: ResultItem = serde_json::from_str(json).unwrap();

        // widget field should ALSO be populated
        let widget = result.widget.expect("widget field should be populated");
        match widget {
            WidgetData::Gauge {
                value,
                min,
                max,
                label,
                color,
            } => {
                assert_eq!(value, 75.5);
                assert_eq!(min, 0.0);
                assert_eq!(max, 100.0);
                assert_eq!(label, Some("12GB/16GB".to_string()));
                assert_eq!(color, Some("#4caf50".to_string()));
            }
            _ => panic!("Expected Gauge widget"),
        }
    }

    #[test]
    fn test_result_item_widget_field_populated_for_progress() {
        let json =
            r#"{"id":"dl","name":"Download","progress":{"value":2.8,"max":4.7,"label":"60%"}}"#;
        let result: ResultItem = serde_json::from_str(json).unwrap();

        // widget field should be populated
        let widget = result.widget.expect("widget field should be populated");
        match widget {
            WidgetData::Progress {
                value, max, label, ..
            } => {
                assert_eq!(value, 2.8);
                assert_eq!(max, 4.7);
                assert_eq!(label, Some("60%".to_string()));
            }
            _ => panic!("Expected Progress widget"),
        }
    }

    #[test]
    fn test_result_item_widget_field_populated_for_graph() {
        let json = r#"{"id":"cpu","name":"CPU","graph":{"data":[10,20,15,30],"min":0,"max":100}}"#;
        let result: ResultItem = serde_json::from_str(json).unwrap();

        // widget field should be populated
        let widget = result.widget.expect("widget field should be populated");
        match widget {
            WidgetData::Graph { data, min, max } => {
                assert_eq!(data, vec![10.0, 20.0, 15.0, 30.0]);
                assert_eq!(min, Some(0.0));
                assert_eq!(max, Some(100.0));
            }
            _ => panic!("Expected Graph widget"),
        }
    }

    #[test]
    fn test_result_item_widget_field_none_for_normal_item() {
        let json = r#"{"id":"app","name":"Firefox","resultType":"app"}"#;
        let result: ResultItem = serde_json::from_str(json).unwrap();

        assert!(
            result.widget.is_none(),
            "widget should be None for normal items"
        );
    }
}

#[cfg(test)]
mod frecency_tests {
    use super::*;

    #[test]
    fn test_frecency_default() {
        let f = Frecency::default();
        assert_eq!(f.count, 0);
        assert_eq!(f.last_used, 0);
        assert!(f.recent_search_terms.is_empty());
        assert_eq!(f.hour_slot_counts, [0u32; 24]);
        assert_eq!(f.day_of_week_counts, [0u32; 7]);
        assert_eq!(f.consecutive_days, 0);
        assert!(f.last_consecutive_date.is_none());
        assert!(f.workspace_counts.is_empty());
    }

    #[test]
    fn test_frecency_new_with_usage() {
        let f = Frecency::new_with_usage(10, 1_737_012_000_000);
        assert_eq!(f.count, 10);
        assert_eq!(f.last_used, 1_737_012_000_000);
        assert!(f.recent_search_terms.is_empty());
    }

    #[test]
    fn test_frecency_has_usage() {
        let f_none = Frecency::default();
        assert!(!f_none.has_usage());

        let f_some = Frecency::new_with_usage(1, 0);
        assert!(f_some.has_usage());
    }

    #[test]
    fn test_frecency_age_ms() {
        let f = Frecency::new_with_usage(5, 1000);
        assert_eq!(f.age_ms(1500), 500);
        assert_eq!(f.age_ms(1000), 0);
        assert_eq!(f.age_ms(500), 0); // saturating sub
    }

    #[test]
    fn test_frecency_serialization_roundtrip() {
        let f = Frecency {
            count: 10,
            last_used: 1_737_012_000_000,
            recent_search_terms: vec!["browser".to_string(), "firefox".to_string()],
            hour_slot_counts: {
                let mut arr = [0u32; 24];
                arr[9] = 5;
                arr[10] = 3;
                arr
            },
            day_of_week_counts: [1, 2, 3, 4, 5, 0, 0],
            consecutive_days: 3,
            last_consecutive_date: Some("2025-01-15".to_string()),
            ..Default::default()
        };

        let json = serde_json::to_string(&f).unwrap();
        let parsed: Frecency = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.count, 10);
        assert_eq!(parsed.last_used, 1_737_012_000_000);
        assert_eq!(parsed.recent_search_terms.len(), 2);
        assert_eq!(parsed.hour_slot_counts[9], 5);
        assert_eq!(parsed.day_of_week_counts[2], 3);
        assert_eq!(parsed.consecutive_days, 3);
        assert_eq!(parsed.last_consecutive_date, Some("2025-01-15".to_string()));
    }

    #[test]
    fn test_frecency_skip_empty_fields() {
        let f = Frecency {
            count: 5,
            last_used: 1000,
            ..Default::default()
        };

        let json = serde_json::to_string(&f).unwrap();

        // Empty arrays/maps should not be serialized
        assert!(!json.contains("recentSearchTerms"));
        assert!(!json.contains("hourSlotCounts"));
        assert!(!json.contains("dayOfWeekCounts"));
        assert!(!json.contains("workspaceCounts"));
        assert!(!json.contains("consecutiveDays")); // 0 is skipped
    }

    #[test]
    fn test_frecency_deserialize_minimal() {
        let json = r#"{"count":5,"lastUsed":1000}"#;
        let f: Frecency = serde_json::from_str(json).unwrap();

        assert_eq!(f.count, 5);
        assert_eq!(f.last_used, 1000);
        assert!(f.recent_search_terms.is_empty());
        assert_eq!(f.hour_slot_counts, [0u32; 24]);
    }

    #[test]
    fn test_frecency_with_workspace_counts() {
        let mut f = Frecency::default();
        f.workspace_counts.insert("workspace-1".to_string(), 10);
        f.workspace_counts.insert("workspace-2".to_string(), 5);

        let json = serde_json::to_string(&f).unwrap();
        assert!(json.contains("workspaceCounts"));

        let parsed: Frecency = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.workspace_counts.get("workspace-1"), Some(&10));
        assert_eq!(parsed.workspace_counts.get("workspace-2"), Some(&5));
    }

    #[test]
    fn test_frecency_session_duration_counts() {
        let f = Frecency {
            session_duration_counts: [1, 2, 3, 4, 5],
            ..Default::default()
        };

        let json = serde_json::to_string(&f).unwrap();
        let parsed: Frecency = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.session_duration_counts, [1, 2, 3, 4, 5]);
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)] // Exact float comparisons are intentional in tests
mod edge_case_tests {
    use super::*;
    use serde_json::json;

    // ResultItem parsing edge cases

    #[test]
    fn test_result_item_missing_required_id_fails() {
        let json = json!({
            "name": "Test Item"
        });
        let result: Result<ResultItem, _> = serde_json::from_value(json);
        assert!(result.is_err(), "Missing id should fail");
    }

    #[test]
    fn test_result_item_missing_required_name_fails() {
        let json = json!({
            "id": "test-id"
        });
        let result: Result<ResultItem, _> = serde_json::from_value(json);
        assert!(result.is_err(), "Missing name should fail");
    }

    #[test]
    fn test_result_item_empty_id_and_name_accepted() {
        let json = json!({
            "id": "",
            "name": ""
        });
        let result: ResultItem = serde_json::from_value(json).unwrap();
        assert_eq!(result.id, "");
        assert_eq!(result.name, "");
    }

    #[test]
    fn test_result_item_invalid_value_type_string_fails() {
        let json = json!({
            "id": "test",
            "name": "Test",
            "value": "not a number"
        });
        let result: Result<ResultItem, _> = serde_json::from_value(json);
        assert!(result.is_err(), "String value should fail");
    }

    #[test]
    fn test_result_item_invalid_value_type_array_fails() {
        let json = json!({
            "id": "test",
            "name": "Test",
            "value": [1, 2, 3]
        });
        let result: Result<ResultItem, _> = serde_json::from_value(json);
        assert!(result.is_err(), "Array value should fail");
    }

    #[test]
    fn test_result_item_unknown_result_type_accepted() {
        let json = json!({
            "id": "test",
            "name": "Test",
            "resultType": "unknown_type"
        });
        // Unknown types should fail deserialization because enum is strict
        let result: Result<ResultItem, _> = serde_json::from_value(json);
        assert!(result.is_err(), "Unknown result_type should fail");
    }

    #[test]
    fn test_result_item_all_result_types_valid() {
        let types = [
            "normal",
            "app",
            "plugin",
            "indexed_item",
            "slider",
            "switch",
            "web_search",
            "suggestion",
            "recent",
            "pattern_match",
        ];
        for type_str in types {
            let json = json!({
                "id": "test",
                "name": "Test",
                "resultType": type_str
            });
            let result: Result<ResultItem, _> = serde_json::from_value(json);
            assert!(result.is_ok(), "Result type {type_str} should be valid");
        }
    }

    #[test]
    fn test_result_item_negative_slider_values_accepted() {
        let json = json!({
            "id": "temp",
            "name": "Temperature",
            "resultType": "slider",
            "value": -10.0,
            "min": -50.0,
            "max": 50.0
        });
        let result: ResultItem = serde_json::from_value(json).unwrap();
        let Some(WidgetData::Slider { value, min, .. }) = result.widget else {
            panic!("Expected Slider widget");
        };
        assert_eq!(value, -10.0);
        assert_eq!(min, -50.0);
    }

    #[test]
    fn test_result_item_unicode_content() {
        let json = json!({
            "id": "emoji",
            "name": "Search Results",
            "description": "Description with unicode: cafe \u{2615}"
        });
        let result: ResultItem = serde_json::from_value(json).unwrap();
        assert!(result.description.unwrap().contains('\u{2615}'));
    }

    #[test]
    fn test_result_item_very_long_strings() {
        let long_string = "a".repeat(10000);
        let json = json!({
            "id": long_string,
            "name": long_string
        });
        let result: ResultItem = serde_json::from_value(json).unwrap();
        assert_eq!(result.id.len(), 10000);
        assert_eq!(result.name.len(), 10000);
    }

    // IconSpec edge cases

    #[test]
    fn test_icon_spec_empty_string_material() {
        let icon = IconSpec::from_wire(String::new(), None);
        match icon {
            IconSpec::Material(name) => assert_eq!(name, ""),
            _ => panic!("Empty string should default to Material"),
        }
    }

    #[test]
    fn test_icon_spec_unknown_type_defaults_material() {
        let icon = IconSpec::from_wire("test".to_string(), Some("unknown_type"));
        match icon {
            IconSpec::Material(name) => assert_eq!(name, "test"),
            _ => panic!("Unknown type should default to Material"),
        }
    }

    #[test]
    fn test_icon_spec_deserialize_plain_string() {
        let json = json!("my-icon");
        let icon: IconSpec = serde_json::from_value(json).unwrap();
        match icon {
            IconSpec::Material(name) => assert_eq!(name, "my-icon"),
            _ => panic!("Plain string should deserialize to Material"),
        }
    }

    #[test]
    fn test_icon_spec_deserialize_tagged_with_invalid_value_fails() {
        let json = json!({
            "type": "system",
            "value": 123  // Should be a string
        });
        let result: Result<IconSpec, _> = serde_json::from_value(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_icon_spec_deserialize_unknown_type_fails() {
        let json = json!({
            "type": "unknown",
            "value": "test"
        });
        let result: Result<IconSpec, _> = serde_json::from_value(json);
        assert!(result.is_err());
    }

    // CoreEvent edge cases

    #[test]
    fn test_core_event_unknown_type_fails() {
        let json = json!({
            "type": "unknown_event_type"
        });
        let result: Result<CoreEvent, _> = serde_json::from_value(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_core_event_query_changed_empty_query() {
        let json = json!({
            "type": "query_changed",
            "query": ""
        });
        let event: CoreEvent = serde_json::from_value(json).unwrap();
        match event {
            CoreEvent::QueryChanged { query } => assert_eq!(query, ""),
            _ => panic!("Expected QueryChanged"),
        }
    }

    #[test]
    fn test_core_event_item_selected_missing_optional_fields() {
        let json = json!({
            "type": "item_selected",
            "id": "item-1"
        });
        let event: CoreEvent = serde_json::from_value(json).unwrap();
        match event {
            CoreEvent::ItemSelected {
                id,
                action,
                plugin_id,
            } => {
                assert_eq!(id, "item-1");
                assert!(action.is_none());
                assert!(plugin_id.is_none());
            }
            _ => panic!("Expected ItemSelected"),
        }
    }

    #[test]
    fn test_core_event_form_submitted_empty_data() {
        let json = json!({
            "type": "form_submitted",
            "form_data": {}
        });
        let event: CoreEvent = serde_json::from_value(json).unwrap();
        match event {
            CoreEvent::FormSubmitted { form_data, context } => {
                assert!(form_data.is_empty());
                assert!(context.is_none());
            }
            _ => panic!("Expected FormSubmitted"),
        }
    }

    // CoreUpdate edge cases

    #[test]
    fn test_core_update_results_empty_results() {
        let json = json!({
            "type": "results",
            "results": []
        });
        let update: CoreUpdate = serde_json::from_value(json).unwrap();
        match update {
            CoreUpdate::Results { results, .. } => {
                assert!(results.is_empty());
            }
            _ => panic!("Expected Results"),
        }
    }

    #[test]
    fn test_core_update_error_empty_message() {
        let update = CoreUpdate::Error {
            message: String::new(),
        };
        let json = serde_json::to_value(&update).unwrap();
        assert_eq!(json["message"], "");
    }

    // FormData/FormField edge cases

    #[test]
    fn test_form_data_defaults() {
        let json = json!({
            "title": "Test Form",
            "fields": []
        });
        let form: FormData = serde_json::from_value(json).unwrap();
        assert_eq!(form.title, "Test Form");
        assert!(form.fields.is_empty());
        assert_eq!(form.submit_label, "Submit"); // default
        assert!(form.cancel_label.is_none());
        assert!(!form.live_update); // default false
    }

    #[test]
    fn test_form_field_all_types() {
        let types = [
            "text",
            "password",
            "number",
            "text_area",
            "select",
            "checkbox",
            "switch",
            "slider",
            "hidden",
            "date",
            "time",
            "email",
            "url",
            "phone",
        ];
        for type_str in types {
            let json = json!({
                "id": "field1",
                "label": "Field",
                "type": type_str
            });
            let field: FormField = serde_json::from_value(json).unwrap();
            assert_eq!(field.id, "field1");
        }
    }

    #[test]
    fn test_form_field_unknown_type_fails() {
        let json = json!({
            "id": "field1",
            "label": "Field",
            "type": "unknown_field_type"
        });
        let result: Result<FormField, _> = serde_json::from_value(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_form_field_defaults_to_text() {
        let json = json!({
            "id": "field1",
            "label": "Field"
        });
        let field: FormField = serde_json::from_value(json).unwrap();
        assert_eq!(field.field_type, FormFieldType::Text);
    }

    #[test]
    fn test_form_option_serialization() {
        let option = FormOption {
            value: "opt1".to_string(),
            label: "Option 1".to_string(),
        };
        let json = serde_json::to_value(&option).unwrap();
        assert_eq!(json["value"], "opt1");
        assert_eq!(json["label"], "Option 1");
    }

    // Progress deserialization edge cases

    #[test]
    fn test_progress_from_number() {
        let json = json!({
            "id": "download",
            "name": "Downloading",
            "progress": 75.0
        });
        let item: ResultItem = serde_json::from_value(json).unwrap();
        let Some(WidgetData::Progress { value, max, .. }) = item.widget else {
            panic!("Expected Progress widget");
        };
        assert_eq!(value, 75.0);
        assert_eq!(max, 100.0); // default
    }

    #[test]
    fn test_progress_from_object() {
        let json = json!({
            "id": "download",
            "name": "Downloading",
            "progress": {
                "value": 2.8,
                "max": 4.7,
                "label": "2.8 GB / 4.7 GB"
            }
        });
        let item: ResultItem = serde_json::from_value(json).unwrap();
        let Some(WidgetData::Progress {
            value, max, label, ..
        }) = item.widget
        else {
            panic!("Expected Progress widget");
        };
        assert_eq!(value, 2.8);
        assert_eq!(max, 4.7);
        assert_eq!(label, Some("2.8 GB / 4.7 GB".to_string()));
    }

    #[test]
    fn test_progress_invalid_type_fails() {
        let json = json!({
            "id": "download",
            "name": "Downloading",
            "progress": "invalid"
        });
        let result: Result<ResultItem, _> = serde_json::from_value(json);
        assert!(result.is_err());
    }

    // DisplayHint edge cases

    #[test]
    fn test_display_hint_all_variants() {
        let variants = ["auto", "list", "grid", "large_grid"];
        for variant in variants {
            let json = json!(variant);
            let hint: DisplayHint = serde_json::from_value(json).unwrap();
            assert!(matches!(
                hint,
                DisplayHint::Auto | DisplayHint::List | DisplayHint::Grid | DisplayHint::LargeGrid
            ));
        }
    }

    #[test]
    fn test_display_hint_invalid_fails() {
        let json = json!("unknown");
        let result: Result<DisplayHint, _> = serde_json::from_value(json);
        assert!(result.is_err());
    }

    // ExecuteAction edge cases

    #[test]
    fn test_execute_action_all_variants() {
        let variants = [
            json!({"type": "launch", "desktop_file": "firefox.desktop"}),
            json!({"type": "open_url", "url": "https://example.com"}),
            json!({"type": "open", "path": "/home/user"}),
            json!({"type": "copy", "text": "copied text"}),
            json!({"type": "type_text", "text": "typed text"}),
            json!({"type": "sound", "sound": "notification"}),
            json!({"type": "notify", "message": "Hello"}),
        ];
        for variant in variants {
            let action: ExecuteAction = serde_json::from_value(variant).unwrap();
            match action {
                ExecuteAction::Launch { .. }
                | ExecuteAction::OpenUrl { .. }
                | ExecuteAction::Open { .. }
                | ExecuteAction::Copy { .. }
                | ExecuteAction::TypeText { .. }
                | ExecuteAction::PlaySound { .. }
                | ExecuteAction::Notify { .. } => {}
            }
        }
    }

    #[test]
    fn test_execute_action_unknown_type_fails() {
        let json = json!({"type": "unknown_action"});
        let result: Result<ExecuteAction, _> = serde_json::from_value(json);
        assert!(result.is_err());
    }

    // CardBlock edge cases

    #[test]
    fn test_card_block_all_variants() {
        let variants = [
            json!({"block_type": "pill", "text": "Today"}),
            json!({"block_type": "separator"}),
            json!({"block_type": "message", "role": "user", "content": "Hello"}),
            json!({"block_type": "note", "content": "Note text"}),
        ];
        for variant in variants {
            let block: CardBlock = serde_json::from_value(variant).unwrap();
            match block {
                CardBlock::Pill { .. }
                | CardBlock::Separator
                | CardBlock::Message { .. }
                | CardBlock::Note { .. } => {}
            }
        }
    }

    #[test]
    fn test_card_data_minimal() {
        let json = json!({"title": "Card Title"});
        let card: CardData = serde_json::from_value(json).unwrap();
        assert_eq!(card.title, "Card Title");
        assert!(card.content.is_none());
        assert!(card.actions.is_empty());
        assert!(card.blocks.is_empty());
    }

    // Badge/Chip edge cases

    #[test]
    fn test_badge_all_fields_optional() {
        let json = json!({});
        let badge: Badge = serde_json::from_value(json).unwrap();
        assert!(badge.text.is_none());
        assert!(badge.icon.is_none());
        assert!(badge.color.is_none());
    }

    #[test]
    fn test_chip_label_alias() {
        let json = json!({"label": "Label Text"});
        let chip: Chip = serde_json::from_value(json).unwrap();
        assert_eq!(chip.text, "Label Text");
    }

    #[test]
    fn test_chip_text_field() {
        let json = json!({"text": "Text Value"});
        let chip: Chip = serde_json::from_value(json).unwrap();
        assert_eq!(chip.text, "Text Value");
    }

    // GaugeData/GraphData edge cases

    #[test]
    fn test_gauge_data_defaults() {
        let json = json!({"value": 50.0});
        let gauge: GaugeData = serde_json::from_value(json).unwrap();
        assert_eq!(gauge.value, 50.0);
        assert_eq!(gauge.min, 0.0); // default
        assert_eq!(gauge.max, 100.0); // default
    }

    #[test]
    fn test_graph_data_empty_data() {
        let json = json!({"data": []});
        let graph: GraphData = serde_json::from_value(json).unwrap();
        assert!(graph.data.is_empty());
    }

    // SliderValue edge cases

    #[test]
    fn test_slider_value_defaults() {
        let json = json!({"value": 50.0});
        let slider: SliderValue = serde_json::from_value(json).unwrap();
        assert_eq!(slider.value, 50.0);
        assert_eq!(slider.min, 0.0); // default
        assert_eq!(slider.max, 100.0); // default
        assert_eq!(slider.step, 1.0); // default
    }

    // AmbientItem edge cases

    #[test]
    fn test_ambient_item_minimal() {
        let json = json!({
            "id": "ambient-1",
            "name": "Ambient"
        });
        let item: AmbientItem = serde_json::from_value(json).unwrap();
        assert_eq!(item.id, "ambient-1");
        assert_eq!(item.duration, 0); // default permanent
    }

    // PluginManifest edge cases

    #[test]
    fn test_plugin_manifest_minimal() {
        let json = json!({
            "id": "plugin-id",
            "name": "Plugin Name"
        });
        let manifest: PluginManifest = serde_json::from_value(json).unwrap();
        assert_eq!(manifest.id, "plugin-id");
        assert_eq!(manifest.name, "Plugin Name");
        assert!(manifest.description.is_none());
        assert_eq!(manifest.priority, 0); // default
    }

    // ResultItem helper methods

    #[test]
    fn test_result_item_icon_or_default() {
        let with_icon = ResultItem {
            id: "test".to_string(),
            name: "Test".to_string(),
            icon: Some("custom-icon".to_string()),
            ..Default::default()
        };
        assert_eq!(with_icon.icon_or_default(), "custom-icon");

        let without_icon = ResultItem::default();
        assert_eq!(without_icon.icon_or_default(), "extension");
    }

    #[test]
    fn test_result_item_verb_or_default() {
        let with_verb = ResultItem {
            id: "test".to_string(),
            name: "Test".to_string(),
            verb: Some("Launch".to_string()),
            ..Default::default()
        };
        assert_eq!(with_verb.verb_or_default(), "Launch");

        let without_verb = ResultItem::default();
        assert_eq!(without_verb.verb_or_default(), "Select");
    }

    #[test]
    fn test_result_item_slider_value_none() {
        let item = ResultItem::default();
        assert!(item.slider_value().is_none());
    }

    #[test]
    fn test_result_item_is_slider_is_switch() {
        // is_slider() and is_switch() are derived from widget field, not result_type
        let slider = ResultItem::default().with_slider(50.0, 0.0, 100.0, 1.0, None);
        assert!(slider.is_slider());
        assert!(!slider.is_switch());

        let switch = ResultItem::default().with_switch(true);
        assert!(!switch.is_slider());
        assert!(switch.is_switch());
    }

    // ResultPatch edge cases

    #[test]
    fn test_result_patch_minimal() {
        let patch = ResultPatch {
            id: "item-1".to_string(),
            ..Default::default()
        };
        let json = serde_json::to_value(&patch).unwrap();
        assert_eq!(json["id"], "item-1");
        // Optional fields should be absent
        assert!(json.get("name").is_none());
    }

    // PreviewData edge cases

    #[test]
    fn test_preview_data_empty() {
        let json = json!({});
        let preview: PreviewData = serde_json::from_value(json).unwrap();
        assert!(preview.title.is_none());
        assert!(preview.content.is_none());
        assert!(preview.metadata.is_empty());
    }

    // Action edge cases

    #[test]
    fn test_action_defaults() {
        let json = json!({
            "id": "action-1",
            "name": "Action"
        });
        let action: Action = serde_json::from_value(json).unwrap();
        assert_eq!(action.id, "action-1");
        assert!(!action.keep_open); // default false
    }

    // ActionSource edge cases

    #[test]
    fn test_action_source_default() {
        let source = ActionSource::default();
        assert_eq!(source, ActionSource::Normal);
    }

    #[test]
    fn test_action_source_serialization() {
        let sources = [
            ActionSource::Normal,
            ActionSource::Ambient,
            ActionSource::Fab,
        ];
        let expected = ["normal", "ambient", "fab"];
        for (source, exp) in sources.iter().zip(expected.iter()) {
            let json = serde_json::to_value(source).unwrap();
            assert_eq!(json, *exp);
        }
    }

    // Builder method tests

    #[test]
    fn test_with_slider() {
        let item = ResultItem {
            id: "volume".to_string(),
            name: "Volume".to_string(),
            ..Default::default()
        }
        .with_slider(75.0, 0.0, 100.0, 5.0, Some("75%".to_string()));

        assert!(
            item.is_slider(),
            "with_slider() should make is_slider() return true"
        );
        assert!(matches!(
            item.widget,
            Some(WidgetData::Slider {
                value: 75.0,
                min: 0.0,
                max: 100.0,
                step: 5.0,
                ..
            })
        ));
        if let Some(WidgetData::Slider { display_value, .. }) = &item.widget {
            assert_eq!(display_value, &Some("75%".to_string()));
        }
    }

    #[test]
    fn test_with_switch() {
        let item_on = ResultItem {
            id: "wifi".to_string(),
            name: "WiFi".to_string(),
            ..Default::default()
        }
        .with_switch(true);

        assert!(
            item_on.is_switch(),
            "with_switch() should make is_switch() return true"
        );
        assert!(matches!(
            item_on.widget,
            Some(WidgetData::Switch { value: true })
        ));

        let item_off = ResultItem {
            id: "bluetooth".to_string(),
            name: "Bluetooth".to_string(),
            ..Default::default()
        }
        .with_switch(false);

        assert!(
            item_off.is_switch(),
            "with_switch() should make is_switch() return true"
        );
        assert!(matches!(
            item_off.widget,
            Some(WidgetData::Switch { value: false })
        ));
    }

    #[test]
    fn test_with_gauge() {
        let item = ResultItem {
            id: "cpu".to_string(),
            name: "CPU Usage".to_string(),
            ..Default::default()
        }
        .with_gauge(GaugeData {
            value: 65.0,
            min: 0.0,
            max: 100.0,
            label: Some("CPU".to_string()),
            color: Some("#ff0000".to_string()),
        });

        assert!(matches!(
            item.widget,
            Some(WidgetData::Gauge {
                value: 65.0,
                min: 0.0,
                max: 100.0,
                ..
            })
        ));
        if let Some(WidgetData::Gauge { label, color, .. }) = &item.widget {
            assert_eq!(label, &Some("CPU".to_string()));
            assert_eq!(color, &Some("#ff0000".to_string()));
        }
    }

    #[test]
    fn test_with_progress() {
        let item = ResultItem {
            id: "download".to_string(),
            name: "Download Progress".to_string(),
            ..Default::default()
        }
        .with_progress(ProgressData {
            value: 50.0,
            max: 100.0,
            label: Some("Downloading".to_string()),
            color: Some("#00ff00".to_string()),
        });

        assert!(matches!(
            item.widget,
            Some(WidgetData::Progress {
                value: 50.0,
                max: 100.0,
                ..
            })
        ));
        if let Some(WidgetData::Progress { label, color, .. }) = &item.widget {
            assert_eq!(label, &Some("Downloading".to_string()));
            assert_eq!(color, &Some("#00ff00".to_string()));
        }
    }

    #[test]
    fn test_with_graph() {
        let item = ResultItem {
            id: "network".to_string(),
            name: "Network Activity".to_string(),
            ..Default::default()
        }
        .with_graph(GraphData {
            data: vec![10.0, 20.0, 30.0, 25.0, 35.0],
            min: Some(0.0),
            max: Some(50.0),
        });

        assert!(matches!(item.widget, Some(WidgetData::Graph { .. })));
        if let Some(WidgetData::Graph { data, min, max }) = &item.widget {
            assert_eq!(data.len(), 5);
            assert_eq!(*min, Some(0.0));
            assert_eq!(*max, Some(50.0));
        }
    }
}

/// Property-based tests using proptest for serialization round-trips.
///
/// These tests verify that types can survive JSON serialization and deserialization
/// without data loss, catching edge cases that hand-written tests might miss.
#[cfg(test)]
mod proptest_roundtrip_tests {
    use super::*;
    use proptest::prelude::*;

    /// Tolerance for floating-point comparisons after JSON round-trip.
    /// JSON's decimal representation can introduce small precision differences.
    const FLOAT_TOLERANCE: f64 = 1e-10;

    /// Compare two f64 values with tolerance for JSON round-trip precision loss.
    fn floats_approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < FLOAT_TOLERANCE || (a - b).abs() / a.abs().max(b.abs()) < FLOAT_TOLERANCE
    }

    /// Compare two optional f64 values with tolerance.
    fn opt_floats_approx_eq(a: Option<f64>, b: Option<f64>) -> bool {
        match (a, b) {
            (None, None) => true,
            (Some(x), Some(y)) => floats_approx_eq(x, y),
            _ => false,
        }
    }

    /// Compare two Vec<f64> with tolerance.
    fn vec_floats_approx_eq(a: &[f64], b: &[f64]) -> bool {
        a.len() == b.len()
            && a.iter()
                .zip(b.iter())
                .all(|(x, y)| floats_approx_eq(*x, *y))
    }

    /// Generate arbitrary strings that are valid for JSON (non-control characters).
    fn arb_json_string() -> impl Strategy<Value = String> {
        proptest::string::string_regex("[a-zA-Z0-9_\\-. ]{0,100}")
            .unwrap()
            .boxed()
    }

    /// Generate arbitrary optional strings.
    fn arb_opt_string() -> impl Strategy<Value = Option<String>> {
        proptest::option::of(arb_json_string())
    }

    /// Generate finite f64 values (no NaN, no infinities).
    fn arb_finite_f64() -> impl Strategy<Value = f64> {
        (-1e10f64..1e10f64).prop_filter("must be finite", |x| x.is_finite())
    }

    /// Generate positive finite f64 values.
    fn arb_positive_f64() -> impl Strategy<Value = f64> {
        (0.0f64..1e10f64).prop_filter("must be finite", |x| x.is_finite())
    }

    // === WidgetData round-trips ===

    prop_compose! {
        fn arb_slider_widget()(
            value in arb_finite_f64(),
            min in arb_finite_f64(),
            max in arb_finite_f64(),
            step in arb_positive_f64().prop_map(|v| if v == 0.0 { 1.0 } else { v }),
            display_value in arb_opt_string()
        ) -> WidgetData {
            WidgetData::Slider { value, min, max, step, display_value }
        }
    }

    prop_compose! {
        fn arb_switch_widget()(value: bool) -> WidgetData {
            WidgetData::Switch { value }
        }
    }

    prop_compose! {
        fn arb_gauge_widget()(
            value in arb_finite_f64(),
            min in arb_finite_f64(),
            max in arb_finite_f64(),
            label in arb_opt_string(),
            color in arb_opt_string()
        ) -> WidgetData {
            WidgetData::Gauge { value, min, max, label, color }
        }
    }

    prop_compose! {
        fn arb_progress_widget()(
            value in arb_finite_f64(),
            max in arb_finite_f64(),
            label in arb_opt_string(),
            color in arb_opt_string()
        ) -> WidgetData {
            WidgetData::Progress { value, max, label, color }
        }
    }

    prop_compose! {
        fn arb_graph_widget()(
            data in proptest::collection::vec(arb_finite_f64(), 0..20),
            min in proptest::option::of(arb_finite_f64()),
            max in proptest::option::of(arb_finite_f64())
        ) -> WidgetData {
            WidgetData::Graph { data, min, max }
        }
    }

    fn arb_widget_data() -> impl Strategy<Value = WidgetData> {
        prop_oneof![
            arb_slider_widget(),
            arb_switch_widget(),
            arb_gauge_widget(),
            arb_progress_widget(),
            arb_graph_widget(),
        ]
    }

    /// Compare two `WidgetData` values with float tolerance.
    fn widgets_approx_eq(a: &WidgetData, b: &WidgetData) -> bool {
        match (a, b) {
            (
                WidgetData::Slider {
                    value: v1,
                    min: min1,
                    max: max1,
                    step: step1,
                    display_value: dv1,
                },
                WidgetData::Slider {
                    value: v2,
                    min: min2,
                    max: max2,
                    step: step2,
                    display_value: dv2,
                },
            ) => {
                floats_approx_eq(*v1, *v2)
                    && floats_approx_eq(*min1, *min2)
                    && floats_approx_eq(*max1, *max2)
                    && floats_approx_eq(*step1, *step2)
                    && dv1 == dv2
            }
            (WidgetData::Switch { value: v1 }, WidgetData::Switch { value: v2 }) => v1 == v2,
            (
                WidgetData::Gauge {
                    value: v1,
                    min: min1,
                    max: max1,
                    label: l1,
                    color: c1,
                },
                WidgetData::Gauge {
                    value: v2,
                    min: min2,
                    max: max2,
                    label: l2,
                    color: c2,
                },
            ) => {
                floats_approx_eq(*v1, *v2)
                    && floats_approx_eq(*min1, *min2)
                    && floats_approx_eq(*max1, *max2)
                    && l1 == l2
                    && c1 == c2
            }
            (
                WidgetData::Progress {
                    value: v1,
                    max: max1,
                    label: l1,
                    color: c1,
                },
                WidgetData::Progress {
                    value: v2,
                    max: max2,
                    label: l2,
                    color: c2,
                },
            ) => {
                floats_approx_eq(*v1, *v2) && floats_approx_eq(*max1, *max2) && l1 == l2 && c1 == c2
            }
            (
                WidgetData::Graph {
                    data: d1,
                    min: min1,
                    max: max1,
                },
                WidgetData::Graph {
                    data: d2,
                    min: min2,
                    max: max2,
                },
            ) => {
                vec_floats_approx_eq(d1, d2)
                    && opt_floats_approx_eq(*min1, *min2)
                    && opt_floats_approx_eq(*max1, *max2)
            }
            _ => false,
        }
    }

    /// Compare two optional `WidgetData` values with float tolerance.
    fn opt_widgets_approx_eq(a: Option<&WidgetData>, b: Option<&WidgetData>) -> bool {
        match (a, b) {
            (None, None) => true,
            (Some(w1), Some(w2)) => widgets_approx_eq(w1, w2),
            _ => false,
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(500))]

        #[test]
        fn widget_data_roundtrip(widget in arb_widget_data()) {
            let json = serde_json::to_string(&widget).unwrap();
            let parsed: WidgetData = serde_json::from_str(&json).unwrap();
            prop_assert!(widgets_approx_eq(&widget, &parsed), "Widget mismatch: {:?} vs {:?}", widget, parsed);
        }
    }

    // === Badge round-trips ===

    prop_compose! {
        fn arb_badge()(
            text in arb_opt_string(),
            icon in arb_opt_string(),
            color in arb_opt_string()
        ) -> Badge {
            Badge { text, icon, color }
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(200))]

        #[test]
        fn badge_roundtrip(badge in arb_badge()) {
            let json = serde_json::to_string(&badge).unwrap();
            let parsed: Badge = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(badge.text, parsed.text);
            prop_assert_eq!(badge.icon, parsed.icon);
            prop_assert_eq!(badge.color, parsed.color);
        }
    }

    // === Chip round-trips ===

    prop_compose! {
        fn arb_chip()(
            text in arb_json_string(),
            icon in arb_opt_string(),
            color in arb_opt_string()
        ) -> Chip {
            Chip { text, icon, color }
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(200))]

        #[test]
        fn chip_roundtrip(chip in arb_chip()) {
            let json = serde_json::to_string(&chip).unwrap();
            let parsed: Chip = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(chip.text, parsed.text);
            prop_assert_eq!(chip.icon, parsed.icon);
            prop_assert_eq!(chip.color, parsed.color);
        }
    }

    // === Action round-trips ===

    prop_compose! {
        fn arb_action()(
            id in arb_json_string().prop_filter("non-empty", |s| !s.is_empty()),
            name in arb_json_string(),
            icon in arb_opt_string(),
            icon_type in arb_opt_string(),
            keep_open: bool
        ) -> Action {
            Action { id, name, icon, icon_type, keep_open }
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(200))]

        #[test]
        fn action_roundtrip(action in arb_action()) {
            let json = serde_json::to_string(&action).unwrap();
            let parsed: Action = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(action.id, parsed.id);
            prop_assert_eq!(action.name, parsed.name);
            prop_assert_eq!(action.icon, parsed.icon);
            prop_assert_eq!(action.icon_type, parsed.icon_type);
            prop_assert_eq!(action.keep_open, parsed.keep_open);
        }
    }

    // === ResultItem round-trips ===

    fn arb_result_type() -> impl Strategy<Value = ResultType> {
        prop_oneof![
            Just(ResultType::Normal),
            Just(ResultType::App),
            Just(ResultType::Plugin),
            Just(ResultType::IndexedItem),
            Just(ResultType::WebSearch),
            Just(ResultType::Suggestion),
            Just(ResultType::Recent),
            Just(ResultType::PatternMatch),
        ]
    }

    fn arb_display_hint() -> impl Strategy<Value = DisplayHint> {
        prop_oneof![
            Just(DisplayHint::Auto),
            Just(DisplayHint::List),
            Just(DisplayHint::Grid),
            Just(DisplayHint::LargeGrid),
        ]
    }

    prop_compose! {
        fn arb_result_item_basic()(
            id in arb_json_string().prop_filter("non-empty", |s| !s.is_empty()),
            name in arb_json_string(),
            description in arb_opt_string(),
            icon in arb_opt_string(),
            icon_type in arb_opt_string(),
            verb in arb_opt_string(),
            result_type in arb_result_type(),
            keep_open: bool,
            is_suggestion: bool,
            has_ocr: bool,
            display_hint in proptest::option::of(arb_display_hint()),
            widget in proptest::option::of(arb_widget_data())
        ) -> ResultItem {
            ResultItem {
                id,
                name,
                description,
                icon,
                icon_type,
                verb,
                result_type,
                badges: Vec::new(),
                chips: Vec::new(),
                thumbnail: None,
                preview: None,
                actions: Vec::new(),
                plugin_id: None,
                app_id: None,
                app_id_fallback: None,
                keywords: None,
                entry_point: None,
                keep_open,
                is_suggestion,
                suggestion_reason: None,
                has_ocr,
                display_hint,
                widget,
                open_url: None,
                copy: None,
                notify: None,
                should_close: None,
                composite_score: 0.0,
            }
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(300))]

        #[test]
        fn result_item_roundtrip(item in arb_result_item_basic()) {
            let json = serde_json::to_string(&item).unwrap();
            let parsed: ResultItem = serde_json::from_str(&json).unwrap();

            prop_assert_eq!(&item.id, &parsed.id);
            prop_assert_eq!(&item.name, &parsed.name);
            prop_assert_eq!(&item.description, &parsed.description);
            prop_assert_eq!(&item.icon, &parsed.icon);
            prop_assert_eq!(&item.icon_type, &parsed.icon_type);
            prop_assert_eq!(&item.verb, &parsed.verb);
            prop_assert_eq!(item.result_type, parsed.result_type);
            prop_assert_eq!(item.keep_open, parsed.keep_open);
            prop_assert_eq!(item.is_suggestion, parsed.is_suggestion);
            prop_assert_eq!(item.has_ocr, parsed.has_ocr);
            prop_assert_eq!(&item.display_hint, &parsed.display_hint);
            prop_assert!(opt_widgets_approx_eq(item.widget.as_ref(), parsed.widget.as_ref()), "Widget mismatch");
        }
    }

    // === GaugeData/ProgressData/GraphData round-trips ===

    prop_compose! {
        fn arb_gauge_data()(
            value in arb_finite_f64(),
            min in arb_finite_f64(),
            max in arb_finite_f64(),
            label in arb_opt_string(),
            color in arb_opt_string()
        ) -> GaugeData {
            GaugeData { value, min, max, label, color }
        }
    }

    prop_compose! {
        fn arb_progress_data()(
            value in arb_finite_f64(),
            max in arb_finite_f64(),
            label in arb_opt_string(),
            color in arb_opt_string()
        ) -> ProgressData {
            ProgressData { value, max, label, color }
        }
    }

    prop_compose! {
        fn arb_graph_data()(
            data in proptest::collection::vec(arb_finite_f64(), 0..50),
            min in proptest::option::of(arb_finite_f64()),
            max in proptest::option::of(arb_finite_f64())
        ) -> GraphData {
            GraphData { data, min, max }
        }
    }

    /// Compare two `GaugeData` values with float tolerance.
    fn gauge_approx_eq(a: &GaugeData, b: &GaugeData) -> bool {
        floats_approx_eq(a.value, b.value)
            && floats_approx_eq(a.min, b.min)
            && floats_approx_eq(a.max, b.max)
            && a.label == b.label
            && a.color == b.color
    }

    /// Compare two `ProgressData` values with float tolerance.
    fn progress_approx_eq(a: &ProgressData, b: &ProgressData) -> bool {
        floats_approx_eq(a.value, b.value)
            && floats_approx_eq(a.max, b.max)
            && a.label == b.label
            && a.color == b.color
    }

    /// Compare two `GraphData` values with float tolerance.
    fn graph_approx_eq(a: &GraphData, b: &GraphData) -> bool {
        vec_floats_approx_eq(&a.data, &b.data)
            && opt_floats_approx_eq(a.min, b.min)
            && opt_floats_approx_eq(a.max, b.max)
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(200))]

        #[test]
        fn gauge_data_roundtrip(gauge in arb_gauge_data()) {
            let json = serde_json::to_string(&gauge).unwrap();
            let parsed: GaugeData = serde_json::from_str(&json).unwrap();
            prop_assert!(gauge_approx_eq(&gauge, &parsed), "Gauge mismatch: {:?} vs {:?}", gauge, parsed);
        }

        #[test]
        fn progress_data_roundtrip(progress in arb_progress_data()) {
            let json = serde_json::to_string(&progress).unwrap();
            let parsed: ProgressData = serde_json::from_str(&json).unwrap();
            prop_assert!(progress_approx_eq(&progress, &parsed), "Progress mismatch: {:?} vs {:?}", progress, parsed);
        }

        #[test]
        fn graph_data_roundtrip(graph in arb_graph_data()) {
            let json = serde_json::to_string(&graph).unwrap();
            let parsed: GraphData = serde_json::from_str(&json).unwrap();
            prop_assert!(graph_approx_eq(&graph, &parsed), "Graph mismatch: {:?} vs {:?}", graph, parsed);
        }
    }

    // === Frecency round-trips ===

    prop_compose! {
        fn arb_frecency()(
            count in 0u32..1000,
            last_used in 0u64..u64::MAX/2,
            recent_search_terms in proptest::collection::vec(arb_json_string(), 0..5),
            consecutive_days in 0u32..365,
            last_consecutive_date in arb_opt_string(),
            launch_from_empty_count in 0u32..100,
            session_start_count in 0u32..100,
            resume_from_idle_count in 0u32..100
        ) -> Frecency {
            Frecency {
                count,
                last_used,
                recent_search_terms,
                consecutive_days,
                last_consecutive_date,
                launch_from_empty_count,
                session_start_count,
                resume_from_idle_count,
                ..Default::default()
            }
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(200))]

        #[test]
        fn frecency_roundtrip(frecency in arb_frecency()) {
            let json = serde_json::to_string(&frecency).unwrap();
            let parsed: Frecency = serde_json::from_str(&json).unwrap();

            prop_assert_eq!(frecency.count, parsed.count);
            prop_assert_eq!(frecency.last_used, parsed.last_used);
            prop_assert_eq!(&frecency.recent_search_terms, &parsed.recent_search_terms);
            prop_assert_eq!(frecency.consecutive_days, parsed.consecutive_days);
            prop_assert_eq!(&frecency.last_consecutive_date, &parsed.last_consecutive_date);
        }
    }

    // === CoreEvent round-trips (selected variants) ===

    fn arb_core_event() -> impl Strategy<Value = CoreEvent> {
        prop_oneof![
            arb_json_string().prop_map(|query| CoreEvent::QueryChanged { query }),
            (arb_json_string(), arb_opt_string())
                .prop_map(|(query, context)| CoreEvent::QuerySubmitted { query, context }),
            (
                arb_json_string().prop_filter("non-empty", |s| !s.is_empty()),
                arb_opt_string(),
                arb_opt_string()
            )
                .prop_map(|(id, action, plugin_id)| CoreEvent::ItemSelected {
                    id,
                    action,
                    plugin_id
                }),
            Just(CoreEvent::Back),
            Just(CoreEvent::Cancel),
            Just(CoreEvent::LauncherOpened),
            Just(CoreEvent::LauncherClosed),
            Just(CoreEvent::FormCancelled),
            arb_json_string()
                .prop_filter("non-empty", |s| !s.is_empty())
                .prop_map(|plugin_id| CoreEvent::OpenPlugin { plugin_id }),
            Just(CoreEvent::ClosePlugin),
            (arb_json_string(), arb_finite_f64(), arb_opt_string()).prop_map(
                |(id, value, plugin_id)| CoreEvent::SliderChanged {
                    id,
                    value,
                    plugin_id
                }
            ),
            (arb_json_string(), any::<bool>(), arb_opt_string()).prop_map(
                |(id, value, plugin_id)| CoreEvent::SwitchToggled {
                    id,
                    value,
                    plugin_id
                }
            ),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(300))]

        #[test]
        fn core_event_roundtrip(event in arb_core_event()) {
            let json = serde_json::to_string(&event).unwrap();
            let parsed: CoreEvent = serde_json::from_str(&json).unwrap();

            // Verify key fields based on variant
            match (&event, &parsed) {
                (
                    CoreEvent::QueryChanged { query: q1 },
                    CoreEvent::QueryChanged { query: q2 },
                ) => prop_assert_eq!(q1, q2),
                (
                    CoreEvent::QuerySubmitted { query: q1, context: c1 },
                    CoreEvent::QuerySubmitted { query: q2, context: c2 },
                ) => {
                    prop_assert_eq!(q1, q2);
                    prop_assert_eq!(c1, c2);
                }
                (
                    CoreEvent::ItemSelected { id: i1, action: a1, plugin_id: p1 },
                    CoreEvent::ItemSelected { id: i2, action: a2, plugin_id: p2 },
                ) => {
                    prop_assert_eq!(i1, i2);
                    prop_assert_eq!(a1, a2);
                    prop_assert_eq!(p1, p2);
                }
                (
                    CoreEvent::SliderChanged { id: i1, value: v1, plugin_id: p1 },
                    CoreEvent::SliderChanged { id: i2, value: v2, plugin_id: p2 },
                ) => {
                    prop_assert_eq!(i1, i2);
                    prop_assert!(floats_approx_eq(*v1, *v2), "Slider value mismatch: {} vs {}", v1, v2);
                    prop_assert_eq!(p1, p2);
                }
                (
                    CoreEvent::SwitchToggled { id: i1, value: v1, plugin_id: p1 },
                    CoreEvent::SwitchToggled { id: i2, value: v2, plugin_id: p2 },
                ) => {
                    prop_assert_eq!(i1, i2);
                    prop_assert_eq!(v1, v2);
                    prop_assert_eq!(p1, p2);
                }
                (CoreEvent::Back, CoreEvent::Back)
                | (CoreEvent::Cancel, CoreEvent::Cancel)
                | (CoreEvent::LauncherOpened, CoreEvent::LauncherOpened)
                | (CoreEvent::LauncherClosed, CoreEvent::LauncherClosed)
                | (CoreEvent::FormCancelled, CoreEvent::FormCancelled)
                | (CoreEvent::ClosePlugin, CoreEvent::ClosePlugin) => {}
                (
                    CoreEvent::OpenPlugin { plugin_id: p1 },
                    CoreEvent::OpenPlugin { plugin_id: p2 },
                ) => prop_assert_eq!(p1, p2),
                _ => prop_assert!(false, "Variant mismatch: {:?} vs {:?}", event, parsed),
            }
        }
    }

    // === FormField round-trips ===

    fn arb_form_field_type() -> impl Strategy<Value = FormFieldType> {
        prop_oneof![
            Just(FormFieldType::Text),
            Just(FormFieldType::Password),
            Just(FormFieldType::Number),
            Just(FormFieldType::TextArea),
            Just(FormFieldType::Select),
            Just(FormFieldType::Checkbox),
            Just(FormFieldType::Switch),
            Just(FormFieldType::Slider),
            Just(FormFieldType::Hidden),
            Just(FormFieldType::Date),
            Just(FormFieldType::Time),
            Just(FormFieldType::Email),
            Just(FormFieldType::Url),
            Just(FormFieldType::Phone),
        ]
    }

    prop_compose! {
        fn arb_form_field()(
            id in arb_json_string().prop_filter("non-empty", |s| !s.is_empty()),
            label in arb_json_string(),
            field_type in arb_form_field_type(),
            placeholder in arb_opt_string(),
            default_value in arb_opt_string(),
            required: bool,
            hint in arb_opt_string()
        ) -> FormField {
            FormField {
                id,
                label,
                field_type,
                placeholder,
                default_value,
                required,
                options: Vec::new(),
                hint,
                rows: None,
                min: None,
                max: None,
                step: None,
            }
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(200))]

        #[test]
        fn form_field_roundtrip(field in arb_form_field()) {
            let json = serde_json::to_string(&field).unwrap();
            let parsed: FormField = serde_json::from_str(&json).unwrap();

            prop_assert_eq!(&field.id, &parsed.id);
            prop_assert_eq!(&field.label, &parsed.label);
            prop_assert_eq!(field.field_type, parsed.field_type);
            prop_assert_eq!(&field.placeholder, &parsed.placeholder);
            prop_assert_eq!(&field.default_value, &parsed.default_value);
            prop_assert_eq!(field.required, parsed.required);
            prop_assert_eq!(&field.hint, &parsed.hint);
        }
    }

    // === ExecuteAction round-trips ===

    fn arb_execute_action() -> impl Strategy<Value = ExecuteAction> {
        prop_oneof![
            arb_json_string().prop_map(|desktop_file| ExecuteAction::Launch { desktop_file }),
            arb_json_string().prop_map(|url| ExecuteAction::OpenUrl { url }),
            arb_json_string().prop_map(|path| ExecuteAction::Open { path }),
            arb_json_string().prop_map(|text| ExecuteAction::Copy { text }),
            arb_json_string().prop_map(|text| ExecuteAction::TypeText { text }),
            arb_json_string().prop_map(|sound| ExecuteAction::PlaySound { sound }),
            arb_json_string().prop_map(|message| ExecuteAction::Notify { message }),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(200))]

        #[test]
        fn execute_action_roundtrip(action in arb_execute_action()) {
            let json = serde_json::to_string(&action).unwrap();
            let parsed: ExecuteAction = serde_json::from_str(&json).unwrap();

            match (&action, &parsed) {
                (
                    ExecuteAction::Launch { desktop_file: d1 },
                    ExecuteAction::Launch { desktop_file: d2 },
                ) => prop_assert_eq!(d1, d2),
                (ExecuteAction::OpenUrl { url: u1 }, ExecuteAction::OpenUrl { url: u2 }) => {
                    prop_assert_eq!(u1, u2);
                }
                (ExecuteAction::Open { path: p1 }, ExecuteAction::Open { path: p2 }) => {
                    prop_assert_eq!(p1, p2);
                }
                (ExecuteAction::Copy { text: t1 }, ExecuteAction::Copy { text: t2 }) => {
                    prop_assert_eq!(t1, t2);
                }
                (ExecuteAction::TypeText { text: t1 }, ExecuteAction::TypeText { text: t2 }) => {
                    prop_assert_eq!(t1, t2);
                }
                (
                    ExecuteAction::PlaySound { sound: s1 },
                    ExecuteAction::PlaySound { sound: s2 },
                ) => prop_assert_eq!(s1, s2),
                (
                    ExecuteAction::Notify { message: m1 },
                    ExecuteAction::Notify { message: m2 },
                ) => prop_assert_eq!(m1, m2),
                _ => prop_assert!(false, "Variant mismatch"),
            }
        }
    }
}
