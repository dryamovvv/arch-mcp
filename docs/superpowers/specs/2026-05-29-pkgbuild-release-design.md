# arch-mcp PKGBUILD + Release Pipeline

**Date:** 2026-05-29
**Status:** approved

## Goal

Package arch-mcp as a self-contained Arch Linux `.pkg.tar.xz` with no external Python dependencies, built on GitHub Actions and published to GitHub Releases. Single-command install (`curl | bash` or `pacman -U`). Server is running immediately after install.

## Architecture

```
GitHub tag v*.*.*
      │
      ▼
┌─────────────────────────┐
│  CI: release-pkg.yml    │
│                         │
│  1. uv build → .whl     │
│  2. uv pip install      │
│     → /opt/arch-ops-    │
│       server/           │
│  3. makepkg → .pkg.tar  │
│  4. Upload to Release   │
└──────────┬──────────────┘
           │
           ▼
   GitHub Releases
   assets:
    arch-ops-server-3.4.1-1-aarch64.pkg.tar.xz
           │
           ▼
   User: curl | bash (or pacman -U)
           │
           ▼
┌─────────────────────────┐
│  /opt/arch-ops-server/  │
│  ├── .venv/             │  ← bundled Python + deps
│  ├── bin/               │  ← symlink wrappers
│  │   arch-ops-server    │
│  │   arch-ops-server-   │
│  │   http               │
│  └── src/               │  ← package sources
│                         │
│  /usr/bin/              │
│    arch-ops-server   ─────────> /opt/arch-ops-server/bin/
│    arch-ops-server-http ──────> /opt/arch-ops-server/bin/
│                         │
│  /etc/systemd/system/   │
│    arch-ops-server.service
└─────────────────────────┘
```

## Components

### 1. PKGBUILD (`packaging/arch/PKGBUILD`)

- **pkgname:** arch-ops-server
- **pkgver:** from pyproject.toml (auto-extracted)
- **arch:** any
- **depends:** python>=3.11 (system python only, all other deps bundled)
- **optdepends:** btrfs-progs, snapper, rpi-eeprom (for optional tools)
- **install:** `arch-ops-server.install` — post_install enables + starts systemd service, pre_remove stops + disables

**build():**
1. `uv pip install --target "$pkgdir/opt/arch-ops-server/vendor" .` — install all Python deps into vendor dir
2. Copy source into `$pkgdir/opt/arch-ops-server/src/`

**package():**
1. Create wrapper script at `/usr/bin/arch-ops-server`:
   ```sh
   #!/bin/sh
   exec /usr/bin/env PYTHONPATH=/opt/arch-ops-server/vendor \
        /usr/bin/python -m arch_ops_server "$@"
   ```
2. Create `arch-ops-server-http` similarly
3. Install systemd service file
4. Copy source tree to `/opt/arch-ops-server/src/`

### 2. Systemd service (`packaging/arch/arch-ops-server.service`)

```ini
[Unit]
Description=Arch Linux MCP Server (HTTP)
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=/usr/bin/arch-ops-server-http
Restart=on-failure
RestartSec=5
Environment=PYTHONUNBUFFERED=1

[Install]
WantedBy=multi-user.target
```

### 3. Install hook (`packaging/arch/arch-ops-server.install`)

```
post_install() {
    systemctl daemon-reload
    systemctl enable --now arch-ops-server.service
}
pre_remove() {
    systemctl stop arch-ops-server.service
    systemctl disable arch-ops-server.service
}
post_upgrade() {
    systemctl daemon-reload
    systemctl restart arch-ops-server.service
}
```

### 4. CI: `.github/workflows/release-pkg.yml`

Trigger: `push tags v*.*.*`

Jobs:
- Install `pacman`, `makepkg` on Ubuntu runner (build Arch package in CI)
- Install `uv`
- Extract version from tag
- Run `makepkg` with dynamic pkgver
- Upload `.pkg.tar.xz` to GitHub Release

**Alternative (simpler):** Build on Arch Linux Docker container in CI.

### 5. Install script (`scripts/install.sh`)

```bash
#!/bin/bash
# One-liner installer
set -euo pipefail

ARCH=$(uname -m)
LATEST_TAG=$(curl -s https://api.github.com/repos/dryamovvv/arch-mcp/releases/latest | jq -r .tag_name)
URL="https://github.com/dryamovvv/arch-mcp/releases/download/${LATEST_TAG}/arch-ops-server-${LATEST_TAG#v}-1-${ARCH}.pkg.tar.xz"

curl -fsSL "$URL" -o /tmp/arch-ops-server.pkg.tar.xz
sudo pacman -U --noconfirm /tmp/arch-ops-server.pkg.tar.xz
rm /tmp/arch-ops-server.pkg.tar.xz
```

User runs:
```bash
curl -fsSL https://raw.githubusercontent.com/dryamovvv/arch-mcp/master/scripts/install.sh | bash
```

### 6. GitHub Release assets

Each release contains:
- `arch-ops-server-3.4.1-1-aarch64.pkg.tar.xz` (RPi5 / ARM64)
- `arch-ops-server-3.4.1-1-x86_64.pkg.tar.xz` (x86_64)
- `arch-ops-server-3.4.1-1-any.pkg.tar.xz` (universal)

## File layout in repo

```
arch-mcp/
├── packaging/
│   └── arch/
│       ├── PKGBUILD
│       ├── arch-ops-server.service
│       └── arch-ops-server.install
├── scripts/
│   └── install.sh
├── .github/
│   └── workflows/
│       ├── publish-pypi.yml     (existing)
│       ├── publish-ghcr.yml     (existing)
│       └── release-pkg.yml      (new)
└── ...
```

## Verification

1. Push tag → CI builds `.pkg.tar.xz`
2. On clean RPi5: `pacman -U arch-ops-server-*.pkg.tar.xz`
3. `systemctl status arch-ops-server` → active
4. `curl http://localhost:8080/tools/list` → returns tools JSON
