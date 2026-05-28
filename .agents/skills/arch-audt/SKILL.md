---
name: arch-audt
description: Use arch-linux MCP server to manage your remote Raspberry Pi 5 running Arch Linux ARM. The server provides 26 tools for system monitoring, package management, AUR, configs, BTRFS, mirrors, and health checks.
---

## When to use

- User asks about RPi5 status, packages, or troubleshooting
- Package installation/removal on the remote Arch system
- System diagnostics or health checks
- Config analysis (pacman.conf, makepkg.conf)
- BTRFS filesystem monitoring, snapshots, or scrub
- Mirror optimization
- Arch Wiki lookups

## Usage pattern

Use `arch-linux_TOOL_NAME` to call tools. All remote operations are safe (read-only by default, writes require explicit flag). Run independent tools in parallel batches.

## Tool catalog (26 tools)

### System (read-only)

| Tool | Description |
|---|---|
| `get_system_info` | Kernel, arch, hostname, uptime, RAM |
| `analyze_storage action="disk_usage"` | Disk space for critical paths |
| `analyze_storage action="cache_stats"` | Pacman package cache stats |
| `diagnose_system action="failed_services"` | Check for failed systemd units |
| `diagnose_system action="boot_logs"` | Recent journalctl boot logs |
| `run_system_health_check` | Comprehensive multi-tool health check |

### Packages (read-only)

| Tool | Description |
|---|---|
| `get_official_package_info` | Official repo package details |
| `search_aur` | Search AUR (relevance/votes/popularity/modified) |
| `check_updates_dry_run` | Pending system updates |
| `check_database_freshness` | Pacman DB sync age per repository |
| `search_archwiki` | Arch Wiki search with ranked results |
| `fetch_news action="latest|critical|since_update"` | Arch news feed |

### Packages (write)

| Tool | Description |
|---|---|
| `install_package_secure` | Install with AUR security audit |
| `remove_packages` | Remove with deps or force |
| `manage_orphans action="list|remove"` | Find/remove orphaned packages |
| `manage_install_reason action="list|mark_explicit|mark_dependency"` | Toggle install reason |
| `verify_package_integrity` | Check installed files against checksums |

### Organization & History

| Tool | Description |
|---|---|
| `query_file_ownership mode="file_to_package|package_to_files|filename_search"` | File-package mapping |
| `manage_groups action="list_groups|list_packages_in_group"` | Package group management |
| `query_package_history query_type="all|package|failures|sync"` | Pacman log history |

### Config

| Tool | Description |
|---|---|
| `analyze_pacman_conf focus="full|ignored_packages|parallel_downloads"` | pacman.conf analysis |
| `analyze_makepkg_conf` | makepkg.conf (CFLAGS, MAKEFLAGS, etc.) |

### Mirrors

| Tool | Description |
|---|---|
| `optimize_mirrors action="status|test|suggest|health"` | Mirror management |

### Security

| Tool | Description |
|---|---|
| `audit_package_security action="pkgbuild_analysis|metadata_risk"` | AUR PKGBUILD & trust scoring |

### BTRFS (new)

| Tool | Description |
|---|---|
| `analyze_btrfs` | 11 actions: filesystem_info, filesystem_df, filesystem_usage, subvolumes, subvolume_info, device_stats, device_usage, properties, scrub_status, snapshots, snapper_configs |
| `manage_btrfs_snapshots` | 4 actions: list, configs, create, delete (via snapper) |
| `manage_btrfs_scrub` | 3 actions: status, start, cancel |

## Report format

Present results as a structured markdown report with sections:

1. System Overview (kernel, uptime, RAM, disk)
2. Health (failed services, updates, orphans)
3. BTRFS (filesystem, subvolumes, snapshots, scrub, device errors)
4. Databases (freshness per repo)
5. Mirrors (health score, active count)
6. Config (pacman + makepkg highlights)
7. News (critical items count + relevant titles)
8. Summary of issues (table: issue, severity, action)

If any tool returns an error or timeout, note it in the report and continue.

## Audit plan (parallel batches)

### Batch 1 — all independent, run in parallel

```
get_system_info
run_system_health_check
analyze_storage action="disk_usage"
analyze_storage action="cache_stats"
diagnose_system action="failed_services"
check_updates_dry_run
check_database_freshness
manage_orphans action="list"
analyze_pacman_conf
analyze_makepkg_conf
fetch_news action="critical" limit=5
optimize_mirrors action="health"
analyze_btrfs action="filesystem_info"
analyze_btrfs action="filesystem_usage"
analyze_btrfs action="device_stats"
analyze_btrfs action="scrub_status"
analyze_btrfs action="snapshots"
analyze_btrfs action="snapper_configs"
```

### Batch 2 — after batch 1, if needed

```
diagnose_system action="boot_logs" lines=50
get_official_package_info package_name="linux-rpi-16k"
analyze_btrfs action="subvolumes"
analyze_btrfs action="subvolume_info"
```

## After presenting the report

Ask if the user wants to:
- Fix identified issues (sync DBs, enable mirrors, remove orphans, create/delete snapshots, start scrub)
- Run deeper diagnostics on specific areas
