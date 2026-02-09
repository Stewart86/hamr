<p align="center">
  <img src="assets/logo.png" alt="Hamr Logo" width="200">
</p>

<h1 align="center">Hamr</h1>

<p align="center">A fast, extensible desktop launcher for Linux.</p>

![Hamr Screenshot](docs/plugins/images/results-basic.png)

![License](https://img.shields.io/badge/license-MIT-blue)
![Rust](https://img.shields.io/badge/rust-1.88+-orange)
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

### Quick Install (Linux)

```bash
curl -fsSL https://hamr.run/install.sh | bash

# Or opt-in to systemd setup during install
curl -fsSL https://hamr.run/install.sh | bash -s -- --systemd
```

This downloads the latest release binaries (x86_64/aarch64), installs to `~/.local/bin`, and copies bundled plugins next to the binaries.
Systemd setup is optional (opt-in) via `--systemd` or by running `hamr install` after installation.

**Dependencies:** GTK4 4.20+, gtk4-layer-shell, Python 3.9+

### Manual Download

```bash
# Pick the right archive for your CPU:
# - x86_64:  hamr-linux-x86_64.tar.gz
# - aarch64: hamr-linux-aarch64.tar.gz
wget https://github.com/Stewart86/hamr/releases/latest/download/hamr-linux-x86_64.tar.gz
tar -xzf hamr-linux-x86_64.tar.gz
cd hamr-linux-x86_64
mkdir -p ~/.local/bin/
cp hamr hamr-daemon hamr-gtk hamr-tui ~/.local/bin/
cp -r plugins ~/.local/bin/

# Option 1: Run directly (no systemd)
~/.local/bin/hamr

# Option 2 (recommended, opt-in): systemd user services
~/.local/bin/hamr install
systemctl --user start hamr-gtk
```

### Compositor Support

| Compositor | Status | Notes |
|------------|--------|-------|
| **Hyprland** | ✅ Supported | Full functionality with layer-shell |
| **Niri** | ✅ Supported | Full functionality with layer-shell |
| **Sway** | ✅ Supported | Works with layer-shell protocol |
| **KDE Wayland** | ✅ Supported | Requires layer-shell support |
| **GNOME Wayland** | ❌ Not Supported | No layer-shell protocol support |
| **X11** | ❌ Not Supported | Wayland-only application |

**Installer Flags:**

| Flag | Description |
|------|-------------|
| `--check` | Dry-run mode: show what would be installed without making changes |
| `--yes` | Assume yes for all prompts (non-interactive mode) |
| `--reset-user-data` | Reset user configuration and plugins (backup created) |
| `--systemd` | Run `hamr install` after installing binaries (opt-in) |

### Build from Source

Requires Rust 1.88+, GTK4 4.20+, gtk4-layer-shell.

```bash
# Install dependencies (Arch)
sudo pacman -S gtk4 gtk4-layer-shell rust python

# Install dependencies (Fedora)
sudo dnf install gtk4-devel gtk4-layer-shell-devel rust python3

# Install dependencies (Ubuntu/Debian)
sudo apt install libgtk-4-dev gtk4-layer-shell-dev rustc cargo python3

# Clone and build
git clone https://github.com/stewart86/hamr
cd hamr
./install.sh

# Or build manually
cargo build --release
mkdir -p ~/.local/bin
cp target/release/{hamr,hamr-daemon,hamr-gtk,hamr-tui} ~/.local/bin/

# Option 1: Run directly (no systemd)
hamr

# Option 2 (recommended, opt-in): systemd user services
hamr install
systemctl --user start hamr-gtk
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
# Pre-built binary (recommended - faster install)
paru -S hamr-bin

# Or build from source
paru -S hamr
```

Run (two ways):

```bash
# Option 1: Run directly (no systemd)
hamr

# Option 2 (recommended, opt-in): systemd user services
hamr install
systemctl --user start hamr-gtk
```

Note: AUR packages do not auto-enable systemd services; `hamr install` is the opt-in step.

## Quick Start

```bash
hamr                    # Start launcher (auto-starts daemon)
hamr toggle             # Toggle visibility
hamr plugin clipboard   # Open specific plugin
hamr status             # Check daemon status
```

### Compositor Setup

**Hyprland** (`~/.config/hypr/hyprland.conf`):

```conf
exec-once = hamr
bind = $mainMod, SPACE, exec, hamr toggle
bind = $mainMod, V, exec, hamr plugin clipboard
```

**Niri** (`~/.config/niri/config.kdl`):

```kdl
spawn-at-startup "hamr"

binds {
    Mod+Space { spawn "hamr" "toggle"; }
    Mod+V { spawn "hamr" "plugin" "clipboard"; }
}
```

**Systemd** (optional):

```bash
# Systemd user services are recommended for auto-start on login (opt-in)
hamr install
systemctl --user start hamr-gtk

# Without systemd, just run:
hamr
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
- [Logging](docs/getting-started/logging.md) - Log paths, env vars, debugging
- [Theming](docs/getting-started/theming.md) - Material Design 3 colors, matugen/pywal
- [Troubleshooting](docs/getting-started/troubleshooting.md) - Common issues and solutions
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
