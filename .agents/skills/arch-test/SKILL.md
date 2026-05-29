---
name: arch-test
description: Test a new release of arch-mcp MCP server. Use before merging or releasing — installs server on test machine, runs every tool, verifies systemd stability, compiles pass/fail report.
---

# Arch Test

Test a new release (or pre-release) of the arch-mcp MCP server end-to-end.

## When to use

- Before merging a feature branch into master
- Before tagging a new `v*.*.*` release
- After major changes to `server.py`, `report.py`, PKGBUILD, or packaging
- User says "протестируй релиз", "проверь новую версию", "run tests on real hardware"

## Prerequisites

- Test machine with Arch Linux (physical or VM — not CI mock)
- SSH access with sudo (password or key)
- The server source code available (local clone or GitHub)

## Test workflow

### Phase 1 — Install

1. SSH to the test machine
2. Clone the repo (or pull latest) — use the branch/commit under test
3. Install the server:
   - **Arch**: `cd packaging/arch && makepkg -si` (or `bash scripts/install.sh`)
   - **Debian**: `cd packaging/debian && sudo bash build-deb.sh`
4. Verify installation:
   ```bash
   which arch-ops-server-http
   systemctl status arch-ops-server
   ```
5. Fix PYTHONPATH if needed (see known issues below)

### Phase 2 — Tool-by-tool verification (28 tools)

Run ALL tools via HTTP API. Use parallel batches where safe. For each tool, record: PASS / FAIL / SKIP.

**Batch A — Monitoring (6 tools, parallel):**
```
get_system_info
analyze_storage action="disk_usage"
analyze_storage action="cache_stats"
diagnose_system action="failed_services"
manage_logs lines=5 boot=true
run_system_health_check
```

**Batch B — Package info (6 tools, parallel):**
```
check_updates_dry_run
manage_orphans action="list"
check_database_freshness
get_official_package_info package_name="pacman"
query_file_ownership query="/usr/bin/pacman" mode="file_to_package"
manage_groups action="list_groups"
```

**Batch C — Package lifecycle (3 tools, sequential — writes):**
```
manage_install_reason action="list"
query_package_history query_type="all" limit=5
verify_package_integrity package_name="pacman"
```

> **WARNING:** Skip `install_package_secure`, `remove_packages`, `manage_orphans action="remove"` on production machines. Only test write tools on disposable VMs.

**Batch D — Discovery (3 tools, parallel):**
```
search_archwiki query="pacman" limit=3
search_aur query="yay" limit=3 sort_by="relevance"
fetch_news action="critical" limit=3
```

**Batch E — BTRFS (6 tools, parallel):**
```
analyze_btrfs action="filesystem_info"
analyze_btrfs action="device_stats"
analyze_btrfs action="scrub_status"
analyze_btrfs action="subvolumes"
analyze_btrfs action="filesystem_usage"
manage_btrfs_scrub action="status"
```

> `analyze_btrfs action="snapshots"` / `manage_btrfs_snapshots action="list"` may need sudo or snapper config. Mark SKIP if not available.

**Batch F — Config + Mirrors (3 tools, parallel):**
```
analyze_pacman_conf focus="full"
analyze_makepkg_conf
optimize_mirrors action="health"
```

**Batch G — Boot (read-only):**
```
manage_boot action="status"
```

> **NEVER** test `set_boot_order` or `next_boot` with `reboot=true`. Status-only.

**Batch H — Report:**
```
generate_report action="full"
```

### Phase 3 — Systemd stability

```bash
# Check service uptime and restart count
systemctl status arch-ops-server

# Verify no recent crashes
journalctl -u arch-ops-server --since "5 minutes ago" | grep -i "error\|fail\|traceback" || echo "No errors"

# Check restart counter
systemctl show arch-ops-server -p NRestarts
```

Expected: `NRestarts=0` (no crashes since service start).

### Phase 4 — API endpoint check

```bash
# SSE endpoint
curl -s http://localhost:8080/sse -H "Accept: text/event-stream" --max-time 2 || true

# Direct POST (Smithery mode)
curl -s -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"tools/list","id":1}' | python3 -c "import sys,json; print(len(json.load(sys.stdin)['result']['tools']), 'tools')"
```

Expected: 28 tools listed.

### Phase 5 — Auth (if ARCH_OPS_SERVER_API_KEY set)

```bash
# Without token — should be rejected
curl -s -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"tools/list","id":1}'

# With token — should succeed
curl -s -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $ARCH_OPS_SERVER_API_KEY" \
  -d '{"jsonrpc":"2.0","method":"tools/list","id":1}'
```

## Report format

```markdown
# Arch MCP Test Report — v<version>

**Branch:** <branch> | **Commit:** <sha> | **Date:** <date>
**Machine:** <hostname> (<arch>) | **Kernel:** <version>

## Results

| # | Tool | Status | Note |
|---|------|--------|------|
| 1 | get_system_info | ✅ PASS | |
| 2 | analyze_storage | ✅ PASS | |
| ... | ... | ... | ... |
| 28 | manage_boot status | ✅ PASS | |

**Passed:** X/28 | **Failed:** Y/28 | **Skipped:** Z/28

## Systemd Stability
- NRestarts: <count>
- Errors in journal: <count>
- Uptime: <duration>

## Issues Found
1. <description> — <severity> — <fix>
2. ...

## generate_report output
<markdown table>

## Verdict
✅ READY FOR RELEASE / ❌ BLOCKED — <reason>
```

## Known issues

### ModuleNotFoundError after install
`uv pip install --target` overwrites PKGBUILD wrapper scripts. Fix:
```bash
printf '[Service]\nEnvironment=PYTHONPATH=/opt/arch-ops-server/vendor\n' | sudo tee /etc/systemd/system/arch-ops-server.service.d/override.conf
sudo systemctl daemon-reload && sudo systemctl restart arch-ops-server
```

### snapper snapshots — No permissions
`snapper list` requires root. Either run server as root or add user to snapper group.

### Port already in use
If port 8080 is occupied:
```bash
sudo ss -tlnp | grep 8080
sudo systemctl stop <conflicting-service>
sudo systemctl restart arch-ops-server
```

## CI integration

```yaml
# .github/workflows/test-release.yml
name: Test Release on Real Hardware
on:
  pull_request:
    branches: [master]
jobs:
  test-on-rpi:
    runs-on: self-hosted  # RPi5 runner
    steps:
      - uses: actions/checkout@v4
      - name: Install and test
        run: |
          cd packaging/arch && makepkg -si --noconfirm
          systemctl start arch-ops-server
          sleep 3
          # Run smoke test
          curl -s -X POST http://localhost:8080/mcp \
            -H "Content-Type: application/json" \
            -d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"get_system_info","arguments":{}},"id":1}'
      - name: Full test
        run: |
          # Call each tool via curl, verify non-error response
          ...
```
