# Installation

## Architecture Overview

Hamr consists of several components that work together:

- **hamr-daemon**: Core service that manages plugins and search
- **hamr-gtk**: GTK4 launcher UI (what you see when you press the hotkey)
- **hamr**: Unified CLI and default entrypoint (starts GTK; auto-starts daemon)
- **hamr-tui**: Terminal UI for headless environments

The daemon runs continuously and communicates with the UI clients via JSON-RPC.

## Requirements

- **GTK4 4.20+** and **gtk4-layer-shell** (for the GTK4 interface)
- A supported Wayland compositor: **Hyprland** or **Niri**
- Python 3.9+ (for plugins)
- Rust 1.88+ (for building from source)

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
# Pre-built binary (recommended - faster install)
paru -S hamr-bin

# Or build from source
paru -S hamr
```

After installing:

```bash
# Option 1: Run directly (no systemd)
hamr

# Option 2 (recommended, opt-in): systemd user services
hamr install
systemctl --user start hamr-gtk
```

Note: AUR packages do not auto-enable systemd services; `hamr install` is the opt-in step.

## Manual Download

Download pre-built binaries directly from GitHub:

```bash
# Download latest release
wget https://github.com/Stewart86/hamr/releases/latest/download/hamr-linux-x86_64.tar.gz

# Extract
tar -xzf hamr-linux-x86_64.tar.gz
cd hamr-linux-x86_64

# Install binaries
mkdir -p ~/.local/bin
cp hamr hamr-daemon hamr-gtk hamr-tui ~/.local/bin/
cp -r plugins ~/.local/bin/

# Run directly (no systemd)
~/.local/bin/hamr

# Or (recommended, opt-in): set up systemd user services
~/.local/bin/hamr install
systemctl --user start hamr-gtk
```

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
curl -fsSL https://hamr.run/install.sh | bash

# Or opt-in to systemd setup during install
curl -fsSL https://hamr.run/install.sh | bash -s -- --systemd
```

The install script will:

- Download the latest release binaries
- Install to `~/.local/bin/`
- Copy bundled plugins next to the binaries (`~/.local/bin/plugins/`)
- Optionally run `hamr install` to set up systemd user services (opt-in)

**Installer Flags:**

| Flag | Description |
|------|-------------|
| `--check` | Dry-run mode: show what would be installed without making changes |
| `--yes` | Assume yes for all prompts (non-interactive mode) |
| `--reset-user-data` | Reset user configuration and plugins (backup created) |
| `--systemd` | Run `hamr install` after installing binaries (opt-in) |

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

## Post-Installation Setup

You can run Hamr immediately (no systemd required):

```bash
hamr
```

To install and enable systemd user services (recommended, opt-in), run:

```bash
hamr install --check  # Preview what will be set up
hamr install          # Set up systemd services and directories
systemctl --user start hamr-gtk
```

This creates:

- Systemd user services (`hamr-daemon.service`, `hamr-gtk.service`)
- Config directory (`~/.config/hamr/`)
- Essential plugins copied to user config

Then start the launcher:

```bash
# Option 1: Via systemd (recommended for auto-start on login)
systemctl --user start hamr-gtk

# Option 2: Direct (works without systemd)
hamr
```

**Without systemd**: Running `hamr` will auto-start the daemon as a background process. No additional setup needed.

For full CLI documentation, see [CLI Reference](cli.md).

## Keybinding

Bind `hamr toggle` to a key in your compositor config.

### Hyprland

```conf
exec-once = hamr
bind = $mainMod, SPACE, exec, hamr toggle
bind = $mainMod, V, exec, hamr plugin clipboard
```

### Niri

```kdl
// ~/.config/niri/config.kdl
spawn-at-startup "hamr"

binds {
    Mod+Space { spawn "hamr" "toggle"; }
    Mod+V { spawn "hamr" "plugin" "clipboard"; }
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

For detailed logging configuration, including `RUST_LOG` and `HAMR_PLUGIN_DEBUG` environment variables, see the [Logging Guide](logging.md).

If you encounter issues, see the [Troubleshooting Guide](troubleshooting.md) for common problems and solutions.

## Updating

Arch Linux (AUR):

```bash
paru -Syu hamr
# or
paru -Syu hamr-bin
```

Other distributions:

```bash
# Re-run the installer to update
curl -fsSL https://hamr.run/install.sh | bash
```

## Uninstall

The recommended way to uninstall depends on how you installed Hamr.

### Using the CLI (recommended)

```bash
hamr uninstall          # Remove binaries, services, socket (preserves config)
hamr uninstall --purge  # Remove everything including ~/.config/hamr
```

This will:

- Stop and disable systemd user services
- Remove service files
- Remove binaries (hamr, hamr-daemon, hamr-gtk, hamr-tui)
- Remove system plugins next to binaries
- Remove socket files
- Clean PATH entries from shell rc files
- Preserve user config by default (use `--purge` to remove)

### Using the uninstall script

If the `hamr` binary is already removed or broken:

```bash
curl -fsSL https://hamr.run/uninstall.sh | bash

# Or remove everything including config
curl -fsSL https://hamr.run/uninstall.sh | bash -s -- --purge

# Non-interactive mode
curl -fsSL https://hamr.run/uninstall.sh | bash -s -- --yes
```

### Arch Linux (AUR)

```bash
hamr uninstall          # Remove systemd services first
paru -R hamr            # or hamr-bin
```

### NixOS / Nix

```bash
nix profile remove hamr
```

Or remove the flake input from your NixOS/Home Manager configuration.
