#!/bin/bash
set -euo pipefail

REPO="dryamovvv/arch-mcp"
DEST="/usr/local/bin/arch-opsd"

case $(uname -m) in
aarch64) ARCH="aarch64" ;;
x86_64) ARCH="x86_64" ;;
armv7l) ARCH="aarch64" ;; # RPi 32-bit kernel, 64-bit userspace
*)
	echo "Unsupported: $(uname -m)"
	exit 1
	;;
esac

URL="https://github.com/$REPO/releases/latest/download/arch-opsd-linux-$ARCH"
echo "Downloading arch-opsd ($ARCH)..."
curl -fsSL -o /tmp/arch-opsd "$URL"
chmod +x /tmp/arch-opsd
sudo mv /tmp/arch-opsd "$DEST"

echo "Installed: $DEST"
"$DEST" stdio --help 2>&1 | head -2
