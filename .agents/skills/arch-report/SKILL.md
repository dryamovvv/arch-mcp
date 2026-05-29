---
name: arch-report
description: Generate structured health audit reports for Arch Linux systems. Use when the user wants a system diagnostics report, CI audit output, or health summary. Supports both bare table and free-form reports.
---

# Audit Report

Generate structured health audit reports using the arch-linux MCP server.

## Relationship with `arch-system`

- **`arch-report`** (this skill) — focused on generating *finished reports* (bare table or narrative). Use `generate_report` tool as primary mechanism.
- **`arch-system`** — focused on *interactive system management* (diagnostics, fixing issues, package operations). Use for troubleshooting sessions.

For a quick CI-friendly audit, use `generate_report`. For deep interactive diagnostics, load `arch-system`.

## When to use

- User asks "report on the system", "health check", "what's wrong with the system", "дай отчет"
- CI pipeline needs a structured report
- Periodic system audits

## Report types

| Type | Tool | Format |
|------|------|--------|
| **Bare table** | `generate_report(action='full')` | Markdown table `ПРОГРАММА \| ЧТО ЗНАЧИТ \| РЕЗУЛЬТАТ` |
| **Free form** | Run tools individually, build narrative | Sections with issues highlighted |

## Workflow

### 1. Bare table report (CI-friendly)

```
generate_report(action='full')
```

Scopes: `full`, `system`, `packages`, `storage`, `btrfs`, `mirrors`, `config`.

- `full` — all sections: System, Storage, Packages, Mirrors, BTRFS, Boot, Config
- `system` — system info (kernel, RAM, hostname, uptime) + failed services + boot config
- `packages` — pending updates, orphans, database freshness
- `storage` — disk usage + pacman cache stats
- `btrfs` — filesystem info, device errors, scrub status
- `mirrors` — mirror health score
- `config` — pacman.conf availability

Returns `{"report": "<markdown table>", "sections": [...], "row_count": N}`.

### 2. Free-form report (conversational)

For detailed narrative reports, run tools in parallel batches, then build a report:

**Batch 1 (parallel):**
```
get_system_info
analyze_storage(action='disk_usage')
analyze_storage(action='cache_stats')
diagnose_system(action='failed_services')
check_updates_dry_run
manage_orphans(action='list')
check_database_freshness
optimize_mirrors(action='health')
analyze_btrfs(action='filesystem_info')
analyze_btrfs(action='device_stats')
analyze_btrfs(action='scrub_status')
manage_boot(action='status')
```

**Batch 2 (if needed):**
```
diagnose_system(action='boot_logs', lines=50)
analyze_pacman_conf
analyze_makepkg_conf
fetch_news(action='critical', limit=5)
analyze_btrfs(action='snapshots')
analyze_btrfs(action='snapper_configs')
```

## Report format guidelines

Present as structured markdown with sections:

1. **System Overview** (kernel, arch, hostname, uptime, RAM, disk)
2. **Health** (failed services, pending updates, orphans, DB freshness)
3. **BTRFS** (filesystem, subvolumes, snapshots, scrub, device errors)
4. **Database Freshness** (per repo)
5. **Mirrors** (health score, issues count)
6. **Boot Config** (BOOT_ORDER, boot sequence, available devices)
7. **Config** (pacman.conf + makepkg.conf highlights)
8. **Summary of Issues** (table: issue, severity, action)

If any tool returns an error or timeout, note it and continue.

## CI integration

```yaml
# .github/workflows/audit.yml
# Use the HTTP SSE endpoint to get a bare table report
curl -X POST http://TARGET:8080/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"generate_report","arguments":{"action":"full"}},"id":1}'
```

Save output as a GitHub artifact or publish to GitLab/Pages.

## After presenting the report

Ask if the user wants to fix identified issues. Prioritize:
1. Failed services (urgent — use `diagnose_system(action='boot_logs')` to investigate)
2. Stale databases (`pacman -Sy` via the server)
3. Orphans (remove or mark as explicit via `manage_orphans`)
4. Zero-score mirrors (enable better mirrors via `optimize_mirrors(action='suggest')`)
5. BTRFS scrub (start if never run via `manage_btrfs_scrub(action='start')`)
