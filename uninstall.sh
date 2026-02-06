#!/usr/bin/env bash
#
# Hamr Launcher Uninstaller
#
# Usage:
#   curl -fsSL https://hamr.run/uninstall.sh | bash
#   ./uninstall.sh
#
# Options:
#   --purge    Also remove user config and plugins (~/.config/hamr)
#   --yes      Assume yes for all prompts (non-interactive mode)
#
# Environment variables:
#   HAMR_DIR=~/.local    Install directory (default: ~/.local)
#

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m'

INSTALL_DIR="${HAMR_DIR:-$HOME/.local}"
PURGE=""
ASSUME_YES=""

info() { printf "${BLUE}==>${NC} %s\n" "$*"; }
success() { printf "${GREEN}==>${NC} %s\n" "$*"; }
warn() { printf "${YELLOW}Warning:${NC} %s\n" "$*"; }
error() { printf "${RED}Error:${NC} %s\n" "$*" >&2; exit 1; }

prompt_yes_no() {
    local prompt="$1"
    local default="${2:-n}"

    if [[ -n "$ASSUME_YES" ]]; then
        return 0
    fi

    while true; do
        if [[ "$default" == "y" ]]; then
            printf "${BLUE}==>${NC} %s [Y/n] " "$prompt"
        else
            printf "${BLUE}==>${NC} %s [y/N] " "$prompt"
        fi

        read -r response
        case "$response" in
            [yY]|[yY][eE][sS]) return 0 ;;
            [nN]|[nN][oO]) return 1 ;;
            "") [[ "$default" == "y" ]] && return 0 || return 1 ;;
            *) echo "Please answer yes or no." ;;
        esac
    done
}

parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --purge)
                PURGE=1
                shift
                ;;
            --yes)
                ASSUME_YES=1
                shift
                ;;
            *)
                error "Unknown option: $1\nUsage: uninstall.sh [--purge] [--yes]"
                ;;
        esac
    done
}

remove_file() {
    local path="$1"
    if [[ -f "$path" ]]; then
        rm -f "$path"
        echo "  Removed: $path"
    fi
}

remove_dir() {
    local path="$1"
    if [[ -d "$path" ]]; then
        rm -rf "$path"
        echo "  Removed: $path"
    fi
}

# Remove PATH entries added by install.sh from shell rc files
clean_shell_rc() {
    local bin_dir="$1"

    for rc_file in "$HOME/.bashrc" "$HOME/.zshrc" "$HOME/.config/fish/config.fish"; do
        [[ -f "$rc_file" ]] || continue
        grep -q "$bin_dir" "$rc_file" 2>/dev/null || continue

        # Remove the comment and PATH line added by the installer
        local tmp
        tmp=$(mktemp)
        grep -v "# Added by hamr installer" "$rc_file" \
            | grep -v "$bin_dir" > "$tmp" || true
        mv "$tmp" "$rc_file"
        echo "  Cleaned PATH from: $rc_file"
    done
}

main() {
    echo ""
    info "Uninstalling Hamr Launcher"
    echo ""

    parse_args "$@"

    local bin_dir="$INSTALL_DIR/bin"
    local config_dir="${XDG_CONFIG_HOME:-$HOME/.config}/hamr"
    local systemd_dir="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"
    local runtime_dir="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"

    # Show what will be removed
    echo "This will remove:"
    echo "  - Binaries in $bin_dir (hamr, hamr-daemon, hamr-gtk, hamr-tui)"
    echo "  - System plugins in $bin_dir/plugins/"
    echo "  - Systemd user services"
    echo "  - Socket files"
    echo "  - PATH entries from shell rc files"
    if [[ -n "$PURGE" ]]; then
        echo "  - User config and plugins in $config_dir (--purge)"
    else
        echo "  - User config in $config_dir will be PRESERVED"
    fi
    echo ""

    if ! prompt_yes_no "Proceed with uninstall?" "n"; then
        echo "Cancelled."
        exit 0
    fi

    echo ""

    # 1. Stop and disable systemd services
    info "Systemd services..."
    if command -v systemctl &>/dev/null; then
        for service in hamr-gtk hamr-daemon; do
            systemctl --user stop "$service" 2>/dev/null || true
            systemctl --user disable "$service" 2>/dev/null || true
        done
        echo "  Stopped and disabled hamr services"

        remove_file "$systemd_dir/hamr-daemon.service"
        remove_file "$systemd_dir/hamr-gtk.service"

        systemctl --user daemon-reload 2>/dev/null || true
        echo "  Reloaded systemd daemon"
    else
        echo "  systemctl not available, skipping"
    fi

    # 2. Kill running processes
    info "Stopping running processes..."
    killall hamr hamr-daemon hamr-gtk hamr-tui 2>/dev/null || true
    sleep 0.5

    # 3. Remove socket files
    info "Socket files..."
    remove_file "$runtime_dir/hamr.sock"
    remove_file "$runtime_dir/hamr-dev.sock"

    # 4. Remove binaries
    info "Binaries..."
    for binary in hamr hamr-daemon hamr-gtk hamr-tui; do
        remove_file "$bin_dir/$binary"
    done

    # 5. Remove system plugins (next to binaries)
    info "System plugins..."
    remove_dir "$bin_dir/plugins"

    # 6. Clean PATH from shell rc files
    info "Shell PATH..."
    clean_shell_rc "$bin_dir"

    # 7. User config
    if [[ -n "$PURGE" ]]; then
        info "User config (--purge)..."
        remove_dir "$config_dir"
    else
        echo ""
        info "User config preserved at: $config_dir"
        echo "  To remove: rm -rf $config_dir"
        echo "  Or re-run: ./uninstall.sh --purge"
    fi

    echo ""
    success "Hamr has been uninstalled."

    if [[ -z "$PURGE" ]]; then
        echo ""
        echo "Note: Your config and plugins are still at $config_dir"
        echo "Run with --purge to remove everything."
    fi

    echo ""
}

main "$@"
