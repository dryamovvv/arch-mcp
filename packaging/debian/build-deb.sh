#!/bin/bash
set -euo pipefail

PKG="arch-ops-server"
VER="${1:-3.4.2}"
BUILD_DIR="/tmp/${PKG}_${VER}_build"
DEB_FILE="${PKG}_${VER}_amd64.deb"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "=== Building $PKG $VER ==="

# --- Prepare target directory ---
rm -rf "$BUILD_DIR"
mkdir -p "$BUILD_DIR/opt/$PKG/vendor"
mkdir -p "$BUILD_DIR/usr/bin"
mkdir -p "$BUILD_DIR/etc/systemd/system"
mkdir -p "$BUILD_DIR/DEBIAN"

# --- Install Python deps ---
echo "Installing Python dependencies..."
python3 -m pip install \
    --target "$BUILD_DIR/opt/$PKG/vendor" \
    --no-compile \
    "$REPO_DIR"

# --- Wrapper scripts ---
cat > "$BUILD_DIR/opt/$PKG/vendor/bin/arch-ops-server" << 'WRAPPER'
#!/bin/sh
exec env PYTHONPATH=/opt/arch-ops-server/vendor \
    /usr/bin/python3 -c 'from arch_ops_server import main; import asyncio; asyncio.run(main())'
WRAPPER
chmod 755 "$BUILD_DIR/opt/$PKG/vendor/bin/arch-ops-server"

cat > "$BUILD_DIR/opt/$PKG/vendor/bin/arch-ops-server-http" << 'WRAPPER'
#!/bin/sh
exec env PYTHONPATH=/opt/arch-ops-server/vendor \
    /usr/bin/python3 -c 'from arch_ops_server import main_http; import asyncio; asyncio.run(main_http())'
WRAPPER
chmod 755 "$BUILD_DIR/opt/$PKG/vendor/bin/arch-ops-server-http"

ln -s /opt/$PKG/vendor/bin/arch-ops-server "$BUILD_DIR/usr/bin/arch-ops-server"
ln -s /opt/$PKG/vendor/bin/arch-ops-server-http "$BUILD_DIR/usr/bin/arch-ops-server-http"

# --- Systemd service ---
install -Dm644 "$SCRIPT_DIR/arch-ops-server.service" \
    "$BUILD_DIR/etc/systemd/system/arch-ops-server.service"

# --- DEBIAN control ---
cp "$SCRIPT_DIR/control" "$BUILD_DIR/DEBIAN/"
cp "$SCRIPT_DIR/postinst" "$BUILD_DIR/DEBIAN/"
cp "$SCRIPT_DIR/prerm" "$BUILD_DIR/DEBIAN/"
sed -i "s/^Version:.*/Version: $VER/" "$BUILD_DIR/DEBIAN/control"

# --- Package ---
mkdir -p "$REPO_DIR/dist"
dpkg-deb --build "$BUILD_DIR" "$REPO_DIR/dist/$DEB_FILE"
echo "=== Built: dist/$DEB_FILE ==="
ls -lh "$REPO_DIR/dist/$DEB_FILE"
