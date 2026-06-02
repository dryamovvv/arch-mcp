# Arch Linux MCP Server — `arch-opsd`

**Disclaimer:** Unofficial community project, not affiliated with Arch Linux.

A [Model Context Protocol](https://modelcontextprotocol.io/) (MCP) server that bridges AI assistants with the Arch Linux ecosystem. Single static Rust binary — zero runtime dependencies.

## Key Features

- **Single binary (~4.2 MB)** — download and run, no Python/Node/ interpreter needed
- **Cross-platform** — builds for `aarch64` and `x86_64` (musl static)
- **48 MCP tools**, 2 prompts, 2 resources (and growing)
- **STDIO transport** (default) + **HTTP/SSE transport** (optional feature)

## Tools

### Fully Implemented (18 tools)

| Tool                        | Description                                     | Platform |
| --------------------------- | ----------------------------------------------- | -------- |
| `search_archwiki`           | Search Arch Wiki via MediaWiki API              | Any      |
| `search_aur`                | Search AUR RPC v5 (sort by relevance/votes/popularity/updated) | Any |
| `audit_package_security`    | PKGBUILD safety analysis (50+ red flags) + metadata risk scoring | Any |
| `install_package_secure`    | 5-step secure AUR install with security checks  | Arch     |
| `get_official_package_info` | Package info via `pacman -Si` or archlinux.org API | Any    |
| `check_updates_dry_run`     | Check available updates via `checkupdates`      | Arch     |
| `remove_packages`           | Remove packages with dep/force options          | Arch     |
| `manage_orphans`            | List/remove orphan packages (dry-run by default)| Arch     |
| `manage_install_reason`     | List/mark explicit/dependency install reason    | Arch     |
| `verify_package_integrity`  | Package file verification (-Qk / -Qkk)          | Arch     |
| `check_database_freshness`  | Pacman DB sync staleness check                  | Arch     |
| `query_file_ownership`      | 3 modes: file→package, package→files, filename search | Arch |
| `manage_groups`             | List groups or packages in a group              | Arch     |
| `get_system_info`           | Kernel, uptime, memory from /proc               | Any      |
| `analyze_storage`           | Disk usage and pacman cache stats               | Any/Arch |
| `diagnose_system`           | Failed services and boot logs                   | systemd  |
| `fetch_news`                | Arch RSS feed (latest/critical/since-update)    | Any      |
| `optimize_mirrors`          | Mirror status, speed test, suggestions, health  | Any      |
| `analyze_pacman_conf`       | pacman.conf analysis (full/ignored/parallel)    | Arch     |
| `analyze_makepkg_conf`      | makepkg.conf CFLAGS, MAKEFLAGS extraction       | Arch     |

### Stubs (30 tools — return "not yet implemented")

`run_system_health_check`, `query_package_history`, `manage_logs`, `manage_journal_gateway`, `analyze_btrfs`, `manage_btrfs_snapshots`, `manage_btrfs_scrub`, `manage_boot`, `generate_report`, `verify_boot_artifacts`, `verify_service_health`, `verify_homectl_user`, `compare_fstab`, `compare_packages`, `check_security_posture`, `check_rpi_hardware`, `benchmark_quick`, `manage_luks`, `manage_firewall`, `manage_hardware`, `manage_boot_config`, `manage_backup`, `manage_recovery`, `manage_telegram_unlock`

## Installation

### Arch Linux (binary from GitHub Releases)

```bash
curl -fsSL https://raw.githubusercontent.com/dryamovvv/arch-mcp/master/scripts/install.sh | bash
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

# HTTP mode
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

## Prompts

| Prompt               | Status     |
| -------------------- | ---------- |
| `troubleshoot_issue` | ✅ Partial |
| `safe_system_update` | ✅ Partial |
| `audit_aur_package`  | ❌ Stub    |
| `analyze_dependencies`| ❌ Stub   |

## Resources

| URI Scheme      | Example                      | Status      |
| --------------- | ---------------------------- | ----------- |
| `archwiki://`   | `archwiki://Installation_guide` | ✅ Basic |
| `aur://`        | —                            | ❌ Stub     |
| `archrepo://`   | —                            | ❌ Stub     |
| `pacman://`     | —                            | ❌ Stub     |
| `system://`     | —                            | ❌ Stub     |
| `archnews://`   | —                            | ❌ Stub     |
| `mirrors://`    | —                            | ❌ Stub     |
| `config://`     | —                            | ❌ Stub     |

## License

Dual-licensed: GPL-3.0-only OR MIT.
