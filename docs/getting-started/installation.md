# Installation

## Requirements

- [Quickshell](https://quickshell.outfoxxed.me/) (QML-based shell framework)
- A supported Wayland compositor: **Hyprland** or **Niri**
- Python 3.9+ (for plugins)

## Arch Linux (AUR)

```bash
paru -S hamr
systemctl --user enable --now hamr
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

## Other Distributions

```bash
curl -fsSL https://raw.githubusercontent.com/stewart86/hamr/main/install.sh | bash
```

The install script will:

- Detect your distribution and show how to install Quickshell if missing
- Clone the repo to `~/.local/share/hamr`
- Create symlinks and default config
- Show compositor-specific setup instructions

For Niri users, enable the systemd service:

```bash
~/.local/share/hamr/install.sh --enable-service
```

## Keybinding

Bind `hamr toggle` to a key in your compositor config.

### Hyprland

```conf
# ~/.config/hypr/hyprland.conf
exec-once = hamr

bind = $mainMod, SPACE, exec, hamr toggle
```

### Niri

```kdl
// ~/.config/niri/config.kdl
binds {
    Mod+Space { spawn "hamr" "toggle"; }
}
```

## Verify Installation

Check if Hamr is running:

```bash
hamr status
```

View logs:

```bash
journalctl --user -u hamr -f
```

## Updating

Arch Linux:

```bash
paru -Syu hamr
```

Other distributions:

```bash
~/.local/share/hamr/install.sh --update
```

## Uninstall

Arch Linux:

```bash
systemctl --user disable --now hamr
paru -R hamr
```

Other distributions:

```bash
~/.local/share/hamr/install.sh --uninstall
rm -rf ~/.local/share/hamr
```
