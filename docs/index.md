<p align="center">
  <img src="assets/logo.png" alt="Hamr Logo" width="200">
</p>

<h1 align="center">Hamr</h1>

<p align="center">A standalone search bar / launcher built with Rust and GTK4.</p>

> **Migration Notice**: This documentation covers the Rust/GTK4 rewrite of Hamr. For the legacy QML/Quickshell version, see the [legacy-qml branch](https://github.com/stewart86/hamr/tree/legacy-qml).

## Features

- Fast fuzzy search across apps, files, and plugins
- Plugin system with JSON-over-stdio protocol
- Frecency-based ranking (frequently used items rank higher)
- GTK4 native UI with layer shell support
- Support for Hyprland and Niri compositors
- Material Design 3 theming with dynamic color support

## Quick Start

### Installation

Install from AUR:

```bash
paru -S hamr-bin  # Pre-built binary (recommended)
# or
paru -S hamr      # Build from source
```

Or use the quick install script:

```bash
curl -fsSL https://hamr.run/install.sh | bash
```

Toggle the launcher with `hamr toggle` (bind this to a key in your compositor).

### Basic Usage

| Action       | Description                    |
| ------------ | ------------------------------ |
| Start typing | Search apps and indexed items  |
| `/plugin`    | Open a specific plugin         |
| `Tab`        | View actions for selected item |
| `Enter`      | Execute selected item/action   |
| `Escape`     | Go back / close                |

## Documentation

<!-- prettier-ignore-start -->

<div class="grid cards" markdown>

-   :material-book-open-variant:{ .lg .middle } **Getting Started**

    ---

    Installation, configuration, and theming

    [:octicons-arrow-right-24: Installation](getting-started/installation.md)

    [:octicons-arrow-right-24: Theming](getting-started/theming.md)

-   :material-puzzle:{ .lg .middle } **Plugins**

    ---

    Build custom plugins to extend Hamr

    [:octicons-arrow-right-24: Building Plugins](plugins/index.md)

    [:octicons-arrow-right-24: Cheat Sheet](plugins/cheatsheet.md)

</div>

<!-- prettier-ignore-end -->

## CLI Commands

```bash
hamr toggle          # Toggle launcher visibility
hamr plugin <name>   # Open specific plugin
hamr status          # Check if daemon is running
hamr test <plugin>   # Test a plugin
```

## Links

- [GitHub Repository](https://github.com/stewart86/hamr)
- [AUR Package](https://aur.archlinux.org/packages/hamr)
