#!/bin/bash
set -euo pipefail

REPO="dryamovvv/arch-mcp"
ARCH=$(uname -m)
TMP_PKG="/tmp/arch-ops-server.pkg.tar.zst"

echo "arch-ops-server installer for Arch Linux"
echo "Architecture: $ARCH"

LATEST_TAG=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" | python3 -c "import sys,json; print(json.load(sys.stdin)['tag_name'])")
VER="${LATEST_TAG#v}"
BASE="https://github.com/$REPO/releases/download/$LATEST_TAG/arch-ops-server-${VER}-1-any"

echo "Trying prebuilt package arch-ops-server $VER ($ARCH)..."

for ext in zst xz gz; do
    URL="$BASE-$ARCH.pkg.tar.$ext"
    if curl -fsSL -o "$TMP_PKG" "$URL"; then
        echo "Downloaded prebuilt ($ARCH, .$ext). Installing..."
        sudo pacman -U --noconfirm "$TMP_PKG"
        rm -f "$TMP_PKG"
        echo "Server status:"
        systemctl status arch-ops-server.service --no-pager 2>/dev/null || echo "Check: systemctl status arch-ops-server"
        exit 0
    fi
done

# Build from source (all architectures fall through here if prebuilt unavailable)
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

echo "Server status:"
systemctl status arch-ops-server.service --no-pager 2>/dev/null || echo "Check server with: systemctl status arch-ops-server"
