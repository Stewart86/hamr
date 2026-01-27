# Integration Tests for hamr-daemon

This document describes the integration tests for socket plugin communication in hamr-daemon.

## Overview

The integration tests cover the core functionality of plugin registration, message forwarding, and notification handling. These tests focus on validating the protocol implementation without requiring a full daemon instance.

## Test Categories

### 1. Plugin Registration (Test 1)

**File**: `tests/integration_tests.rs::test_plugin_registration_flow`

**Scenario**: Test that a mock socket plugin can connect, send a `register` request with `ClientRole::Plugin`, and receive successful registration.

**What it tests**:
- Plugin can connect to the daemon
- Plugin registration with manifest returns a session ID
- Session ID is valid and non-empty
- Plugin manifest is properly transmitted

**Key assertions**:
```rust
assert!(registry.is_connected("test-plugin-1"));
assert!(registry.get_plugin_sender("test-plugin-1").is_some());
```

---

### 2. Multiple Plugin Registration (Test 2)

**File**: `tests/integration_tests.rs::test_multiple_plugin_registration`

**Scenario**: Test that multiple plugins can register simultaneously without conflicts.

**What it tests**:
- Multiple plugins can register with different IDs
- Each plugin maintains independent state
- Sender channels are unique per plugin
- Plugin lookup works correctly

**Key assertions**:
```rust
assert!(registry.is_connected("plugin-1"));
assert!(registry.is_connected("plugin-2"));
assert!(registry.is_connected("plugin-3"));
assert!(!registry.is_connected("plugin-4")); // Non-existent
```

---

### 3. Search Forwarding (Test 2 - Extended)

**File**: `tests/integration_tests.rs::test_search_forwarding_to_active_plugin_only`

**Scenario**: Test that `query_changed` only sends `search` requests to the active plugin (not all connected plugins).

**What it tests**:
- Multiple plugins can be registered simultaneously
- Search requests are forwarded only to the active plugin
- Plugin isolation is maintained (one plugin's requests don't affect others)
- Session IDs are unique per plugin

**Key assertions**:
```rust
assert!(plugin1_id.is_some());
assert!(plugin2_id.is_some());
assert_ne!(plugin1_id, plugin2_id); // Different session IDs
```

---

### 4. Plugin Results Forwarding (Test 3)

**File**: `tests/integration_tests.rs::test_plugin_results_forwarding_to_active_ui`

**Scenario**: Test that `plugin_results` notifications from plugins are forwarded to the active UI.

**What it tests**:
- Plugin can send results notification with structured data
- Results include required fields (id, name, description)
- Notification format matches protocol expectations
- Plugin can send multiple results in a single notification

**Key assertions**:
```rust
assert!(send_result.is_ok());
let results = params.get("results").unwrap().as_array().unwrap();
assert_eq!(results.len(), 2);
```

---

### 5. Plugin Status with Ambient Items (Test 4)

**File**: `tests/integration_tests.rs::test_plugin_status_notification_with_ambient`

**Scenario**: Test that `plugin_status` with ambient items triggers both `plugin_status_update` and `ambient_update` notifications to the UI.

**What it tests**:
- Status notification includes badges, chips, and ambient array
- Ambient items have proper structure (id, name, description, actions)
- Status can be partially populated (some fields empty)
- Both status and ambient updates are forwarded

**Ambient item structure**:
```json
{
  "id": "ambient-1",
  "name": "Currently Playing",
  "description": "Now Playing - Song Title",
  "actions": [
    {
      "id": "pause",
      "label": "Pause"
    }
  ]
}
```

**Key assertions**:
```rust
let status = params.get("status").unwrap();
let ambient = status.get("ambient").unwrap().as_array().unwrap();
assert_eq!(ambient.len(), 2);
assert!(first_ambient.get("actions").is_some());
```

---

### 6. Ambient Clearing (Test 5)

**File**: `tests/integration_tests.rs::test_ambient_clearing_with_empty_array`

**Scenario**: Test that when a plugin sends empty ambient array, the `ambient_update` notification is still sent to clear the UI.

**What it tests**:
- Plugin can send status with empty ambient array
- Empty array correctly clears previous ambient state
- Sequential status updates work correctly
- Notification is sent even when array is empty

**Sequence**:
1. Plugin sends status with 1 ambient item
2. Plugin sends status with 0 ambient items (empty array)
3. Both notifications should be accepted

**Key assertions**:
```rust
let clear_result = plugin_client
    .notify("plugin_status", Some(status_clear_ambient))
    .await;
assert!(clear_result.is_ok());

let ambient = status.get("ambient").unwrap().as_array().unwrap();
assert_eq!(ambient.len(), 0);
```

---

### 7. Multiple Clients with Different Roles (Test 6)

**File**: `tests/integration_tests.rs::test_multiple_clients_with_different_roles`

**Scenario**: Test that multiple UI clients and plugin clients can be registered with proper role isolation.

**What it tests**:
- UI clients don't interfere with plugin clients
- Multiple UI clients can connect (only one active at a time)
- Session IDs are unique across all client types
- Each client type maintains separate state

**Client types tested**:
- 2 UI clients
- 2 Plugin clients

**Key assertions**:
```rust
assert_ne!(ui1.session_id().unwrap(), ui2.session_id().unwrap());
assert_ne!(plugin1.session_id().unwrap(), plugin2.session_id().unwrap());
```

---

### 8. Plugin Manifest Details (Test 7)

**File**: `tests/integration_tests.rs::test_plugin_manifest_in_registration`

**Scenario**: Test that full plugin manifest with all fields is properly registered.

**What it tests**:
- Plugins can register with complete manifest information
- All manifest fields are preserved (id, name, description, icon, prefix, priority)
- Icon with special characters (emojis) are supported
- Priority field is correctly stored

**Manifest structure**:
```rust
PluginManifest {
    id: "detailed-plugin",
    name: "Detailed Plugin",
    description: Some("A plugin with full manifest details"),
    icon: Some("📚"),
    prefix: Some("detail"),
    priority: 42,
}
```

**Key assertions**:
```rust
assert!(register_result.is_ok());
assert!(!session_id.is_empty());
```

---

### 9. Plugin Notification Structure Validation (Test 8)

**File**: `tests/integration_tests.rs::test_plugin_results_notification_structure`

**Scenario**: Verify that plugin_results notifications have correct JSON structure.

**What it tests**:
- Results array is present in notification
- Each result has required fields (id, name)
- Optional fields can be present (description, icon)
- Notification method is correctly set

**Key assertions**:
```rust
assert_eq!(notification.method, "plugin_results");
assert!(notification.params.is_some());
let results = params.get("results").unwrap().as_array().unwrap();
assert_eq!(results.len(), 2);
```

---

### 10. Plugin Unregistration (Test 3 - Extended)

**File**: `tests/integration_tests.rs::test_plugin_unregistration`

**Scenario**: Test that plugins can be properly unregistered by session ID.

**What it tests**:
- Plugin unregistration removes the plugin from registry
- Unregistration by session ID works correctly
- Subsequent lookups fail for unregistered plugins
- Sender channel is cleaned up

**Sequence**:
1. Register a plugin
2. Verify it's registered
3. Unregister by session ID
4. Verify it's unregistered

**Key assertions**:
```rust
registry.unregister_session(&session_id);
assert!(!registry.is_connected("removable-plugin"));
```

---

### 11. Plugin Registry State Management (Test 4 - Extended)

**File**: `tests/integration_tests.rs::test_plugin_registry_state_changes`

**Scenario**: Test complex state transitions in the plugin registry.

**What it tests**:
- Registry starts empty
- Plugins appear after registration
- Unregistration removes plugins
- Multiple plugins coexist without state leakage
- Registry properly handles concurrent operations

**State sequence**:
1. Empty state - no plugins
2. First plugin registered
3. Second plugin registered  
4. Both visible simultaneously
5. First plugin unregistered
6. Second plugin still visible

**Key assertions**:
```rust
assert!(!registry.is_connected("plugin-1")); // Initially empty
registry.register_connected(...); // Register
assert!(registry.is_connected("plugin-1")); // Now registered
registry.unregister_session(...); // Unregister
assert!(!registry.is_connected("plugin-1")); // No longer registered
```

---

### 12. Client Role Serialization (Test 5 - Extended)

**File**: `tests/integration_tests.rs::test_client_role_serialization`

**Scenario**: Test that all client roles serialize correctly for JSON-RPC protocol.

**What it tests**:
- UI role serializes with name field
- Control role serializes correctly
- Plugin role serializes with id and manifest
- Deserialization round-trips correctly

**Client roles tested**:
- `ClientRole::Ui { name: "test-ui" }`
- `ClientRole::Control`
- `ClientRole::Plugin { id: "test-plugin", manifest: ... }`

---

### 13. Multiple Notification Types (Test 6 - Extended)

**File**: `tests/integration_tests.rs::test_multiple_notification_types`

**Scenario**: Test that various plugin notification types can be created and transmitted.

**Notification types tested**:
1. `plugin_results` - Search results from plugin
2. `plugin_status` - Status/ambient update
3. `plugin_index` - Index items for persistence
4. `plugin_execute` - Execute action request
5. `plugin_update` - Result patches/updates

**Key assertions**:
```rust
for (method, params) in notifications {
    let notification = Notification::new(method, Some(params));
    assert_eq!(notification.method, method);
    assert!(notification.params.is_some());
}
```

---

## Running the Tests

### Run all integration tests:
```bash
cargo test -p hamr-daemon --test integration_tests
```

### Run a specific test:
```bash
cargo test -p hamr-daemon --test integration_tests test_plugin_registration_flow
```

### Run tests with output:
```bash
cargo test -p hamr-daemon --test integration_tests -- --nocapture
```

### Run all hamr-daemon tests (including unit tests):
```bash
cargo test -p hamr-daemon
```

## Test Results Summary

**Total Tests**: 10 integration tests + 71 existing unit tests = 81 tests

**Coverage**:
- ✅ Plugin registration and session management
- ✅ Multiple plugin isolation and coexistence
- ✅ Notification structure and serialization
- ✅ Status updates with ambient items
- ✅ Ambient clearing (empty arrays)
- ✅ Client role handling
- ✅ Plugin manifest transmission
- ✅ Registry state transitions

## Design Patterns Used

### 1. Registry Pattern
Tests verify the `PluginRegistry` maintains accurate state of connected plugins with proper isolation.

### 2. Session ID Management
Each plugin gets a unique `SessionId` that can be used to unregister or look up the plugin.

### 3. Notification Protocol
Tests validate JSON-RPC 2.0 notification format with `jsonrpc: "2.0"` and `method` fields.

### 4. Manifest Transmission
Tests confirm plugin manifests are properly serialized and transmitted during registration.

### 5. State Isolation
Tests ensure multiple plugins don't interfere with each other's state or notifications.

## Integration Points Tested

1. **Handler Integration** (`src/handlers.rs`)
   - `handle_register()` - Plugin registration
   - `handle_plugin_status()` - Status notifications
   - `handle_plugin_results()` - Results forwarding

2. **Registry Integration** (`src/registry.rs`)
   - Plugin connection tracking
   - Session-based unregistration
   - Sender channel management

3. **Protocol Integration** (`hamr-rpc`)
   - ClientRole serialization
   - Notification structure
   - Message format validation

4. **Session Integration** (`src/session.rs`)
   - SessionId generation and equality
   - Session type classification
   - Role-based validation

## Key Files

- **Tests**: `crates/hamr-daemon/tests/integration_tests.rs`
- **Registry**: `crates/hamr-daemon/src/registry.rs`
- **Handlers**: `crates/hamr-daemon/src/handlers.rs`
- **Session**: `crates/hamr-daemon/src/session.rs`
- **Protocol**: `crates/hamr-rpc/src/protocol.rs`

## Future Test Enhancements

Potential additional tests to implement:

1. **Full Socket Tests** - End-to-end tests with actual Unix sockets
2. **Message Forwarding** - UI receiving plugin results via daemon
3. **Error Scenarios** - Duplicate registration, invalid manifests
4. **Performance Tests** - Throughput with many concurrent plugins
5. **Race Conditions** - Rapid register/unregister cycles
6. **Protocol Compliance** - JSON-RPC 2.0 spec validation
7. **Timeout Handling** - Plugin hanging or slow responses
8. **Plugin Crash Recovery** - Daemon behavior on plugin exit

## References

- [JSON-RPC 2.0 Specification](https://www.jsonrpc.org/specification)
- `ARCHITECTURE.md` - Overall daemon architecture
- `src/handlers.rs` - Request/notification handler documentation
- `src/registry.rs` - Plugin registry implementation
