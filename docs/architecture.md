# Software Architecture

Hamr is a fast, extensible desktop launcher built with Rust, featuring a client-server architecture with native UI implementations.

## System Overview

```mermaid
graph TB
    subgraph "UI Layer"
        TUI[hamr-tui<br/>TUI + Ratatui]
        GTK[hamr-gtk<br/>GTK4 + Layer Shell]
        Future[(future)<br/>macOS/Windows]
    end

    subgraph "Communication Layer"
        RPC[JSON-RPC 2.0 over Unix Socket]
    end

    subgraph "Daemon Layer"
        Daemon[hamr-daemon<br/>Socket Server]
        Core[HamrCore<br/>State Machine<br/>Search, Plugins, Index]
        PluginMgr[Plugin Manager<br/>Spawning & IPC]
    end

    subgraph "Plugin Registry"
        Discovered[Discovered Plugins<br/>• stdio<br/>• socket spawned]
        Registered[Registered Plugins<br/>• External services<br/>• Spawned sockets]
    end

    subgraph "Example Plugins"
        Apps[apps<br/>stdio<br/>spawn/req]
        Calc[calc<br/>stdio<br/>spawn/req]
        Wifi[wifi<br/>socket<br/>daemon]
    end

    TUI --> RPC
    GTK --> RPC
    Future --> RPC

    RPC --> Daemon
    Daemon --> Core
    Daemon --> PluginMgr

    Core --> PluginMgr
    PluginMgr --> Discovered
    PluginMgr --> Registered

    Registered --> Apps
    Registered --> Calc
    Registered --> Wifi
```

## Core Components

```mermaid
graph TB
    subgraph "Core Components"
        Core[HamrCore<br/>Platform-Agnostic Core]
        Daemon[HamrDaemon<br/>Socket Server]
        TUI[HamrTUI<br/>Terminal UI]
        GTK[HamrGTK<br/>Native Linux UI<br/>Planned]
        CLI[HamrCLI<br/>Control Client]
    end

    subgraph "HamrCore Responsibilities"
        Plugin[Plugin Management<br/>Discovery, Spawning, Lifecycle]
        Search[Search Engine<br/>Nucleo Fuzzy Matching<br/>Frecency Ranking]
        Index[Index Storage<br/>Disk Persistence<br/>Incremental Updates]
        Config[Configuration<br/>XDG Loading<br/>Hot-Reload]
        State[State Machine<br/>CoreEvent → CoreUpdate]
    end

    subgraph "HamrDaemon Responsibilities"
        Socket[Unix Socket Server<br/>JSON-RPC 2.0<br/>Length-Prefixed Framing]
        Clients[Client Management<br/>UI, Control, Plugin]
        Registry[Plugin Registry<br/>Merge Discovered + Registered]
        Session[Session Handling<br/>Single Active UI]
        Watch[Config Watching<br/>Filesystem Monitoring]
    end

    subgraph "UI Client Features"
        TUI_F[TUI Features<br/>Ratatui Interface<br/>Keyboard Navigation<br/>Compositor Integration<br/>Action Execution]
        GTK_F[GTK Features<br/>GTK4 + Layer Shell<br/>Material Design 3<br/>Multi-Monitor]
        CLI_F[CLI Features<br/>Control Operations<br/>Toggle/Show/Hide<br/>Plugin Management]
    end

    Core --> Plugin
    Core --> Search
    Core --> Index
    Core --> Config
    Core --> State

    Daemon --> Socket
    Daemon --> Clients
    Daemon --> Registry
    Daemon --> Session
    Daemon --> Watch

    TUI --> TUI_F
    GTK --> GTK_F
    CLI --> CLI_F
```

## Plugin System

Hamr supports two plugin architectures for different use cases:

```mermaid
graph TD
    subgraph "Plugin Types"
        Stdio[Stdio Plugins<br/>Request-Response]
        Socket[Socket Plugins<br/>Persistent Connection]
    end

    subgraph "Stdio Characteristics"
        S1[Stateless<br/>Spawn per request]
        S2[Simple Communication<br/>stdin/stdout JSON]
        S3[Short-lived<br/>Terminates after response]
        S4[Use Cases<br/>Simple queries<br/>Transformations<br/>One-shot operations]
    end

    subgraph "Socket Characteristics"
        K1[Stateful<br/>Long-running processes]
        K2[Full JSON-RPC<br/>Bidirectional communication]
        K3[Lifecycle Managed<br/>Auto-restart on crash]
        K4[Use Cases<br/>Monitoring<br/>Background services<br/>Ambient items]
    end

    subgraph "Communication Examples"
        Calc[Calculator Plugin<br/>python calc.py]
        Wifi[WiFi Plugin<br/>Background Service]
    end

    Stdio --> S1
    Stdio --> S2
    Stdio --> S3
    Stdio --> S4

    Socket --> K1
    Socket --> K2
    Socket --> K3
    Socket --> K4

    S4 --> Calc
    K4 --> Wifi
```

### Stdio Plugin Example

```mermaid
sequenceDiagram
    participant U as User
    participant D as Daemon
    participant P as Plugin

    U->>D: Types "2+2"
    D->>P: Spawn calc.py
    D->>P: Send JSON {"method": "search", "query": "2+2"}
    P->>D: Return {"results": [{"name": "4", "description": "2 + 2 = 4"}]}
    D->>P: Plugin terminates
    D->>U: Display results
```

### Socket Plugin Example

```mermaid
sequenceDiagram
    participant D as Daemon
    participant P as Plugin

    Note over D,P: Plugin startup
    D->>P: Spawn wifi.py (persistent)
    P->>D: Register as plugin

    Note over D,P: User interaction
    D->>P: {"jsonrpc": "2.0", "method": "search", "params": {"query": "wifi"}, "id": 1}
    P->>D: {"jsonrpc": "2.0", "result": {"results": [{"name": "WiFi: Connected"}]}}
```

## Data Flow

### Search Flow

```mermaid
sequenceDiagram
    participant U as User
    participant UI as UI Client
    participant D as Daemon
    participant C as HamrCore
    participant P as Plugins

    U->>UI: Types search query
    UI->>D: query_changed (JSON-RPC)
    D->>C: Process search request
    C->>P: Plugin.search() calls
    P->>C: Return search results
    C->>D: Emit CoreUpdate::Results
    D->>UI: results/results_update (JSON-RPC)
    UI->>U: Display updated results
```

### Action Execution

```mermaid
sequenceDiagram
    participant U as User
    participant UI as UI Client
    participant D as Daemon
    participant C as HamrCore

    U->>UI: Selects item/action
    UI->>D: item_selected (JSON-RPC)
    D->>C: Process action request
    C->>D: Return ExecuteAction
    D->>UI: execute (JSON-RPC)
    UI->>UI: Execute platform action
    Note over UI: Launch app, copy text,<br/>open URL, show notification
```

## Key Design Principles

### 1. Platform Agnostic Core

- **No I/O in core**: Actions are data structures, not execution
- **UI owns execution**: Platform-specific APIs (launch, copy, notify)
- **Type safety**: Zero runtime type coercion with comprehensive Rust types

### 2. Plugin-First Architecture

- **Extensible by design**: All features implemented as plugins
- **Two plugin types**: Stdio for simple, socket for complex
- **Registry merging**: Socket plugins can override discovered ones

### 3. Event-Driven Communication

- **JSON-RPC 2.0**: Industry standard protocol
- **Length-prefixed framing**: Reliable message boundaries
- **Multi-client support**: Single daemon serves all clients

### 4. Performance Focus

- **Nucleo fuzzy search**: State-of-the-art matching algorithm
- **Frecency scoring**: Usage patterns + time decay
- **Incremental indexing**: Only update changed items
- **Hot config reload**: No restart required

## Contributing

### Understanding the Architecture

When contributing to Hamr, keep these principles in mind:

1. **Core stays pure**: Business logic only, no platform code
2. **UI executes actions**: Core defines what, UI decides how
3. **Plugins are first-class**: New features should be plugins
4. **Type safety everywhere**: Use Rust's type system extensively
5. **Test core logic**: Unit tests for all business logic

### Development Workflow

```mermaid
graph LR
    subgraph "Development Cycle"
        Plan[Plan Feature<br/>Design API]
        Core[Implement Core<br/>hamr-core]
        Types[Add Types<br/>hamr-types]
        Test[Test Core Logic<br/>Unit Tests]
        UI[Update UI<br/>hamr-tui/hamr-gtk]
        Integration[Test Integration<br/>Full System]
        Docs[Update Docs<br/>architecture.md]
    end

    Plan --> Core
    Core --> Types
    Types --> Test
    Test --> UI
    UI --> Integration
    Integration --> Docs
    Docs --> Plan

    style Plan fill:#e8f5e8
    style Core fill:#e1f5fe
    style Test fill:#fff3e0
    style Integration fill:#f3e5f5
```

```bash
# Work on core logic
cd crates/hamr-core
cargo test                    # Run core tests
cargo test specific_test      # Run single test

# Work on daemon
cd crates/hamr-daemon
cargo run                     # Start daemon

# Work on TUI (separate terminal)
cd crates/hamr-tui
cargo run                     # Start TUI client

# Check everything compiles
cargo check --workspace
```

### Adding New Features

1. **Core logic first**: Implement in `hamr-core` with tests
2. **Type definitions**: Add to `hamr-types` if shared
3. **UI integration**: Update UI clients to handle new types
4. **Documentation**: Update this guide and API docs

### Plugin Development

1. **Choose plugin type**: Stdio for simple, socket for complex
2. **Manifest first**: Define capabilities in `manifest.json`
3. **Protocol compliance**: Follow JSON protocol specification
4. **Error handling**: Graceful failure with meaningful messages

## Crate Structure

### Dependency Graph

```mermaid
graph TD
    Types[hamr-types<br/>Shared Types Foundation]

    Core[hamr-core<br/>Platform-Agnostic Core<br/>Plugin Management<br/>Search Engine<br/>State Machine]

    RPC[hamr-rpc<br/>JSON-RPC Protocol<br/>Transport Layer<br/>Client Library]

    Daemon[hamr-daemon<br/>Socket Server<br/>Client Management<br/>Session Handling]

    TUI[hamr-tui<br/>Terminal UI<br/>Ratatui Interface<br/>Action Execution]

    CLI[hamr-cli<br/>Control Client<br/>Command Interface]

    GTK[hamr-gtk<br/>GTK4 UI<br/>Native Linux<br/>TODO]

    Types --> Core
    Types --> RPC

    Core --> Daemon
    RPC --> Daemon

    Daemon --> TUI
    Daemon --> CLI
    Daemon --> GTK

    style Core fill:#e1f5fe
    style Daemon fill:#f3e5f5
    style TUI fill:#e8f5e8
    style CLI fill:#fff3e0
    style GTK fill:#ffebee,stroke-dasharray: 5 5
```

This architecture enables Hamr to be fast, extensible, and maintainable while supporting multiple platforms and UI implementations.