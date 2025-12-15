# This PKGBUILD is used by install.sh to define dependencies
# It is NOT meant to be built with makepkg directly
pkgname=hamr
pkgver=0.1.0
pkgrel=1
pkgdesc='Hamr - Extensible launcher for Hyprland built with Quickshell'
arch=(any)
license=(GPL3)
depends=(
    # Core (quickshell-git or illogical-impulse-quickshell-git)
    quickshell
    qt6-5compat  # Required for Qt5Compat.GraphicalEffects

    # Python runtime
    python
    python-click

    # Thumbnail generation
    python-loguru
    python-tqdm
    python-gobject
    gnome-desktop-4

    # Clipboard
    wl-clipboard
    cliphist

    # File search
    fd
    fzf

    # Desktop integration
    xdg-utils
    libnotify
    gtk3
    hyprland
    libpulse
    jq

    # Calculator
    libqalculate

    # Fonts
    ttf-material-symbols-variable
    ttf-jetbrains-mono-nerd
    ttf-readex-pro
)
optdepends=(
    'tesseract: OCR text extraction for screenshot search'
    'tesseract-data-eng: English OCR language data'
    'imagemagick: Alternative thumbnail generation'
    'bitwarden-cli: Bitwarden password manager integration'
    'slurp: Screen region selection for screenshots'
    'wf-recorder: Screen recording'
)
