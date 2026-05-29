#!/bin/bash
set -euo pipefail

REPO="dryamovvv/arch-mcp"
ARCH=$(uname -m)
TMP_PKG="/tmp/arch-ops-server.pkg.tar.xz"

echo "arch-ops-server installer for Arch Linux"
echo "Architecture: $ARCH"

LATEST_TAG=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" | python3 -c "import sys,json; print(json.load(sys.stdin)['tag_name'])")
VER="${LATEST_TAG#v}"
URL="https://github.com/$REPO/releases/download/$LATEST_TAG/arch-ops-server-${VER}-1-any.pkg.tar.xz"

echo "Downloading arch-ops-server $VER..."
if curl -fsSL -o "$TMP_PKG" -w "%{http_code}" "$URL" 2>/dev/null | grep -q 200; then
    echo "Downloaded prebuilt package."
else
    echo "No prebuilt package found for $ARCH."
    echo "Building from source..."
    TMP_DIR=$(mktemp -d)
    git clone "https://github.com/$REPO.git" "$TMP_DIR" 2>/dev/null || true
    cd "$TMP_DIR/packaging/arch" 2>/dev/null
    makepkg -si --noconfirm
    rm -rf "$TMP_DIR"
    echo "Done. Built and installed from source."
    exit 0
fi

echo "Installing..."
sudo pacman -U --noconfirm "$TMP_PKG" 2>/dev/null || {
    echo "Install failed. Try building from source:"
    echo "  git clone https://github.com/$REPO.git && cd arch-mcp/packaging/arch && makepkg -si"
    exit 1
}

rm -f "$TMP_PKG"
echo "Done. Server status:"
systemctl status arch-ops-server.service --no-pager 2>/dev/null || echo "Check server with: systemctl status arch-ops-server"
