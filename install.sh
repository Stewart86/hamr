#!/usr/bin/env bash
# Hamr installation script
# Usage: ./install.sh [--uninstall]

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}"
QUICKSHELL_DIR="$CONFIG_DIR/quickshell"
HAMR_LINK="$QUICKSHELL_DIR/hamr"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

info() { echo -e "${GREEN}[INFO]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; exit 1; }

get_aur_helper() {
    for helper in paru yay pikaur trizen aurman; do
        if command -v "$helper" >/dev/null 2>&1; then
            echo "$helper"
            return
        fi
    done
    echo "pacman"  # fallback, won't work for AUR packages
}

get_dependencies_from_pkgbuild() {
    # Source PKGBUILD to get depends array
    local pkgbuild="$SCRIPT_DIR/PKGBUILD"
    if [[ -f "$pkgbuild" ]]; then
        # shellcheck source=/dev/null
        source "$pkgbuild"
        echo "${depends[@]}"
    fi
}

get_missing_dependencies() {
    local all_deps
    all_deps=$(get_dependencies_from_pkgbuild)
    local missing=()

    for dep in $all_deps; do
        # pacman -Qq checks if package is installed (including as a provider)
        if ! pacman -Qq "$dep" &>/dev/null; then
            missing+=("$dep")
        fi
    done

    echo "${missing[@]}"
}

install_dependencies() {
    local helper
    helper=$(get_aur_helper)
    local missing
    missing=$(get_missing_dependencies)

    if [[ -z "$missing" ]]; then
        info "All dependencies already installed."
        return 0
    fi

    info "Installing dependencies with $helper..."
    echo "Packages: $missing"
    echo ""

    if [[ "$helper" == "pacman" ]]; then
        warn "No AUR helper found. Some packages require AUR."
        warn "Install an AUR helper first: https://wiki.archlinux.org/title/AUR_helpers"
        exit 1
    else
        $helper -S --needed $missing
    fi
}

check_dependencies() {
    local missing
    missing=$(get_missing_dependencies)

    if [[ -n "$missing" ]]; then
        warn "Missing dependencies:"
        for dep in $missing; do
            echo "  - $dep"
        done
        echo ""
        local helper
        helper=$(get_aur_helper)
        
        read -p "Install dependencies automatically? [Y/n] " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Nn]$ ]]; then
            install_dependencies
            # Re-check after install
            missing=$(get_missing_dependencies)
            if [[ -n "$missing" ]]; then
                error "Some dependencies failed to install: $missing"
            fi
        else
            echo "Install manually with:"
            echo "  $helper -S $missing"
            echo ""
            read -p "Continue anyway? [y/N] " -n 1 -r
            echo
            [[ $REPLY =~ ^[Yy]$ ]] || exit 1
        fi
    else
        info "All required dependencies installed."
    fi

    # Show optional dependencies from PKGBUILD
    local pkgbuild="$SCRIPT_DIR/PKGBUILD"
    if [[ -f "$pkgbuild" ]]; then
        # shellcheck source=/dev/null
        source "$pkgbuild"
        if [[ ${#optdepends[@]} -gt 0 ]]; then
            local missing_opt=()
            for opt in "${optdepends[@]}"; do
                local pkg="${opt%%:*}"
                if ! pacman -Qi "$pkg" &>/dev/null; then
                    missing_opt+=("$opt")
                fi
            done
            if [[ ${#missing_opt[@]} -gt 0 ]]; then
                info "Optional dependencies not installed:"
                for dep in "${missing_opt[@]}"; do
                    echo "  - $dep"
                done
            fi
        fi
    fi
}

create_default_config() {
    local config_file="$CONFIG_DIR/hamr/config.json"
    
    # Default config template
    local default_config='{
  "apps": {
    "terminal": "ghostty",
    "terminalArgs": "--class=floating.terminal",
    "shell": "zsh"
  },
  "search": {
    "nonAppResultDelay": 30,
    "debounceMs": 50,
    "pluginDebounceMs": 150,
    "maxHistoryItems": 500,
    "maxDisplayedResults": 16,
    "maxRecentItems": 20,
    "shellHistoryLimit": 50,
    "engineBaseUrl": "https://www.google.com/search?q=",
    "excludedSites": ["quora.com", "facebook.com"],
    "prefix": {
      "action": "/",
      "app": ">",
      "clipboard": ";",
      "emojis": ":",
      "file": "~",
      "math": "=",
      "shellCommand": "$",
      "shellHistory": "!",
      "webSearch": "?"
    },
    "shellHistory": {
      "enable": true,
      "shell": "auto",
      "customHistoryPath": "",
      "maxEntries": 500
    },
    "actionKeys": ["u", "i", "o", "p"]
  },
  "imageBrowser": {
    "useSystemFileDialog": false,
    "columns": 4,
    "cellAspectRatio": 1.333,
    "sidebarWidth": 140
  },
  "appearance": {
    "backgroundTransparency": 0.2,
    "contentTransparency": 0.2,
    "launcherXRatio": 0.5,
    "launcherYRatio": 0.1
  },
  "sizes": {
    "searchWidth": 580,
    "searchInputHeight": 40,
    "maxResultsHeight": 600,
    "resultIconSize": 40,
    "imageBrowserWidth": 1200,
    "imageBrowserHeight": 690,
    "windowPickerMaxWidth": 350,
    "windowPickerMaxHeight": 220
  },
  "fonts": {
    "main": "Google Sans Flex",
    "monospace": "JetBrains Mono NF",
    "reading": "Readex Pro",
    "icon": "Material Symbols Rounded"
  },
  "paths": {
    "wallpaperDir": "",
    "colorsJson": ""
  }
}'

    if [[ -f "$config_file" ]]; then
        # Config exists - merge new keys without overwriting existing values
        if command -v jq >/dev/null 2>&1; then
            info "Updating config with new default keys (preserving existing values)..."
            local tmp_file=$(mktemp)
            # Use jq to merge: existing values take priority over defaults
            echo "$default_config" | jq -s '.[0] * .[1]' - "$config_file" > "$tmp_file"
            mv "$tmp_file" "$config_file"
        else
            info "Config exists. Install jq to auto-merge new config options."
        fi
    else
        # Create new config
        info "Creating default config: $config_file"
        echo "$default_config" > "$config_file"
    fi
}

install_hamr() {
    info "Installing hamr..."

    # Create quickshell config directory
    mkdir -p "$QUICKSHELL_DIR"

    # Remove existing symlink or directory
    if [[ -L "$HAMR_LINK" ]]; then
        rm "$HAMR_LINK"
    elif [[ -d "$HAMR_LINK" ]]; then
        warn "Existing hamr directory found at $HAMR_LINK"
        read -p "Replace with symlink? [y/N] " -n 1 -r
        echo
        [[ $REPLY =~ ^[Yy]$ ]] || exit 1
        rm -rf "$HAMR_LINK"
    fi

    # Create symlink
    ln -s "$SCRIPT_DIR" "$HAMR_LINK"
    info "Created symlink: $HAMR_LINK -> $SCRIPT_DIR"

    # Create user plugins directory
    mkdir -p "$CONFIG_DIR/hamr/plugins"
    info "Created user plugins directory: $CONFIG_DIR/hamr/plugins"
    info "Built-in plugins loaded from: $SCRIPT_DIR/plugins/"
    info "User plugins loaded from: $CONFIG_DIR/hamr/plugins/"

    # Create or update default config
    create_default_config

    # Copy switchwall.sh to user config if it doesn't exist
    mkdir -p "$CONFIG_DIR/hamr/scripts"
    if [[ ! -f "$CONFIG_DIR/hamr/scripts/switchwall.sh" ]]; then
        cp "$SCRIPT_DIR/scripts/colors/switchwall.sh" "$CONFIG_DIR/hamr/scripts/switchwall.sh"
        chmod +x "$CONFIG_DIR/hamr/scripts/switchwall.sh"
        info "Copied switchwall.sh to $CONFIG_DIR/hamr/scripts/"
    fi

    # Make scripts executable
    chmod +x "$SCRIPT_DIR/scripts/thumbnails/thumbgen.sh" 2>/dev/null || true
    chmod +x "$SCRIPT_DIR/scripts/thumbnails/thumbgen.py" 2>/dev/null || true
    chmod +x "$SCRIPT_DIR/scripts/ocr/ocr-index.sh" 2>/dev/null || true
    chmod +x "$SCRIPT_DIR/scripts/ocr/ocr-index.py" 2>/dev/null || true
    chmod +x "$SCRIPT_DIR/scripts/colors/switchwall.sh" 2>/dev/null || true

    info "Installation complete!"
    echo ""
    echo "Start hamr with:"
    echo "  qs -c hamr"
    echo ""
    
    # Detect compositor and show appropriate instructions
    if [[ -n "${HYPRLAND_INSTANCE_SIGNATURE:-}" ]]; then
        show_hyprland_instructions
    elif [[ -n "${NIRI_SOCKET:-}" ]]; then
        show_niri_instructions
    else
        echo "Add to your compositor config for autostart."
        echo ""
        echo "For Hyprland, run: ./install.sh --hyprland-config"
        echo "For Niri, run: ./install.sh --niri-config"
    fi
}

show_hyprland_instructions() {
    echo "Hyprland detected! Add to ~/.config/hypr/hyprland.conf:"
    echo ""
    echo "  # Autostart hamr"
    echo "  exec-once = qs -c hamr"
    echo ""
    echo "  # Toggle hamr with Super key"
    echo "  bind = SUPER, SUPER_L, global, quickshell:hamrToggle"
    echo "  bindr = SUPER, SUPER_L, global, quickshell:hamrToggleRelease"
    echo ""
    echo "  # Or with Ctrl+Space"
    echo "  bind = CTRL, Space, global, quickshell:hamrToggle"
    echo ""
}

show_niri_instructions() {
    echo "Niri detected!"
    echo ""
    echo "1. Enable systemd service (recommended):"
    echo "   ./install.sh --enable-service"
    echo ""
    echo "2. Add keybinding to ~/.config/niri/config.kdl:"
    echo ""
    echo "   binds {"
    echo "       // Toggle hamr with Ctrl+Space"
    echo "       Ctrl+Space { spawn \"qs\" \"ipc\" \"call\" \"hamr\" \"toggle\"; }"
    echo ""
    echo "       // Or with Super key (Mod key)"
    echo "       Mod+Space { spawn \"qs\" \"ipc\" \"call\" \"hamr\" \"toggle\"; }"
    echo "   }"
    echo ""
}

install_systemd_service() {
    local service_src="$SCRIPT_DIR/hamr.service"
    local service_dest="$HOME/.config/systemd/user/hamr.service"
    
    if [[ ! -f "$service_src" ]]; then
        error "hamr.service not found in $SCRIPT_DIR"
    fi
    
    mkdir -p "$HOME/.config/systemd/user"
    cp "$service_src" "$service_dest"
    info "Installed systemd service: $service_dest"
    
    systemctl --user daemon-reload
    systemctl --user enable hamr.service
    systemctl --user add-wants niri.service hamr.service
    info "Enabled hamr.service (will start with niri.service)"
    
    echo ""
    echo "To start now: systemctl --user start hamr.service"
    echo "To check status: systemctl --user status hamr.service"
    echo "To view logs: journalctl --user -u hamr.service -f"
}

disable_systemd_service() {
    systemctl --user stop hamr.service 2>/dev/null || true
    systemctl --user disable hamr.service 2>/dev/null || true
    # Remove the wants link
    rm -f "$HOME/.config/systemd/user/niri.service.wants/hamr.service" 2>/dev/null || true
    rm -f "$HOME/.config/systemd/user/hamr.service"
    systemctl --user daemon-reload
    info "Disabled and removed hamr.service"
}

update_hamr() {
    info "Updating hamr..."

    if [[ ! -d "$SCRIPT_DIR/.git" ]]; then
        error "Not a git repository. Cannot update."
    fi

    cd "$SCRIPT_DIR"
    
    # Check for local changes
    if [[ -n $(git status --porcelain) ]]; then
        warn "Local changes detected:"
        git status --short
        echo ""
        echo "Options:"
        echo "  1. Stash changes:  git stash && ./install.sh -U && git stash pop"
        echo "  2. Commit changes: git add -A && git commit -m 'local changes'"
        echo "  3. Discard changes: git checkout -- ."
        echo ""
        error "Please resolve local changes before updating."
    fi

    git pull --rebase

    info "Update complete!"
    echo ""
    echo "Restart quickshell to apply changes:"
    echo "  qs -c hamr"
}

uninstall_hamr() {
    info "Uninstalling hamr..."

    if [[ -L "$HAMR_LINK" ]]; then
        rm "$HAMR_LINK"
        info "Removed symlink: $HAMR_LINK"
    elif [[ -d "$HAMR_LINK" ]]; then
        warn "Found directory instead of symlink at $HAMR_LINK"
        read -p "Remove it? [y/N] " -n 1 -r
        echo
        [[ $REPLY =~ ^[Yy]$ ]] && rm -rf "$HAMR_LINK"
    else
        warn "No hamr installation found at $HAMR_LINK"
    fi

    info "Uninstall complete. User data in $CONFIG_DIR/hamr/ was preserved."
}

# Main
case "${1:-}" in
    --update|-U)
        update_hamr
        ;;
    --uninstall|-u)
        uninstall_hamr
        ;;
    --check|-c)
        check_dependencies
        ;;
    --hyprland-config)
        show_hyprland_instructions
        ;;
    --niri-config)
        show_niri_instructions
        ;;
    --enable-service)
        install_systemd_service
        ;;
    --disable-service)
        disable_systemd_service
        ;;
    --help|-h)
        echo "Usage: $0 [OPTIONS]"
        echo ""
        echo "Options:"
        echo "  --check, -c        Check dependencies only"
        echo "  --update, -U       Update hamr via git pull"
        echo "  --uninstall, -u    Remove hamr installation"
        echo "  --hyprland-config  Show Hyprland configuration instructions"
        echo "  --niri-config      Show Niri configuration instructions"
        echo "  --enable-service   Install and enable systemd user service (for Niri)"
        echo "  --disable-service  Disable and remove systemd user service"
        echo "  --help, -h         Show this help"
        echo ""
        echo "Without options, installs hamr by creating a symlink in ~/.config/quickshell/"
        echo ""
        echo "Supported compositors:"
        echo "  - Hyprland (full support with global shortcuts)"
        echo "  - Niri (full support via IPC + systemd)"
        ;;
    *)
        check_dependencies
        install_hamr
        ;;
esac
