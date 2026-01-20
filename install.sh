#!/usr/bin/env bash
#
# Hamr Launcher Installer
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/stewart86/hamr/main/install.sh | bash
#   ./install.sh  # If running from cloned repository
#
# Options (via environment variables):
#   HAMR_VERSION=v0.1.0    Install specific version (default: latest)
#   HAMR_DIR=~/.local      Install directory (default: ~/.local)
#   HAMR_NO_MODIFY_PATH=1  Don't add to PATH via shell rc
#   HAMR_SKIP_INSTALL=1    Download only, don't run `hamr install`
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
SKIP_INSTALL="${HAMR_SKIP_INSTALL:-}"

info() { printf "${BLUE}==>${NC} %s\n" "$*"; }
success() { printf "${GREEN}==>${NC} %s\n" "$*"; }
warn() { printf "${YELLOW}Warning:${NC} %s\n" "$*"; }
error() { printf "${RED}Error:${NC} %s\n" "$*" >&2; exit 1; }

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

main() {
    echo ""
    info "Installing Hamr Launcher"
    echo ""

    # Check requirements
    check_requirements

    # Check if local clone
    if is_local_clone; then
        info "Detected local repository clone, building from source..."
        # Build from local source
        if ! cargo build --release; then
            error "Failed to build from local source"
        fi

        # Use local build directory
        local local_build_dir="target/release"
        if [[ ! -d "$local_build_dir" ]]; then
            error "Build directory not found: $local_build_dir"
        fi

        # Create installation directories
        local bin_dir="$INSTALL_DIR/bin"
        mkdir -p "$bin_dir"

        # Install binaries from local build
        info "Installing binaries from local build to $bin_dir..."
        for binary in hamr hamr-daemon hamr-gtk hamr-tui; do
            if [[ -f "$local_build_dir/$binary" ]]; then
                cp "$local_build_dir/$binary" "$bin_dir/"
                chmod +x "$bin_dir/$binary"
            fi
        done

        # Install plugins directory if it exists
        if [[ -d "plugins" ]]; then
            info "Installing plugins..."
            cp -r "plugins" "$bin_dir/../"
        fi

        success "Binaries installed from local build to $bin_dir"

        # Add to PATH if needed
        if [[ -z "$NO_MODIFY_PATH" ]]; then
            add_to_path "$bin_dir"
        fi

        # Run hamr install to set up config and systemd
        if [[ -z "$SKIP_INSTALL" ]]; then
            echo ""
            info "Running 'hamr install' to set up config and services..."
            echo ""

            # Add bin_dir to PATH for this invocation
            export PATH="$bin_dir:$PATH"

            if ! "$bin_dir/hamr" install; then
                warn "'hamr install' encountered issues. You may need to run it manually."
            fi
        else
            echo ""
            info "Skipping 'hamr install' (HAMR_SKIP_INSTALL=1)"
            info "Run 'hamr install' manually to set up config and systemd services"
        fi

        echo ""
        success "Installation complete!"
        echo ""
        echo "Quick start:"
        echo "  hamr                    # Start GTK launcher (auto-starts daemon)"
        echo "  hamr toggle             # Toggle visibility (bind to Super+Space)"
        echo "  hamr plugin clipboard   # Open clipboard manager"
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

    # Create installation directories
    local bin_dir="$INSTALL_DIR/bin"
    mkdir -p "$bin_dir"

    # Install binaries
    info "Installing binaries to $bin_dir..."
    for binary in hamr hamr-daemon hamr-gtk hamr-tui; do
        if [[ -f "$extract_dir/$binary" ]]; then
            cp "$extract_dir/$binary" "$bin_dir/"
            chmod +x "$bin_dir/$binary"
        fi
    done

    # Install plugins directory next to binaries (for plugin discovery)
    if [[ -d "$extract_dir/plugins" ]]; then
        info "Installing plugins..."
        cp -r "$extract_dir/plugins" "$bin_dir/../"
    fi

    success "Binaries installed to $bin_dir"

    # Add to PATH if needed
    if [[ -z "$NO_MODIFY_PATH" ]]; then
        add_to_path "$bin_dir"
    fi

    # Run hamr install to set up config and systemd
    if [[ -z "$SKIP_INSTALL" ]]; then
        echo ""
        info "Running 'hamr install' to set up config and services..."
        echo ""

        # Add bin_dir to PATH for this invocation
        export PATH="$bin_dir:$PATH"

        if ! "$bin_dir/hamr" install; then
            warn "'hamr install' encountered issues. You may need to run it manually."
        fi
    else
        echo ""
        info "Skipping 'hamr install' (HAMR_SKIP_INSTALL=1)"
        info "Run 'hamr install' manually to set up config and systemd services"
    fi

    echo ""
    success "Installation complete!"
    echo ""
    echo "Quick start:"
    echo "  hamr                    # Start GTK launcher (auto-starts daemon)"
    echo "  hamr toggle             # Toggle visibility (bind to Super+Space)"
    echo "  hamr plugin clipboard   # Open clipboard manager"
    echo ""
    echo "Keybinding examples (Hyprland):"
    echo "  exec-once = hamr        # Auto-start on login"
    echo "  bind = SUPER, Space, exec, hamr toggle"
    echo ""
    echo "For more info: https://github.com/${REPO}"
}

main "$@"
