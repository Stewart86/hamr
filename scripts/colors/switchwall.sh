#!/usr/bin/env bash
# switchwall.sh - Wallpaper and theme switcher for Hamr
#
# Usage:
#   switchwall.sh --image /path/to/image.jpg [--mode dark|light]
#   switchwall.sh --mode dark|light --noswitch
#   switchwall.sh --color <hex_color>
#
# Supported wallpaper backends (auto-detected):
#   - awww (recommended for Wayland)
#   - swww (legacy name for awww)
#   - hyprpaper (via hyprctl)
#   - swaybg
#   - feh (X11)
#
# Theme generation:
#   Uses matugen to generate Material You colors.
#   Outputs to DMS colors file for DankMaterialShell integration.

set -euo pipefail

DMS_COLORS_FILE="${XDG_CACHE_HOME:-$HOME/.cache}/DankMaterialShell/dms-colors.json"
NIRI_HAMR_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/niri/hamr"
NIRI_CONFIG="${XDG_CONFIG_HOME:-$HOME/.config}/niri/config.kdl"

# Setup niri hamr integration (creates dir and adds include if needed)
setup_niri() {
    [[ -z "${NIRI_SOCKET:-}" ]] && return
    
    # Create hamr directory
    mkdir -p "$NIRI_HAMR_DIR"
    
    # Create empty colors.kdl if it doesn't exist
    [[ -f "$NIRI_HAMR_DIR/colors.kdl" ]] || touch "$NIRI_HAMR_DIR/colors.kdl"
    
    # Add include to niri config if not present
    if [[ -f "$NIRI_CONFIG" ]] && ! grep -q 'include "hamr/colors.kdl"' "$NIRI_CONFIG"; then
        echo 'include "hamr/colors.kdl"' >> "$NIRI_CONFIG"
    fi
}

# Detect available wallpaper backend
detect_backend() {
    if command -v awww &>/dev/null; then
        if awww query &>/dev/null 2>&1; then
            echo "awww"
            return
        fi
    fi
    if command -v swww &>/dev/null; then
        if swww query &>/dev/null 2>&1; then
            echo "swww"
            return
        fi
    fi
    if command -v hyprctl &>/dev/null && pidof hyprpaper &>/dev/null; then
        echo "hyprpaper"
        return
    fi
    if command -v swaybg &>/dev/null; then
        echo "swaybg"
        return
    fi
    if command -v feh &>/dev/null; then
        echo "feh"
        return
    fi
    echo "none"
}

# Set wallpaper using detected backend
set_wallpaper() {
    local image="$1"
    local backend
    backend=$(detect_backend)
    
    case "$backend" in
        awww)
            awww img "$image" --transition-type fade --transition-duration 1
            ;;
        swww)
            swww img "$image" --transition-type fade --transition-duration 1
            ;;
        hyprpaper)
            # Set wallpaper on all monitors
            hyprctl monitors -j | jq -r '.[].name' | while read -r monitor; do
                hyprctl hyprpaper wallpaper "$monitor,$image"
            done
            ;;
        swaybg)
            pkill swaybg || true
            swaybg -i "$image" -m fill &
            ;;
        feh)
            feh --bg-fill "$image"
            ;;
        none)
            notify-send "Wallpaper" "No wallpaper backend found. Install awww, swww, hyprpaper, swaybg, or feh."
            return 1
            ;;
    esac
}

# Generate colors with matugen and output DMS-compatible format
# matugen outputs all variants (dark, default, light) in one run
generate_colors() {
    local source_type="$1"  # "image" or "color"
    local source_value="$2" # path or hex color
    
    if ! command -v matugen &>/dev/null; then
        return 1
    fi
    
    local tmp_json=$(mktemp)
    trap "rm -f '$tmp_json'" EXIT
    
    # Generate colors (matugen outputs dark/default/light variants for each color)
    if [[ "$source_type" == "image" ]]; then
        matugen image "$source_value" --mode dark --type scheme-tonal-spot --dry-run -j hex > "$tmp_json" 2>/dev/null
    else
        matugen color hex "$source_value" --mode dark --type scheme-tonal-spot --dry-run -j hex > "$tmp_json" 2>/dev/null
    fi
    
    # Convert to DMS format using jq
    # matugen format: { colors: { primary: { dark: "#xxx", light: "#xxx" }, ... } }
    # DMS format: { colors: { dark: { primary: "#xxx", ... }, light: { primary: "#xxx", ... } } }
    if command -v jq &>/dev/null && [[ -s "$tmp_json" ]]; then
        mkdir -p "$(dirname "$DMS_COLORS_FILE")"
        jq '{
            colors: {
                dark: (.colors | with_entries(.value = .value.dark)),
                light: (.colors | with_entries(.value = .value.light))
            }
        }' "$tmp_json" > "$DMS_COLORS_FILE"
    fi
    
    # Also run matugen normally for user templates (hamr, gtk, niri, etc.)
    if [[ "$source_type" == "image" ]]; then
        matugen image "$source_value" --mode dark --type scheme-tonal-spot
    else
        matugen color hex "$source_value" --mode dark --type scheme-tonal-spot
    fi
    
    # Reload niri config to apply new colors
    if [[ -n "${NIRI_SOCKET:-}" ]]; then
        setup_niri
        niri msg action load-config-file &>/dev/null || true
    fi
}

# Set color scheme (dark/light mode) via gsettings
set_color_scheme() {
    local mode="$1"
    
    if command -v gsettings &>/dev/null; then
        if [[ "$mode" == "dark" ]]; then
            gsettings set org.gnome.desktop.interface color-scheme 'prefer-dark'
            gsettings set org.gnome.desktop.interface gtk-theme 'adw-gtk3-dark' 2>/dev/null || true
        elif [[ "$mode" == "light" ]]; then
            gsettings set org.gnome.desktop.interface color-scheme 'prefer-light'
            gsettings set org.gnome.desktop.interface gtk-theme 'adw-gtk3' 2>/dev/null || true
        fi
    fi
}

# Main
main() {
    local image=""
    local mode="dark"
    local noswitch=""
    local color=""
    
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --image)
                image="$2"
                shift 2
                ;;
            --mode)
                mode="$2"
                shift 2
                ;;
            --noswitch)
                noswitch="1"
                shift
                ;;
            --color)
                if [[ -n "${2:-}" && "$2" != --* ]]; then
                    color="$2"
                    shift 2
                else
                    echo "Error: --color requires a hex color value"
                    exit 1
                fi
                ;;
            *)
                # Treat as image path if not a flag
                if [[ -z "$image" && -f "$1" ]]; then
                    image="$1"
                fi
                shift
                ;;
        esac
    done
    
    # Set gsettings color scheme
    set_color_scheme "$mode"
    
    # Handle color-only mode (no image)
    if [[ -n "$color" ]]; then
        generate_colors "color" "$color" || true
        return
    fi
    
    # Set wallpaper unless --noswitch
    if [[ -z "$noswitch" && -n "$image" ]]; then
        if [[ -f "$image" ]]; then
            set_wallpaper "$image"
            generate_colors "image" "$image" || true
        else
            notify-send "Wallpaper" "File not found: $image"
            exit 1
        fi
    elif [[ -n "$image" ]]; then
        # --noswitch with image: just generate colors, don't change wallpaper
        if [[ -f "$image" ]]; then
            generate_colors "image" "$image" || true
        fi
    fi
}

main "$@"
