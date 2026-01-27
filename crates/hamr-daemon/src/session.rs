//! Session management types for client connections.

use hamr_rpc::protocol::ClientRole;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(String);

impl SessionId {
    #[must_use]
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for SessionId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for SessionId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

#[derive(Debug, Clone)]
pub struct ClientInfo {
    pub id: SessionId,
    pub role: Option<ClientRole>,
    pub registered: bool,
}

impl ClientInfo {
    #[must_use]
    pub fn new() -> Self {
        Self {
            id: SessionId::new(),
            role: None,
            registered: false,
        }
    }

    #[must_use]
    pub fn with_id(id: SessionId) -> Self {
        Self {
            id,
            role: None,
            registered: false,
        }
    }

    pub fn register(&mut self, role: ClientRole) {
        self.role = Some(role);
        self.registered = true;
    }
}

impl Default for ClientInfo {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub enum Session {
    Pending(ClientInfo),
    Ui(UiSession),
    Control(ControlSession),
    Plugin(PluginSession),
}

impl Session {
    #[must_use]
    pub fn id(&self) -> &SessionId {
        match self {
            Session::Pending(info) => &info.id,
            Session::Ui(s) => &s.info.id,
            Session::Control(s) => &s.info.id,
            Session::Plugin(s) => &s.info.id,
        }
    }

    #[must_use]
    pub fn is_registered(&self) -> bool {
        !matches!(self, Session::Pending(_))
    }

    #[must_use]
    pub fn is_ui(&self) -> bool {
        matches!(self, Session::Ui(_))
    }

    #[must_use]
    pub fn is_control(&self) -> bool {
        matches!(self, Session::Control(_))
    }

    #[must_use]
    pub fn ui_name(&self) -> Option<&str> {
        match self {
            Session::Ui(s) => Some(&s.name),
            _ => None,
        }
    }

    #[must_use]
    pub fn plugin_id(&self) -> Option<&str> {
        match self {
            Session::Plugin(s) => Some(&s.plugin_id),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct UiSession {
    pub info: ClientInfo,
    pub name: String,
}

impl UiSession {
    #[must_use]
    pub fn new(info: ClientInfo, name: String) -> Self {
        Self { info, name }
    }
}

#[derive(Debug, Clone)]
pub struct ControlSession {
    pub info: ClientInfo,
}

impl ControlSession {
    #[must_use]
    pub fn new(info: ClientInfo) -> Self {
        Self { info }
    }
}

#[derive(Debug, Clone)]
pub struct PluginSession {
    pub info: ClientInfo,
    pub plugin_id: String,
}

impl PluginSession {
    #[must_use]
    pub fn new(info: ClientInfo, plugin_id: String) -> Self {
        Self { info, plugin_id }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_id_unique() {
        let id1 = SessionId::new();
        let id2 = SessionId::new();
        assert_ne!(id1, id2, "Each new ID should be unique");
    }

    #[test]
    fn test_session_id_from_string() {
        let id: SessionId = "test-session".to_string().into();
        assert_eq!(format!("{id}"), "test-session");
    }

    #[test]
    fn test_session_id_from_str() {
        let id: SessionId = "my-session".into();
        assert_eq!(format!("{id}"), "my-session");
    }

    #[test]
    fn test_session_id_equality() {
        let s1: SessionId = "abc".into();
        let s2: SessionId = "abc".into();
        let s3: SessionId = "xyz".into();
        assert_eq!(s1, s2);
        assert_ne!(s1, s3);
    }

    #[test]
    fn test_session_id_default() {
        let id = SessionId::default();
        assert!(!format!("{id}").is_empty());
    }

    #[test]
    fn test_client_info_new() {
        let info = ClientInfo::new();
        assert!(!info.registered);
        assert!(info.role.is_none());
    }

    #[test]
    fn test_client_info_with_id() {
        let id: SessionId = "custom-id".into();
        let info = ClientInfo::with_id(id.clone());
        assert_eq!(info.id, id);
        assert!(!info.registered);
    }

    #[test]
    fn test_client_info_register() {
        let mut info = ClientInfo::new();
        assert!(!info.registered);
        assert!(info.role.is_none());

        info.register(ClientRole::Control);

        assert!(info.registered);
        assert!(matches!(info.role, Some(ClientRole::Control)));
    }

    #[test]
    fn test_session_pending() {
        let info = ClientInfo::new();
        let session = Session::Pending(info.clone());

        assert!(!session.is_registered());
        assert_eq!(session.id(), &info.id);
    }

    #[test]
    fn test_session_ui() {
        let mut info = ClientInfo::new();
        info.register(ClientRole::Ui {
            name: "gtk".to_string(),
        });
        let ui_session = UiSession::new(info.clone(), "gtk".to_string());
        let session = Session::Ui(ui_session);

        assert!(session.is_registered());
        assert!(session.is_ui());
        assert!(!session.is_control());
        assert_eq!(session.ui_name(), Some("gtk"));
        assert!(session.plugin_id().is_none());
    }

    #[test]
    fn test_session_control() {
        let mut info = ClientInfo::new();
        info.register(ClientRole::Control);
        let control_session = ControlSession::new(info);
        let session = Session::Control(control_session);

        assert!(session.is_registered());
        assert!(!session.is_ui());
        assert!(session.is_control());
        assert!(session.ui_name().is_none());
        assert!(session.plugin_id().is_none());
    }

    #[test]
    fn test_session_plugin() {
        let mut info = ClientInfo::new();
        let manifest = hamr_rpc::PluginManifest {
            id: "notes".to_string(),
            name: "Notes".to_string(),
            description: None,
            icon: None,
            prefix: None,
            priority: 0,
        };
        info.register(ClientRole::Plugin {
            id: "notes".to_string(),
            manifest,
        });
        let plugin_session = PluginSession::new(info, "notes".to_string());
        let session = Session::Plugin(plugin_session);

        assert!(session.is_registered());
        assert!(!session.is_ui());
        assert!(!session.is_control());
        assert!(session.ui_name().is_none());
        assert_eq!(session.plugin_id(), Some("notes"));
    }

    #[test]
    fn test_ui_session_new() {
        let info = ClientInfo::new();
        let session = UiSession::new(info.clone(), "tui".to_string());
        assert_eq!(session.name, "tui");
        assert_eq!(session.info.id, info.id);
    }

    #[test]
    fn test_control_session_new() {
        let info = ClientInfo::new();
        let session = ControlSession::new(info.clone());
        assert_eq!(session.info.id, info.id);
    }

    #[test]
    fn test_plugin_session_new() {
        let info = ClientInfo::new();
        let session = PluginSession::new(info.clone(), "apps".to_string());
        assert_eq!(session.plugin_id, "apps");
        assert_eq!(session.info.id, info.id);
    }
}
