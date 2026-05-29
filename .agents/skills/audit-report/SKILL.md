---
name: audit-report
description: Generate structured health audit reports for Arch Linux systems. Use when the user wants a system diagnostics report, CI audit output, or health summary. Supports both bare table and free-form reports.
---

# Audit Report

Generate structured health audit reports using the arch-linux MCP server.

## When to use

- User asks "дай отчет по системе", "health check", "что с системой"
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

Returns `| Программа | Что значит | Результат |` markdown table.

### 2. Free-form report (conversational)

Run tools in parallel batches, then build a narrative:

**Batch 1 (parallel):**
```
get_system_info
analyze_storage(action='disk_usage')
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
analyze_storage(action='cache_stats')
analyze_pacman_conf
analyze_makepkg_conf
```

## Report format guidelines

Present as structured markdown with sections:

1. **System Overview** (kernel, uptime, RAM, disk)
2. **Health** (failed services, updates, orphans)
3. **BTRFS** (filesystem, subvolumes, snapshots, scrub, device errors)  
4. **Database Freshness** (per repo)
5. **Mirrors** (health score, active count)
6. **Boot Config** (BOOT_ORDER, available devices)
7. **Config** (pacman + makepkg highlights)
8. **Summary of Issues** (table: issue, severity, action)

## CI integration

```yaml
# .github/workflows/audit.yml
# Use the HTTP SSE endpoint to get a bare table report
curl -X POST http://TARGET:8080/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"generate_report","arguments":{"action":"full"}},"id":1}'
```

Save output as a GitHub artifact or publish to Pages.

## After presenting the report

Ask if the user wants to fix identified issues. Prioritize:
1. Failed services (urgent)
2. Stale databases (pacman -Sy)
3. Orphans (remove or mark as explicit)
4. Mirror health (enable better mirrors)
5. BTRFS scrub (start if never run)
