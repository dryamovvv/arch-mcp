#!/bin/bash
set -euo pipefail

REPO="dryamovvv/arch-mcp"
TMP_DEB="/tmp/arch-ops-server.deb"

echo "arch-ops-server installer for Debian/Ubuntu"

LATEST_TAG=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" | python3 -c "import sys,json; print(json.load(sys.stdin)['tag_name'])")
VER="${LATEST_TAG#v}"
DEB="arch-ops-server_${VER}_amd64.deb"
URL="https://github.com/$REPO/releases/download/$LATEST_TAG/$DEB"

echo "Downloading arch-ops-server $VER..."

if curl -fsSL -o "$TMP_DEB" -w "%{http_code}" "$URL" 2>/dev/null | grep -q 200; then
    echo "Downloaded prebuilt package."
    sudo apt install -y "$TMP_DEB"
else
    echo "No prebuilt package found for this architecture."
    echo "Building from source..."
    TMP_DIR=$(mktemp -d)
    git clone "https://github.com/$REPO.git" "$TMP_DIR" 2>/dev/null || true
    cd "$TMP_DIR/packaging/debian"
    sudo bash build-deb.sh
    sudo apt install -y "./dist/arch-ops-server_*.deb"
    rm -rf "$TMP_DIR"
fi

rm -f "$TMP_DEB"
echo "Done. Server status:"
systemctl status arch-ops-server.service --no-pager 2>/dev/null || echo "Check with: systemctl status arch-ops-server"
