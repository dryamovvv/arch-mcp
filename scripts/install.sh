#!/bin/bash
set -euo pipefail

REPO="dryamovvv/arch-mcp"
ARCH=$(uname -m)
TMP_PKG="/tmp/arch-ops-server.pkg.tar.xz"
PACKAGE_URL_PATH="arch-ops-server-${VER:-unknown}-1-any.pkg.tar.xz"

echo "arch-ops-server installer for Arch Linux"
echo "Architecture: $ARCH"

LATEST_TAG=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" | python3 -c "import sys,json; print(json.load(sys.stdin)['tag_name'])")
VER="${LATEST_TAG#v}"
URL="https://github.com/$REPO/releases/download/$LATEST_TAG/arch-ops-server-${VER}-1-any.pkg.tar.xz"

echo "Trying prebuilt package arch-ops-server $VER..."

if curl -fsSL -o "$TMP_PKG" "$URL"; then
    echo "Downloaded prebuilt package."
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
    makepkg -si --noconfirm
    rm -rf "$TMP_DIR"
    echo "Done. Built and installed from source."
fi

echo "Server status:"
systemctl status arch-ops-server.service --no-pager 2>/dev/null || echo "Check server with: systemctl status arch-ops-server"
