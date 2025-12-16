#!/bin/bash
# Publish PKGBUILD to AUR
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
AUR_DIR="$SCRIPT_DIR/aur"

cd "$SCRIPT_DIR"

# Check if AUR repo exists
if [[ ! -d "$AUR_DIR/.git" ]]; then
    echo "AUR repo not found. Cloning..."
    git clone ssh://aur@aur.archlinux.org/hamr.git "$AUR_DIR"
    cd "$AUR_DIR"
    git branch -m main master 2>/dev/null || true
    cd "$SCRIPT_DIR"
fi

# Copy files
cp PKGBUILD hamr.install "$AUR_DIR/"

# Generate .SRCINFO and commit
cd "$AUR_DIR"
makepkg --printsrcinfo > .SRCINFO

# Check for changes
if git diff --quiet && git diff --cached --quiet; then
    echo "No changes to publish."
    exit 0
fi

# Show diff
git diff
echo ""
read -p "Publish to AUR? [y/N] " -n 1 -r
echo ""

if [[ $REPLY =~ ^[Yy]$ ]]; then
    git add PKGBUILD .SRCINFO hamr.install
    
    # Get version from PKGBUILD
    VERSION=$(grep '^pkgver=' PKGBUILD | cut -d'=' -f2)
    REL=$(grep '^pkgrel=' PKGBUILD | cut -d'=' -f2)
    
    git commit -m "Update to $VERSION-$REL"
    git push
    echo ""
    echo "Published: https://aur.archlinux.org/packages/hamr"
fi
