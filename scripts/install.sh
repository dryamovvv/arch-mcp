#!/bin/bash
set -euo pipefail

REPO="dryamovvv/arch-mcp"
ARCH=$(uname -m)
TMP_PKG="/tmp/arch-ops-server.pkg.tar.zst"

echo "arch-ops-server installer for Arch Linux"
echo "Architecture: $ARCH"

LATEST_TAG=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" | python3 -c "import sys,json; print(json.load(sys.stdin)['tag_name'])")
VER="${LATEST_TAG#v}"
URL_ZST="https://github.com/$REPO/releases/download/$LATEST_TAG/arch-ops-server-${VER}-1-any.pkg.tar.zst"
URL_XZ="https://github.com/$REPO/releases/download/$LATEST_TAG/arch-ops-server-${VER}-1-any.pkg.tar.xz"

echo "Trying prebuilt package arch-ops-server $VER..."

if curl -fsSL -o "$TMP_PKG" "$URL_ZST"; then
    echo "Downloaded prebuilt package (.zst)."
    echo "Installing..."
    sudo pacman -U --noconfirm "$TMP_PKG" || {
        echo "Install failed. Try building from source:"
        echo "  git clone https://github.com/$REPO.git && cd arch-mcp/packaging/arch && makepkg -si"
        exit 1
    }
    rm -f "$TMP_PKG"
elif curl -fsSL -o "$TMP_PKG" "$URL_XZ"; then
    echo "Downloaded prebuilt package (.xz)."
    echo "Installing..."
    sudo pacman -U --noconfirm "$TMP_PKG" || {
        echo "Install failed. Try building from source:"
        echo "  git clone https://github.com/$REPO.git && cd arch-mcp/packaging/arch && makepkg -si"
        exit 1
    }
    rm -f "$TMP_PKG"
else
    echo "No prebuilt package found for $ARCH. Building from source..."
    if ! command -v makepkg &>/dev/null; then
        echo "Installing build dependencies (base-devel)..."
        sudo pacman -S --noconfirm --needed base-devel
    fi
    if ! command -v fakeroot &>/dev/null; then
        echo "Installing fakeroot..."
        sudo pacman -S --noconfirm --needed fakeroot
    fi

    TMP_DIR=$(mktemp -d)
    git clone "https://github.com/$REPO.git" "$TMP_DIR"
    cd "$TMP_DIR/packaging/arch"
    makepkg -s --noconfirm
    PACKAGE=$(find . -name "arch-ops-server-*.pkg.tar.*" | head -1)
    if [ -z "$PACKAGE" ]; then
        echo "ERROR: Package build failed"
        exit 1
    fi
    sudo pacman -U --noconfirm "$PACKAGE"
    rm -rf "$TMP_DIR"
    echo "Done. Built and installed from source."
fi

echo "Server status:"
systemctl status arch-ops-server.service --no-pager 2>/dev/null || echo "Check server with: systemctl status arch-ops-server"
