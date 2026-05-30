#!/usr/bin/env bash
# Smoke test for arch-mcp server — runs all tools one by one via HTTP
# Usage: ./test_mcp.sh [host]   (default: 192.168.1.54:8080)
set -euo pipefail

HOST="${1:-192.168.1.54:8080}"
MCP="http://$HOST/mcp"
PASS=0
FAIL=0
SKIP=0

call() {
	local method="$1" params="$2" label="$3"
	local json
	json=$(curl -s --max-time 15 "$MCP" \
		-H "Content-Type: application/json" \
		-d "{\"jsonrpc\":\"2.0\",\"method\":\"$method\",\"params\":$params,\"id\":1}" 2>/dev/null) || true
	if echo "$json" | grep -q '"error"'; then
		local err
		err=$(echo "$json" | grep -o '"message":"[^"]*"' | head -1 | cut -d'"' -f4)
		if echo "$err" | grep -qiE "only available|not installed|not running|not Arch|not supported"; then
			echo "  SKIP  $label → $err"
			SKIP=$((SKIP + 1))
		else
			echo "  FAIL  $label → $err"
			FAIL=$((FAIL + 1))
		fi
	elif echo "$json" | grep -q '"result"'; then
		echo "  PASS  $label"
		PASS=$((PASS + 1))
	else
		echo "  FAIL  $label → no response or malformed: $(echo "$json" | head -c 120)"
		FAIL=$((FAIL + 1))
	fi
}

tool() {
	local name="$1" args="${2:-{}}"
	call "tools/call" "{\"name\":\"$name\",\"arguments\":$args}" "$name($(echo "$args" | tr -d '\n'))"
}

echo "=== arch-mcp smoke test ($HOST) ==="
echo ""

echo "── System ──"
tool "get_system_info"
tool "analyze_storage" '{"action":"disk_usage"}'
tool "analyze_storage" '{"action":"cache_stats"}'
tool "diagnose_system" '{"action":"failed_services"}'
tool "run_system_health_check" '{"action":"full"}'

echo "── Packages ──"
tool "check_updates_dry_run"
tool "check_database_freshness"
tool "manage_orphans" '{"action":"list"}'
tool "query_package_history" '{"query_type":"all","limit":5}'
tool "manage_groups" '{"action":"list_groups"}'

echo "── Config ──"
tool "analyze_pacman_conf"
tool "analyze_makepkg_conf"

echo "── Mirrors ──"
tool "optimize_mirrors" '{"action":"health"}'
tool "optimize_mirrors" '{"action":"status"}'

echo "── News ──"
tool "fetch_news" '{"action":"critical","limit":3}'

echo "── Boot ──"
tool "manage_boot" '{"action":"status"}'
tool "manage_boot_config" '{"action":"read_cmdline"}'
tool "manage_boot_config" '{"action":"verify_boot"}'
tool "manage_boot_config" '{"action":"check_initramfs_hooks"}'
tool "manage_boot_config" '{"action":"check_boot_order"}'

echo "── BTRFS ──"
tool "analyze_btrfs" '{"action":"filesystem_info"}'
tool "analyze_btrfs" '{"action":"filesystem_usage"}'
tool "analyze_btrfs" '{"action":"device_stats"}'
tool "analyze_btrfs" '{"action":"scrub_status"}'
tool "analyze_btrfs" '{"action":"snapshots"}'
tool "analyze_btrfs" '{"action":"snapper_configs"}'
tool "manage_btrfs_snapshots" '{"action":"list"}'
tool "manage_btrfs_snapshots" '{"action":"configs"}'
tool "manage_btrfs_scrub" '{"action":"status"}'

echo "── Security ──"
tool "manage_luks" '{"action":"is_unlocked"}'
tool "manage_luks" '{"action":"status"}'
tool "manage_firewall" '{"action":"list_rules"}'
tool "manage_firewall" '{"action":"validate"}'
tool "check_security_posture" '{"action":"sshd"}'

echo "── Hardware ──"
tool "manage_hardware" '{"action":"health"}'
tool "manage_hardware" '{"action":"eeprom_info"}'
tool "manage_hardware" '{"action":"nvme_info"}'
tool "check_rpi_hardware" '{"action":"temperature"}'

echo "── Backup ──"
tool "manage_backup" '{"action":"status"}'
tool "manage_backup" '{"action":"list"}'

echo "── Recovery ──"
tool "manage_recovery" '{"action":"system_state"}'
tool "manage_recovery" '{"action":"check_emergency"}'

echo "── Journal ──"
tool "manage_logs" '{"lines":3}'
tool "manage_journal_gateway" '{"action":"status"}'

echo "── Reports ──"
tool "generate_report" '{"action":"full"}'

echo "── Telegram ──"
tool "manage_telegram_unlock" '{"action":"status"}'

echo ""
echo "=== Results: $PASS pass, $FAIL fail, $SKIP skip ==="
[ "$FAIL" -eq 0 ] && exit 0 || exit 1
