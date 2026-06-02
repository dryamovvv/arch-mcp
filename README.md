# Arch Linux MCP Server — `arch-opsd`

[![Docs](https://img.shields.io/badge/docs-mkdocs-0094f5?logo=materialformkdocs)](https://dryamovvv.github.io/arch-mcp/)

**Disclaimer:** Unofficial community project, not affiliated with Arch Linux.

A [Model Context Protocol](https://modelcontextprotocol.io/) (MCP) server that bridges AI assistants with the Arch Linux ecosystem. Single static Rust binary — zero runtime dependencies.

## Key Features

- **Single binary (~4.3 MB)** — download and run, no Python/Node/interpreter needed
- **Cross-platform** — builds for `aarch64` and `x86_64` (musl static)
- **48 MCP tools**, 8 prompts, 24 resources
- **STDIO transport** (default) + **HTTP/SSE transport** (optional feature)

## Tools (48)

### Wiki & AUR
| Tool | Description | Platform |
| ---- | ----------- | -------- |
| `search_archwiki` | Search Arch Wiki via MediaWiki API | Any |
| `search_aur` | Search AUR RPC v5 (sort by relevance/votes/popularity/updated) | Any |
| `audit_package_security` | PKGBUILD safety analysis (50+ red flags) + metadata risk scoring | Any |
| `install_package_secure` | 5-step secure AUR install with security checks | Arch |

### Package Management
| Tool | Description | Platform |
| ---- | ----------- | -------- |
| `get_official_package_info` | Package info via `pacman -Si` or archlinux.org API | Any |
| `check_updates_dry_run` | Check available updates via `checkupdates` | Arch |
| `remove_packages` | Remove packages with dep/force options | Arch |
| `manage_orphans` | List/remove orphan packages (dry-run by default) | Arch |
| `manage_install_reason` | List/mark explicit/dependency install reason | Arch |
| `verify_package_integrity` | Package file verification (-Qk / -Qkk) | Arch |
| `check_database_freshness` | Pacman DB sync staleness check | Arch |
| `query_file_ownership` | 3 modes: file→package, package→files, filename search | Arch |
| `manage_groups` | List groups or packages in a group | Arch |
| `query_package_history` | Query pacman transaction log (all/package/failures/sync) | Arch |

### System
| Tool | Description | Platform |
| ---- | ----------- | -------- |
| `get_system_info` | Kernel, uptime, memory from /proc | Any |
| `analyze_storage` | Disk usage and pacman cache stats | Any/Arch |
| `diagnose_system` | Failed services and boot logs | systemd |
| `run_system_health_check` | Comprehensive multi-subsystem health check | Arch |
| `manage_logs` | journalctl operations (size/rotate/vacuum/verify/flush) | systemd |
| `manage_journal_gateway` | systemd-journal-gatewayd service management | systemd |
| `fetch_news` | Arch RSS feed (latest/critical/since-update) | Any |

### Configuration & Mirrors
| Tool | Description | Platform |
| ---- | ----------- | -------- |
| `optimize_mirrors` | Mirror status, speed test, suggestions, health | Any |
| `analyze_pacman_conf` | pacman.conf analysis (full/ignored/parallel) | Arch |
| `analyze_makepkg_conf` | makepkg.conf CFLAGS, MAKEFLAGS extraction | Arch |

### Storage & Boot
| Tool | Description | Platform |
| ---- | ----------- | -------- |
| `analyze_btrfs` | Analyze BTRFS filesystem usage | Arch |
| `manage_btrfs_snapshots` | List/create/delete BTRFS snapshots | Arch |
| `manage_btrfs_scrub` | Run BTRFS scrub operations | Arch |
| `manage_boot` | Manage systemd-boot entries and kernels | Arch |
| `manage_luks` | LUKS encrypted volume management | Arch |
| `manage_boot_config` | Manage bootloader configuration | Arch |

### Networking & Security
| Tool | Description | Platform |
| ---- | ----------- | -------- |
| `manage_firewall` | Firewall rule management (iptables/nftables) | Arch |
| `manage_hardware` | Hardware diagnostics (CPU, disks, PCI, USB) | Any |
| `manage_backup` | Filesystem backup operations (rsync wrapper) | Any |
| `manage_recovery` | System recovery mode operations | Arch |
| `manage_telegram_unlock` | Telegram-based remote unlock for encrypted systems | Any |
| `generate_report` | Generate structured health audit reports | Any |

### Build Test (RPi5 OS validation)
| Tool | Description | Platform |
| ---- | ----------- | -------- |
| `verify_boot_artifacts` | Check boot partition integrity (kernel, DTBs, config) | Arch (RPi) |
| `verify_service_health` | Check systemd service health | systemd |
| `verify_homectl_user` | Check homectl user configuration | Arch |
| `compare_fstab` | Compare current fstab against golden reference | Arch |
| `compare_packages` | Compare installed packages against manifest | Arch |
| `check_security_posture` | Security audit (unattended-upgrades, SSH, firewall, fail2ban) | Arch |
| `check_rpi_hardware` | RPi5 hardware diagnostics (temp, freq, voltage, eeprom) | RPi |
| `benchmark_quick` | Quick system performance benchmark | Any |

## Resources (24)

| URI Scheme | Example | Resources |
| ---------- | ------- | --------- |
| `archwiki://` | `archwiki://Installation_guide` | Wiki page as HTML |
| `aur://` | `aur://yay/info`, `aur://yay/pkgbuild` | AUR package info, PKGBUILD |
| `archrepo://` | `archrepo://vim` | Official repo package info |
| `pacman://` | `pacman://installed`, `pacman://orphans`, `pacman://explicit`, `pacman://groups`, `pacman://group/base-devel`, `pacman://log/recent`, `pacman://log/failed`, `pacman://database/freshness` | Local system package state |
| `system://` | `system://info`, `system://disk`, `system://services/failed`, `system://logs/boot`, `system://health` | System diagnostics |
| `archnews://` | `archnews://latest`, `archnews://critical`, `archnews://since-update` | Arch Linux news |
| `mirrors://` | `mirrors://active`, `mirrors://health` | Mirror configuration |
| `config://` | `config://pacman`, `config://makepkg` | Parsed config files |

## Prompts (8)

| Prompt | Description |
| ------ | ----------- |
| `troubleshoot_issue` | Diagnose errors using Arch Wiki knowledge |
| `audit_aur_package` | Security audit of AUR packages before installation |
| `analyze_dependencies` | Package dependency analysis and install planning |
| `safe_system_update` | Pre-update checks (news, disk, services, DB, updates) |
| `cleanup_system` | Orphan removal, cache cleaning, integrity checks |
| `package_investigation` | Deep research before package install |
| `mirror_optimization` | Speed test and optimal mirror configuration |
| `system_health_check` | Comprehensive multi-subsystem diagnostic |

## Installation

### Arch Linux (binary from GitHub Releases)

```bash
curl -fsSL https://raw.githubusercontent.com/dryamovvv/arch-mcp/rust/scripts/install.sh | sudo bash
```

### Build from source

```bash
git clone https://github.com/dryamovvv/arch-mcp.git
cd arch-mcp
cargo build --release
# Binary: target/release/arch-opsd
```

### Cross-compile for RPi5 (aarch64)

```bash
cargo install cross
cross build --release --target aarch64-unknown-linux-musl
# Binary: target/aarch64-unknown-linux-musl/release/arch-opsd
```

## Usage

```bash
# STDIO mode (default for MCP clients)
arch-opsd stdio

# HTTP mode (with SSE-based MCP transport)
arch-opsd-http        # listens on :8080
```

### Claude / Cursor / MCP clients

```json
{
  "mcpServers": {
    "arch-linux": {
      "command": "arch-opsd",
      "args": ["stdio"]
    }
  }
}
```

### SSH to remote machine

```json
{
  "mcpServers": {
    "rpi5": {
      "command": "ssh",
      "args": ["rpi5.local", "arch-opsd", "stdio"]
    }
  }
}
```

## License

Dual-licensed: GPL-3.0-only OR MIT.
