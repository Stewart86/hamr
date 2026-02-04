#!/usr/bin/env bash
#
# Hamr Launcher Installer
#
# Usage:
#   curl -fsSL https://hamr.run/install.sh | bash
#   ./install.sh  # If running from cloned repository
#
# Options:
#   --reset-user-data        Reset user configuration and plugins (backup created)
#   --check                  Dry-run mode: show what would be installed without making changes
#   --yes                    Assume yes for all prompts (non-interactive mode)
#   --systemd                Set up systemd user services via `hamr install`
#
# Environment variables:
#   HAMR_VERSION=v0.1.0    Install specific version (default: latest)
#   HAMR_DIR=~/.local      Install directory (default: ~/.local)
#   HAMR_NO_MODIFY_PATH=1  Don't add to PATH via shell rc
#   HAMR_SYSTEMD=1         Run `hamr install` to set up systemd services (opt-in)
#   HAMR_SKIP_INSTALL=1    (legacy) Skip running `hamr install`
#

set -euo pipefail

# ANSI color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Repository information
REPO="stewart86/hamr"
BINARY_NAME="hamr"

# Configuration
VERSION="${HAMR_VERSION:-}"
INSTALL_DIR="${HAMR_DIR:-$HOME/.local}"
NO_MODIFY_PATH="${HAMR_NO_MODIFY_PATH:-}"
SETUP_SYSTEMD="${HAMR_SYSTEMD:-}"
SKIP_INSTALL="${HAMR_SKIP_INSTALL:-}"
RESET_USER_DATA=""
DRY_RUN=""
ASSUME_YES=""

info() { printf "${BLUE}==>${NC} %s\n" "$*"; }
success() { printf "${GREEN}==>${NC} %s\n" "$*"; }
warn() { printf "${YELLOW}Warning:${NC} %s\n" "$*"; }
error() { printf "${RED}Error:${NC} %s\n" "$*" >&2; exit 1; }

# Prompt for user confirmation
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
        
        if [[ -n "$DRY_RUN" ]]; then
            echo "(dry-run: would prompt)"
            return 0
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

# Print dry-run summary
print_summary() {
    local title="$1"
    local content="$2"
    local indent="    "
    
    if [[ -n "$DRY_RUN" ]]; then
        printf "${BLUE}==>${NC} %s:\n" "$title"
        echo "$content" | sed "s/^/$indent/"
        echo ""
    fi
}

# Check for file conflicts and prompt for overwrite
check_file_conflicts() {
    local bin_dir="$1"
    local conflicts=()
    
    # Check for existing binaries
    for binary in hamr hamr-daemon hamr-gtk hamr-tui; do
        if [[ -f "$bin_dir/$binary" ]]; then
            conflicts+=("$bin_dir/$binary")
        fi
    done
    
    # Check for existing plugins
    if [[ -d "$bin_dir/plugins" ]]; then
        conflicts+=("$bin_dir/plugins")
    fi
    
    if [[ ${#conflicts[@]} -gt 0 ]]; then
        warn "The following files/directories already exist:"
        for conflict in "${conflicts[@]}"; do
            echo "  - $conflict"
        done
        
        if ! prompt_yes_no "Overwrite existing files?" "n"; then
            error "Installation cancelled by user"
        fi
    fi
}

# Parse command line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --systemd)
                SETUP_SYSTEMD=1
                shift
                ;;
            --reset-user-data)
                RESET_USER_DATA=1
                shift
                ;;
            --check)
                DRY_RUN=1
                shift
                ;;
            --yes)
                ASSUME_YES=1
                shift
                ;;
            *)
                error "Unknown option: $1"
                ;;
        esac
    done
}

# Detect architecture
detect_arch() {
    local arch
    arch=$(uname -m)
    case "$arch" in
        x86_64) echo "x86_64" ;;
        aarch64|arm64) echo "aarch64" ;;
        *) error "Unsupported architecture: $arch (supported: x86_64, aarch64)" ;;
    esac
}

# Detect OS
detect_os() {
    local os
    os=$(uname -s)
    case "$os" in
        Linux) echo "linux" ;;
        *) error "Unsupported OS: $os (only Linux is supported)" ;;
    esac
}

# Check for required commands
check_requirements() {
    local missing=()

    # Always need curl and tar for remote install
    for cmd in curl tar; do
        if ! command -v "$cmd" &>/dev/null; then
            missing+=("$cmd")
        fi
    done

    # Need cargo for local builds
    if is_local_clone && ! command -v cargo &>/dev/null; then
        missing+=("cargo")
    fi

    if [[ ${#missing[@]} -gt 0 ]]; then
        error "Missing required commands: ${missing[*]}"
    fi
}

# Get the latest release version from GitHub
get_latest_version() {
    local url="https://api.github.com/repos/${REPO}/releases/latest"
    local version

    version=$(curl -fsSL "$url" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')

    if [[ -z "$version" ]]; then
        error "Failed to get latest version from GitHub"
    fi

    echo "$version"
}

# Verify checksums
verify_checksum() {
    local archive="$1"
    local checksums_file="$2"
    local expected

    if ! command -v sha256sum &>/dev/null; then
        warn "sha256sum not found, skipping checksum verification"
        return 0
    fi

    expected=$(grep "$(basename "$archive")" "$checksums_file" | awk '{print $1}')
    if [[ -z "$expected" ]]; then
        warn "No checksum found for $(basename "$archive"), skipping verification"
        return 0
    fi

    local actual
    actual=$(sha256sum "$archive" | awk '{print $1}')

    if [[ "$expected" != "$actual" ]]; then
        error "Checksum mismatch for $archive\n  Expected: $expected\n  Actual:   $actual"
    fi

    success "Checksum verified"
}

# Add to PATH in shell rc file
add_to_path() {
    local bin_dir="$1"
    local rc_file=""
    local shell_name=""

    # Detect shell
    shell_name=$(basename "$SHELL")
    case "$shell_name" in
        bash) rc_file="$HOME/.bashrc" ;;
        zsh)  rc_file="$HOME/.zshrc" ;;
        fish) rc_file="$HOME/.config/fish/config.fish" ;;
        *)
            warn "Unknown shell: $shell_name. Please add $bin_dir to your PATH manually."
            return
            ;;
    esac

    # Check if already in PATH
    if [[ ":$PATH:" == *":$bin_dir:"* ]]; then
        return 0
    fi

    # Check if rc file already has the path
    if [[ -f "$rc_file" ]] && grep -q "$bin_dir" "$rc_file" 2>/dev/null; then
        return 0
    fi

    # Add to rc file
    local path_line
    if [[ "$shell_name" == "fish" ]]; then
        path_line="set -gx PATH $bin_dir \$PATH"
    else
        path_line="export PATH=\"$bin_dir:\$PATH\""
    fi

    echo "" >> "$rc_file"
    echo "# Added by hamr installer" >> "$rc_file"
    echo "$path_line" >> "$rc_file"

    info "Added $bin_dir to PATH in $rc_file"
    info "Run 'source $rc_file' or start a new terminal to use hamr"
}

# Check if running from local repository clone
is_local_clone() {
    [[ -d ".git" ]] && [[ -f "Cargo.toml" ]] && command -v cargo &>/dev/null
}

# Check if systemd services are running
check_systemd_services() {
    local services_running=""

    # Check user services
    if systemctl --user is-active --quiet hamr-daemon 2>/dev/null; then
        services_running="user"
    fi

    # Check system services (less common)
    if systemctl is-active --quiet hamr-daemon 2>/dev/null; then
        services_running="system"
    fi

    echo "$services_running"
}

# Stop systemd services before installation
stop_services() {
    local service_type="$1"
    if [[ "$service_type" == "system" ]]; then
        info "Stopping system hamr services..."
        sudo systemctl stop hamr-daemon hamr-gtk 2>/dev/null || true
    elif [[ "$service_type" == "user" ]]; then
        info "Stopping user hamr services..."
        systemctl --user stop hamr-daemon hamr-gtk 2>/dev/null || true
    fi
}

# Kill running hamr processes to prevent "Text file busy" errors during install
kill_hamr_processes() {
    local processes
    processes=$(pgrep -f "hamr-(daemon|gtk|tui)" || true)
    if [[ -n "$processes" ]]; then
        info "Stopping running hamr processes..."
        killall hamr hamr-daemon hamr-gtk hamr-tui 2>/dev/null || true
        sleep 1
    fi
}

# Start systemd services after installation
start_services() {
    local service_type="$1"
    if [[ "$service_type" == "system" ]]; then
        info "Starting system hamr services..."
        sudo systemctl start hamr-daemon hamr-gtk 2>/dev/null || true
    elif [[ "$service_type" == "user" ]]; then
        import_user_environment
        info "Starting user hamr services..."
        systemctl --user start hamr-daemon hamr-gtk 2>/dev/null || true
    fi
}

import_user_environment() {
    if command -v systemctl &>/dev/null; then
        systemctl --user import-environment \
            NIRI_SOCKET \
            HYPRLAND_INSTANCE_SIGNATURE \
            SWAYSOCK \
            WAYLAND_DISPLAY \
            XDG_CURRENT_DESKTOP \
            XDG_SESSION_DESKTOP \
            DISPLAY \
            XDG_RUNTIME_DIR \
            || warn "Failed to import session environment"
    fi
}

reload_user_systemd() {
    if command -v systemctl &>/dev/null; then
        import_user_environment
        info "Reloading systemd user daemon..."
        if systemctl --user daemon-reload; then
            info "Restarting hamr user services..."
            systemctl --user restart hamr-daemon hamr-gtk || warn "Failed to restart hamr services"
        else
            warn "Failed to reload systemd user daemon"
        fi
    fi
}

main() {
    echo ""
    info "Installing Hamr Launcher"
    echo ""

    # Parse command line arguments
    parse_args "$@"

    # Check requirements
    check_requirements

    # Check if local clone
    if is_local_clone; then
        info "Detected local repository clone, building from source..."
        
        # Dry-run summary for local build
        if [[ -n "$DRY_RUN" ]]; then
            local summary="Platform: $(detect_os)-$(detect_arch)
Installation directory: $INSTALL_DIR/bin
Build from source: yes (local repository)
Reset user data: $([ -n "$RESET_USER_DATA" ] && echo "yes" || echo "no")
Systemd setup: $([ -n "$SETUP_SYSTEMD" ] && echo "yes" || echo "no")
Modify PATH: $([ -n "$NO_MODIFY_PATH" ] && echo "no" || echo "yes")"
            print_summary "Installation Summary" "$summary"
            
            info "Dry-run mode: would build from source and install to $INSTALL_DIR/bin"
            return 0
        fi
        
        # Build from local source
        if ! cargo build --release; then
            error "Failed to build from local source"
        fi

        # Use local build directory
        local local_build_dir="target/release"
        if [[ ! -d "$local_build_dir" ]]; then
            error "Build directory not found: $local_build_dir"
        fi

        # Check for running services
        local running_services
        running_services=$(check_systemd_services)

        # Check for file conflicts
        local bin_dir="$INSTALL_DIR/bin"
        check_file_conflicts "$bin_dir"

        # Stop services if running (only if not dry-run)
        if [[ -n "$running_services" ]] && [[ -z "$DRY_RUN" ]]; then
            stop_services "$running_services"
        fi

        # Kill running processes to prevent "Text file busy" errors (only if not dry-run)
        if [[ -z "$DRY_RUN" ]]; then
            kill_hamr_processes
        fi

        # Create installation directories (only if not dry-run)
        if [[ -z "$DRY_RUN" ]]; then
            mkdir -p "$bin_dir"
        fi

        # Install binaries from local build
        info "Installing binaries from local build to $bin_dir..."
        for binary in hamr hamr-daemon hamr-gtk hamr-tui; do
            if [[ -f "$local_build_dir/$binary" ]]; then
                if [[ -n "$DRY_RUN" ]]; then
                    print_summary "Would install" "$binary to $bin_dir/"
                else
                    cp "$local_build_dir/$binary" "$bin_dir/"
                    chmod +x "$bin_dir/$binary"
                fi
            fi
        done

        # Restart services if they were running (only if not dry-run)
        if [[ -n "$running_services" ]] && [[ -z "$DRY_RUN" ]]; then
            start_services "$running_services"
        fi

        # Install plugins directory if it exists
        if [[ -d "plugins" ]]; then
            info "Installing plugins..."
            
            # Backup user data if --reset-user-data is set (only if not dry-run)
            if [[ -n "$RESET_USER_DATA" ]] && [[ -z "$DRY_RUN" ]]; then
                if [[ -d "$HOME/.config/hamr" ]]; then
                    local backup_dir="$HOME/.config/hamr.backup.$(date +%Y%m%d_%H%M%S)"
                    info "Backing up existing user config to $backup_dir..."
                    cp -r "$HOME/.config/hamr" "$backup_dir"
                fi
                # Remove old plugins to ensure clean update
                rm -rf "$bin_dir/plugins"
                rm -rf "$HOME/.config/hamr/plugins"
            else
                # Preserve user config and plugins by default
                info "Preserving existing user configuration and plugins..."
            fi
            
            if [[ -n "$DRY_RUN" ]]; then
                # Dry-run plugin installation summary
                local plugin_summary=""
                for plugin_dir in plugins/*/; do
                    if [[ -d "$plugin_dir" ]]; then
                        local plugin_name=$(basename "$plugin_dir")
                        plugin_summary="${plugin_summary}System plugin: $plugin_name\n"
                    fi
                done
                if [[ -n "$RESET_USER_DATA" ]] || [[ ! -d "$HOME/.config/hamr/plugins" ]]; then
                    plugin_summary="${plugin_summary}User plugins: would copy to ~/.config/hamr/plugins/"
                fi
                print_summary "Plugin Installation" "$plugin_summary"
            else
                # Install system plugins (always update these)
                cp -r "plugins" "$bin_dir/"
                
                # Only copy plugins to user config if they don't exist or if --reset-user-data
                if [[ -n "$RESET_USER_DATA" ]] || [[ ! -d "$HOME/.config/hamr/plugins" ]]; then
                    mkdir -p "$HOME/.config/hamr/plugins"
                    cp -r "plugins"/* "$HOME/.config/hamr/plugins/" 2>/dev/null || true
                fi
            fi
            
            # Make handler scripts executable based on manifest (only if not dry-run)
            if [[ -z "$DRY_RUN" ]]; then
                for manifest in "$bin_dir/plugins"/*/manifest.json; do
                    if [[ -f "$manifest" ]]; then
                        handler_cmd=$(grep -o '"command"[[:space:]]*:[[:space:]]*"[^"]*"' "$manifest" | sed 's/.*"\([^"]*\)"$/\1/' | tail -1)
                        if [[ -n "$handler_cmd" ]]; then
                            handler_file=$(echo "$handler_cmd" | awk '{print $NF}')
                            plugin_dir=$(basename "$(dirname "$manifest")")
                            if [[ -f "$bin_dir/plugins/$plugin_dir/$handler_file" ]]; then
                                chmod +x "$bin_dir/plugins/$plugin_dir/$handler_file"
                            fi
                            if [[ -f "$HOME/.config/hamr/plugins/$plugin_dir/$handler_file" ]]; then
                                chmod +x "$HOME/.config/hamr/plugins/$plugin_dir/$handler_file"
                            fi
                        fi
                    fi
                done
            fi
        fi

        success "Binaries installed from local build to $bin_dir"

        # Add to PATH if needed (only if not dry-run)
        if [[ -z "$NO_MODIFY_PATH" ]] && [[ -z "$DRY_RUN" ]]; then
            add_to_path "$bin_dir"
        elif [[ -n "$DRY_RUN" ]] && [[ -z "$NO_MODIFY_PATH" ]]; then
            print_summary "PATH Update" "Would add $bin_dir to PATH in shell rc file"
        fi

        # Optional systemd setup via `hamr install` (opt-in)
        if [[ -n "$SETUP_SYSTEMD" ]] && [[ -z "$SKIP_INSTALL" ]] && [[ -z "$DRY_RUN" ]]; then
            echo ""
            info "Running 'hamr install' to set up config and services..."
            echo ""

            # Add bin_dir to PATH for this invocation
            export PATH="$bin_dir:$PATH"

            if ! "$bin_dir/hamr" install; then
                warn "'hamr install' encountered issues. You may need to run it manually."
            else
                reload_user_systemd
            fi
        elif [[ -n "$DRY_RUN" ]] && [[ -n "$SETUP_SYSTEMD" ]] && [[ -z "$SKIP_INSTALL" ]]; then
            print_summary "Post-Install" "Would run 'hamr install' to set up systemd services"
        fi

        echo ""
        success "Installation complete!"
        echo ""
        echo "Quick start:"
        echo "  hamr                    # Start GTK launcher (auto-starts daemon)"
        echo "  hamr toggle             # Toggle visibility (bind to Super+Space)"
        echo "  hamr plugin clipboard   # Open clipboard manager"
        echo ""
        echo "Recommended (opt-in): systemd user services"
        echo "  hamr install"
        echo "  systemctl --user start hamr-gtk"
        echo ""
        echo "Keybinding examples (Hyprland):"
        echo "  exec-once = hamr        # Auto-start on login"
        echo "  bind = SUPER, Space, exec, hamr toggle"
        echo ""
        echo "For more info: https://github.com/${REPO}"
        return 0
    fi

    # Detect platform
    local os arch
    os=$(detect_os)
    arch=$(detect_arch)
    info "Detected platform: $os-$arch"

    # Get version
    if [[ -z "$VERSION" ]]; then
        info "Fetching latest release..."
        VERSION=$(get_latest_version)
    fi
    info "Version: $VERSION"

    # Dry-run summary for remote install
    if [[ -n "$DRY_RUN" ]]; then
        local summary="Platform: $os-$arch
Version: $VERSION
Installation directory: $INSTALL_DIR/bin
Download from: GitHub releases
Reset user data: $([ -n "$RESET_USER_DATA" ] && echo "yes" || echo "no")
Systemd setup: $([ -n "$SETUP_SYSTEMD" ] && echo "yes" || echo "no")
Modify PATH: $([ -n "$NO_MODIFY_PATH" ] && echo "no" || echo "yes")"
        print_summary "Installation Summary" "$summary"
        
        info "Dry-run mode: would download and install from GitHub releases"
        return 0
    fi

    # Construct download URLs
    local archive_name="hamr-${os}-${arch}.tar.gz"
    local base_url="https://github.com/${REPO}/releases/download/${VERSION}"
    local archive_url="${base_url}/${archive_name}"
    local checksums_url="${base_url}/checksums.txt"

    # Create temp directory
    local tmp_dir
    tmp_dir=$(mktemp -d)
    trap 'rm -rf "$tmp_dir"' EXIT

    # Download archive
    info "Downloading $archive_name..."
    if ! curl -fsSL --progress-bar "$archive_url" -o "$tmp_dir/$archive_name"; then
        error "Failed to download $archive_url"
    fi

    # Download and verify checksums
    info "Verifying checksum..."
    if curl -fsSL "$checksums_url" -o "$tmp_dir/checksums.txt" 2>/dev/null; then
        verify_checksum "$tmp_dir/$archive_name" "$tmp_dir/checksums.txt"
    else
        warn "Could not download checksums, skipping verification"
    fi

    # Extract archive
    info "Extracting..."
    tar -xzf "$tmp_dir/$archive_name" -C "$tmp_dir"

    # Find extracted directory (hamr-linux-x86_64 or similar)
    local extract_dir
    extract_dir=$(find "$tmp_dir" -maxdepth 1 -type d -name "hamr-*" | head -1)
    if [[ -z "$extract_dir" ]]; then
        error "Could not find extracted directory"
    fi

    # Check for file conflicts
    local bin_dir="$INSTALL_DIR/bin"
    check_file_conflicts "$bin_dir"

    # Check for running services
    local running_services
    running_services=$(check_systemd_services)

    # Stop services if running
    if [[ -n "$running_services" ]]; then
        stop_services "$running_services"
    fi

    # Kill running processes to prevent "Text file busy" errors
    kill_hamr_processes

    # Create installation directories
    mkdir -p "$bin_dir"

    # Install binaries
    info "Installing binaries to $bin_dir..."
    for binary in hamr hamr-daemon hamr-gtk hamr-tui; do
        if [[ -f "$extract_dir/$binary" ]]; then
            cp "$extract_dir/$binary" "$bin_dir/"
            chmod +x "$bin_dir/$binary"
        fi
    done

    # Restart services if they were running
    if [[ -n "$running_services" ]]; then
        start_services "$running_services"
    fi

    # Install plugins directory next to binaries (for plugin discovery)
    if [[ -d "$extract_dir/plugins" ]]; then
        info "Installing plugins..."
        
        # Backup user data if --reset-user-data is set
        if [[ -n "$RESET_USER_DATA" ]]; then
            if [[ -d "$HOME/.config/hamr" ]]; then
                local backup_dir="$HOME/.config/hamr.backup.$(date +%Y%m%d_%H%M%S)"
                info "Backing up existing user config to $backup_dir..."
                cp -r "$HOME/.config/hamr" "$backup_dir"
            fi
            # Remove old plugins to ensure clean update
            rm -rf "$bin_dir/plugins"
            rm -rf "$HOME/.config/hamr/plugins"
        else
            # Preserve user config and plugins by default
            info "Preserving existing user configuration and plugins..."
        fi
        
        # Install system plugins (always update these)
        cp -r "$extract_dir/plugins" "$bin_dir/"
        
        # Only copy plugins to user config if they don't exist or if --reset-user-data
        if [[ -n "$RESET_USER_DATA" ]] || [[ ! -d "$HOME/.config/hamr/plugins" ]]; then
            mkdir -p "$HOME/.config/hamr/plugins"
            cp -r "$extract_dir/plugins"/* "$HOME/.config/hamr/plugins/" 2>/dev/null || true
        fi
        
        # Make handler scripts executable based on manifest
        for manifest in "$bin_dir/plugins"/*/manifest.json; do
            if [[ -f "$manifest" ]]; then
                handler_cmd=$(grep -o '"command"[[:space:]]*:[[:space:]]*"[^"]*"' "$manifest" | sed 's/.*"\([^"]*\)"$/\1/' | tail -1)
                if [[ -n "$handler_cmd" ]]; then
                    handler_file=$(echo "$handler_cmd" | awk '{print $NF}')
                    plugin_dir=$(basename "$(dirname "$manifest")")
                    if [[ -f "$bin_dir/plugins/$plugin_dir/$handler_file" ]]; then
                        chmod +x "$bin_dir/plugins/$plugin_dir/$handler_file"
                    fi
                    if [[ -f "$HOME/.config/hamr/plugins/$plugin_dir/$handler_file" ]]; then
                        chmod +x "$HOME/.config/hamr/plugins/$plugin_dir/$handler_file"
                    fi
                fi
            fi
        done
    fi

    success "Binaries installed to $bin_dir"

    # Add to PATH if needed
    if [[ -z "$NO_MODIFY_PATH" ]]; then
        add_to_path "$bin_dir"
    fi

    # Optional systemd setup via `hamr install` (opt-in)
    if [[ -n "$SETUP_SYSTEMD" ]] && [[ -z "$SKIP_INSTALL" ]]; then
        echo ""
        info "Running 'hamr install' to set up config and services..."
        echo ""

        # Add bin_dir to PATH for this invocation
        export PATH="$bin_dir:$PATH"

        if ! "$bin_dir/hamr" install; then
            warn "'hamr install' encountered issues. You may need to run it manually."
        else
            reload_user_systemd
        fi
    fi

    echo ""
    success "Installation complete!"
    echo ""
    echo "Quick start:"
    echo "  hamr                    # Start GTK launcher (auto-starts daemon)"
    echo "  hamr toggle             # Toggle visibility (bind to Super+Space)"
    echo "  hamr plugin clipboard   # Open clipboard manager"
    echo ""
    echo "Recommended (opt-in): systemd user services"
    echo "  hamr install"
    echo "  systemctl --user start hamr-gtk"
    echo ""
    echo "Keybinding examples (Hyprland):"
    echo "  exec-once = hamr        # Auto-start on login"
    echo "  bind = SUPER, Space, exec, hamr toggle"
    echo ""
    echo "For more info: https://github.com/${REPO}"
}

main "$@"
