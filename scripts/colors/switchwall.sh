#!/usr/bin/env bash
# switchwall.sh - Standalone wallpaper and theme switcher for Hamr
#
# This is a simplified standalone version. For advanced theming with
# Material You color generation, see end-4's illogical-impulse:
# https://github.com/end-4/dots-hyprland
#
# Usage:
#   switchwall.sh --image /path/to/image.jpg [--mode dark|light]
#   switchwall.sh --mode dark|light --noswitch
#   switchwall.sh --color [hex_color|clear]
#
# Supported wallpaper backends (auto-detected):
#   - swww (recommended for Hyprland)
#   - hyprpaper (via hyprctl)
#   - swaybg
#   - feh (X11)

set -euo pipefail

# Detect available wallpaper backend
detect_backend() {
    if command -v swww &>/dev/null; then
        if swww query &>/dev/null 2>&1; then
            echo "swww"
            return
        fi
    fi
    if command -v hyprctl &>/dev/null; then
        if hyprctl hyprpaper listloaded &>/dev/null 2>&1; then
            echo "hyprpaper"
            return
        fi
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
        swww)
            swww img "$image" --transition-type fade --transition-duration 1
            ;;
        hyprpaper)
            hyprctl hyprpaper preload "$image"
            hyprctl hyprpaper wallpaper ",$image"
            ;;
        swaybg)
            pkill swaybg || true
            swaybg -i "$image" -m fill &
            ;;
        feh)
            feh --bg-fill "$image"
            ;;
        none)
            notify-send "Wallpaper" "No wallpaper backend found. Install swww, hyprpaper, swaybg, or feh."
            return 1
            ;;
    esac
}

# Set color scheme (dark/light mode)
set_color_scheme() {
    local mode="$1"
    
    if command -v gsettings &>/dev/null; then
        if [[ "$mode" == "dark" ]]; then
            gsettings set org.gnome.desktop.interface color-scheme 'prefer-dark'
            # Try to set GTK theme if adw-gtk3 is available
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
    local mode=""
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
                # Color support is a placeholder - requires matugen or similar
                # For full Material You theming, use illogical-impulse
                if [[ -n "${2:-}" && "$2" != --* ]]; then
                    color="$2"
                    shift 2
                else
                    notify-send "Accent Color" "Color theming requires matugen. See illogical-impulse for full support."
                    shift
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
    
    # Set color scheme if mode specified
    if [[ -n "$mode" ]]; then
        set_color_scheme "$mode"
    fi
    
    # Set wallpaper unless --noswitch
    if [[ -z "$noswitch" && -n "$image" ]]; then
        if [[ -f "$image" ]]; then
            set_wallpaper "$image"
        else
            notify-send "Wallpaper" "File not found: $image"
            exit 1
        fi
    fi
    
    # Handle color flag (placeholder)
    if [[ -n "$color" ]]; then
        notify-send "Accent Color" "Set to $color (requires matugen for actual theming)"
    fi
}

main "$@"
