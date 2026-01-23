# Hamr

A standalone search bar / launcher built with Rust and GTK4.

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
paru -S hamr-git
```

Or use the quick install script:

```bash
curl -fsSL https://raw.githubusercontent.com/stewart86/hamr/main/install.sh | bash
```

Toggle the launcher with `hamr-gtk toggle` (bind this to a key in your compositor).

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
hamr-gtk toggle          # Toggle launcher visibility
hamr-gtk plugin <name> # Open specific plugin
hamr-cli status          # Check if daemon is running
hamr-cli test <plugin>   # Test a plugin
```

## Links

- [GitHub Repository](https://github.com/stewart86/hamr)
- [AUR Package](https://aur.archlinux.org/packages/hamr-git)
