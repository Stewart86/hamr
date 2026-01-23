# Installation

## Architecture Overview

Hamr consists of several components that work together:

- **hamr-daemon**: Core service that manages plugins and search
- **hamr-gtk**: GTK4 launcher UI (what you see when you press the hotkey)
- **hamr-cli**: Command-line interface for testing and control
- **hamr-tui**: Terminal UI for headless environments

The daemon runs continuously and communicates with the UI clients via JSON-RPC.

## Requirements

- **GTK4 4.20+** and **gtk4-layer-shell** (for the GTK4 interface)
- A supported Wayland compositor: **Hyprland** or **Niri**
- Python 3.9+ (for plugins)
- Rust 1.85+ (for building from source)

### Compositor Support Matrix

| Compositor | Status | Notes |
|------------|--------|-------|
| **Hyprland** | ✅ Supported | Full functionality with layer-shell |
| **Niri** | ✅ Supported | Full functionality with layer-shell |
| **Sway** | ✅ Supported | Works with layer-shell protocol |
| **KDE Wayland** | ✅ Supported | Requires layer-shell support |
| **GNOME Wayland** | ❌ Not Supported | No layer-shell protocol support |
| **X11** | ❌ Not Supported | Wayland-only application |

## Arch Linux (AUR)

```bash
# Install from AUR
paru -S hamr-git

# Or build from source
paru -S hamr

## NixOS / Nix

### Quick install

```bash
# Try without installing
nix run github:Stewart86/hamr -- --help

# Install to your profile
nix profile install github:Stewart86/hamr
```

### NixOS / Home Manager

Add the flake input to your configuration:

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    hamr.url = "github:Stewart86/hamr";
  };

  outputs = { self, nixpkgs, hamr, ... }: {
    # NixOS
    nixosConfigurations.myhost = nixpkgs.lib.nixosSystem {
      modules = [{
        nixpkgs.overlays = [ hamr.overlays.default ];
        environment.systemPackages = [ pkgs.hamr ];
      }];
    };

    # Or Home Manager
    homeConfigurations.myuser = home-manager.lib.homeManagerConfiguration {
      modules = [{
        nixpkgs.overlays = [ hamr.overlays.default ];
        home.packages = [ pkgs.hamr ];
      }];
    };
  };
}
```

## Quick Install (All Distributions)

```bash
curl -fsSL https://raw.githubusercontent.com/stewart86/hamr/main/install.sh | bash
```

The install script will:

- Detect your distribution and install required dependencies (GTK4, gtk4-layer-shell)
- Download the latest release binaries
- Install to `~/.local/bin/`
- Copy essential plugins to `~/.local/share/hamr/plugins/`
- Create default configuration
- Show compositor-specific setup instructions

**Installer Flags:**

| Flag | Description |
|------|-------------|
| `--check` | Dry-run mode: show what would be installed without making changes |
| `--yes` | Assume yes for all prompts (non-interactive mode) |
| `--reset-user-data` | Reset user configuration and plugins (backup created) |

### Manual Dependencies

**Arch Linux:**
```bash
sudo pacman -S gtk4 gtk4-layer-shell python
```

**Fedora:**
```bash
sudo dnf install gtk4-devel gtk4-layer-shell-devel python3
```

**Ubuntu/Debian:**
```bash
sudo apt install libgtk-4-dev gtk4-layer-shell-dev python3
```

### Layer-shell Package Names by Distribution

| Distribution | Package Name |
|--------------|--------------|
| **Arch Linux** | `gtk4-layer-shell` |
| **Fedora** | `gtk4-layer-shell-devel` |
| **Ubuntu/Debian** | `gtk4-layer-shell-dev` |
| **openSUSE** | `gtk4-layer-shell-devel` |
| **Gentoo** | `gui-libs/gtk4-layer-shell` |

## Keybinding

Bind `hamr toggle` to a key in your compositor config.

### Hyprland

```conf
exec-once = hamr daemon
bind = $mainMod, SPACE, exec, hamr toggle
bind = $mainMod, V, exec, hamr plugin clipboard
```

### Niri

```kdl
// ~/.config/niri/config.kdl
spawn-at-startup "hamr-daemon"

binds {
    Mod+Space { spawn "hamr-gtk" "toggle"; }
    Mod+V { spawn "hamr-gtk" "plugin" "clipboard"; }
}
```

## Verify Installation

Check if Hamr daemon is running:

```bash
hamr status
```

View logs:

```bash
# Daemon logs (debug builds write to /tmp/hamr-daemon.log)
tail -f /tmp/hamr-daemon.log

# Or if using systemd
journalctl --user -u hamr-daemon -f
```

## Updating

Arch Linux (AUR):

```bash
paru -Syu hamr-git
```

Other distributions:

```bash
# Re-run the installer to update
curl -fsSL https://raw.githubusercontent.com/stewart86/hamr/main/install.sh | bash
```

## Uninstall

Arch Linux (AUR):

```bash
systemctl --user disable --now hamr-daemon
paru -R hamr-git
```

Other distributions:

```bash
# Remove binaries and config
rm -f ~/.local/bin/{hamr,hamr-cli,hamr-daemon,hamr-gtk,hamr-tui}
rm -rf ~/.local/share/hamr
rm -rf ~/.config/hamr

# Disable systemd service if enabled
systemctl --user disable --now hamr-daemon 2>/dev/null || true
```
