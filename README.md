# Hamr

A fast, extensible desktop launcher for Linux.

![License](https://img.shields.io/badge/license-MIT-blue)
![Rust](https://img.shields.io/badge/rust-1.85+-orange)
![Platform](<https://img.shields.io/badge/platform-Linux%20(Wayland)-green>)

Hamr learns from your usage patterns to surface what you need, when you need it. Type a few characters to launch apps, calculate math, search files, access clipboard history, and more.

## Features

- **Frecency ranking** - Results sorted by frequency + recency
- **Learned shortcuts** - Type "q" to find QuickLinks if that's how you found it before
- **Fuzzy matching** - Fast, typo-tolerant search powered by [nucleo](https://github.com/helix-editor/nucleo)
- **Smart suggestions** - Context-aware suggestions based on time and usage
- **Extensible plugins** - JSON protocol, any language (Python, Bash, Go, Rust)
- **Live updates** - Plugins emit real-time updates without refreshing the list
- **Rich UI** - Forms, cards, sliders, gauges, preview panels, grid browsers

## Installation

### Quick Install (Linux x86_64/aarch64)

```bash
curl -fsSL https://raw.githubusercontent.com/stewart86/hamr/main/install.sh | bash
```

This downloads the latest release binaries, installs to `~/.local/bin`, copies essential plugins, and sets up systemd services.

**Dependencies:** GTK4 4.20+, gtk4-layer-shell, Python 3.9+

### Build from Source

Requires Rust 1.85+, GTK4 4.20+, gtk4-layer-shell.

```bash
# Install dependencies (Arch)
sudo pacman -S gtk4 gtk4-layer-shell python

# Install dependencies (Fedora)
sudo dnf install gtk4-devel gtk4-layer-shell-devel python3

# Install dependencies (Ubuntu/Debian)
sudo apt install libgtk-4-dev gtk4-layer-shell-dev python3

# Clone this Rust branch (main branch contains QML implementation)
git clone -b rusty-hamr-v1.0.0-alpha https://github.com/stewart86/hamr
cd hamr

# Build and install using install.sh
./install.sh

# Or build manually
cargo build --release
mkdir -p ~/.local/bin
cp target/release/{hamr,hamr-daemon,hamr-gtk,hamr-tui} ~/.local/bin/
hamr install
```

### NixOS / Nix

```bash
# Try without installing
nix run github:stewart86/hamr

# Install to profile
nix profile install github:stewart86/hamr
```

Or add to your flake:

```nix
{
  inputs.hamr.url = "github:stewart86/hamr";
  # ...
  nixpkgs.overlays = [ hamr.overlays.default ];
  environment.systemPackages = [ pkgs.hamr ];
}
```

### Arch Linux (AUR)

```bash
# Using yay
yay -S hamr-git

# Or with paru
paru -S hamr-git
```

## Quick Start

```bash
hamr-daemon             # Start daemon (or use systemd)
hamr-gtk                # Start launcher UI
hamr-gtk toggle         # Toggle visibility
hamr-gtk plugin clipboard # Open specific plugin
```

### Compositor Setup

**Hyprland** (`~/.config/hypr/hyprland.conf`):

```conf
exec-once = hamr-daemon
bind = $mainMod, SPACE, exec, hamr-gtk toggle
bind = $mainMod, V, exec, hamr-gtk plugin clipboard
```

**Niri** (`~/.config/niri/config.kdl`):

```kdl
spawn-at-startup "hamr-daemon"

binds {
    Mod+Space { spawn "hamr-gtk" "toggle"; }
    Mod+V { spawn "hamr-gtk" "plugin" "clipboard"; }
}
```

**Systemd** (manual setup):

```bash
# The installer sets up the systemd service
# You only need to start it manually:
systemctl --user start hamr-daemon.service

# Or start the GTK launcher manually (auto-starts daemon)
hamr-gtk
```

## Built-in Plugins

| Plugin      | Description                                  |
| ----------- | -------------------------------------------- |
| `apps`      | Application launcher with categories         |
| `shell`     | Execute shell commands                       |
| `calculate` | Calculator with currency, units, temperature |
| `clipboard` | Clipboard history with search                |
| `power`     | Shutdown, reboot, suspend, logout            |

Additional plugins available: `bitwarden`, `dictionary`, `emoji`, `files`, `quicklinks`, `snippets`, `totp`, `weather`, `wifi`, `youtube`.

## Prefix Shortcuts

| Prefix | Function          | Prefix | Function          |
| ------ | ----------------- | ------ | ----------------- |
| `~`    | File search       | `;`    | Clipboard history |
| `/`    | Actions & plugins | `!`    | Shell history     |
| `=`    | Calculator        | `:`    | Emoji picker      |

Prefixes are configurable in `~/.config/hamr/config.json`.

## Documentation

- [Installation](docs/getting-started/installation.md) - Full installation guide
- [Configuration](docs/getting-started/configuration.md) - All config options
- [Theming](docs/getting-started/theming.md) - Material Design 3 colors, matugen/pywal
- [Plugin Development](docs/plugins/index.md) - Create your own plugins
- [API Reference](docs/plugins/api-reference.md) - Plugin protocol specification
- [Architecture](ARCHITECTURE.md) - System design and crate structure

## Development

```bash
# Build
cargo build

# Run tests
cargo test

# Run with debug logging
RUST_LOG=debug cargo run -p hamr-daemon

# Test a plugin
cargo run -p hamr -- test shell "ls -la"
```

## Architecture

```
hamr-cli     hamr-gtk     hamr-tui
    \           |           /
     \          |          /
      +----JSON-RPC 2.0---+
               |
          hamr-daemon
               |
          hamr-core
               |
    +---------+---------+
    |         |         |
  search   plugins   frecency
```

- **hamr-core**: Platform-agnostic core (search, plugins, frecency, indexing)
- **hamr-daemon**: Socket server wrapping core
- **hamr-gtk**: GTK4 native UI with layer shell
- **hamr-tui**: Terminal UI for headless use
- **hamr-cli**: Command-line interface

## Contributing

Contributions welcome! Please read the [Architecture Guide](ARCHITECTURE.md) and [Agent Guidelines](AGENTS.md) before submitting PRs.

## License

MIT License - see [LICENSE](LICENSE) for details.
