"""
Report generation module — aggregates health-check tool results into a bare markdown table.

Standalone: importable from anywhere, no server dependency.
"""

import asyncio
import logging
from typing import Any, Dict, List, Tuple

from . import boot, btrfs, config, mirrors, pacman, system

logger = logging.getLogger(__name__)

Row = Tuple[str, str, str]

def _fmt_row(program: str, meaning: str, result: str) -> Row:
    return (program, meaning, result)


def _status_icon(ok: bool) -> str:
    return "\u2705" if ok else "\u274c"


def _parse_system_info(data: Dict[str, Any]) -> List[Row]:
    rows = []
    if "error_type" in data:
        rows.append(_fmt_row("uname", "System info", f"Error: {data.get('message', '')}"))
        return rows

    kernel = data.get("kernel", "?")
    arch = data.get("architecture", "?")
    hostname = data.get("hostname", "?")
    uptime = data.get("uptime", "?")
    mem = data.get("memory", {})
    total = mem.get("total", "?")
    used = mem.get("used", "?")

    rows.append(_fmt_row("uname", "Kernel", kernel))
    rows.append(_fmt_row("uname", "Architecture", arch))
    rows.append(_fmt_row("uname", "Hostname", hostname))
    rows.append(_fmt_row("uname", "Uptime", str(uptime)))
    rows.append(_fmt_row("free", "RAM", f"{used} / {total}"))
    return rows


def _parse_disk_usage(data: Dict[str, Any]) -> List[Row]:
    rows = []
    if "error_type" in data:
        rows.append(_fmt_row("df", "Disk usage", f"Error: {data.get('message', '')}"))
        return rows

    filesystems = data.get("filesystems", []) if isinstance(data, dict) else []
    for fs in filesystems:
        name = fs.get("filesystem", "?")
        used_pct = fs.get("used_percent", "?")
        avail = fs.get("available", "?")
        ok = isinstance(used_pct, (int, float)) and used_pct < 90
        icon = _status_icon(ok)
        rows.append(_fmt_row(
            "df", f"{name} ({avail} free)", f"{icon} {used_pct}% used"
        ))
    if not filesystems:
        rows.append(_fmt_row("df", "Disk usage", "No filesystems"))
    return rows


def _parse_cache_stats(data: Dict[str, Any]) -> List[Row]:
    rows = []
    if "error_type" in data:
        rows.append(_fmt_row("paccache", "Pacman cache", f"Error: {data.get('message', '')}"))
        return rows

    size = data.get("current_size", "?")
    packages = data.get("cached_packages", "?")
    rows.append(_fmt_row("paccache", "Pacman cache size", f"{size} ({packages} packages)"))
    return rows


def _parse_failed_services(data: Dict[str, Any]) -> List[Row]:
    rows = []
    if "error_type" in data:
        rows.append(_fmt_row("systemctl", "Failed services", f"Error: {data.get('message', '')}"))
        return rows

    count = data.get("failed_count", 0)
    ok = count == 0
    icon = _status_icon(ok)
    rows.append(_fmt_row("systemctl", "Failed services", f"{icon} {count}"))
    return rows


def _parse_updates(data: Dict[str, Any]) -> List[Row]:
    rows = []
    if "error_type" in data:
        rows.append(_fmt_row("pacman", "Updates", f"Error: {data.get('message', '')}"))
        return rows

    count = data.get("update_count", 0)
    ok = count == 0
    icon = _status_icon(ok)
    rows.append(_fmt_row("pacman", "Pending updates", f"{icon} {count} packages"))
    return rows


def _parse_orphans(data: Dict[str, Any]) -> List[Row]:
    rows = []
    if "error_type" in data:
        rows.append(_fmt_row("pacman", "Orphans", f"Error: {data.get('message', '')}"))
        return rows

    count = data.get("orphan_count", 0)
    ok = count == 0
    icon = _status_icon(ok)
    rows.append(_fmt_row("pacman", "Orphaned packages", f"{icon} {count}"))
    return rows


def _parse_db_freshness(data: Dict[str, Any]) -> List[Row]:
    rows = []
    if "error_type" in data:
        rows.append(_fmt_row("pacman", "DB freshness", f"Error: {data.get('message', '')}"))
        return rows

    databases = data.get("databases", {}) if isinstance(data, dict) else {}
    for db_name, info in databases.items():
        hours = info.get("hours_since_sync", "?")
        status = info.get("status", "?")
        ok = status == "fresh"
        icon = _status_icon(ok)
        rows.append(_fmt_row("pacman", f"DB {db_name}", f"{icon} {hours}h ({status})"))
    if not databases:
        rows.append(_fmt_row("pacman", "DB freshness", "No databases"))
    return rows


def _parse_mirror_health(data: Dict[str, Any]) -> List[Row]:
    rows = []
    if "error_type" in data:
        rows.append(_fmt_row("mirrors", "Mirror health", f"Error: {data.get('message', '')}"))
        return rows

    score = data.get("health_score", "?")
    issues = data.get("issues", [])
    ok = isinstance(score, (int, float)) and score >= 70
    icon = _status_icon(ok)
    issue_text = f" ({len(issues)} issues)" if issues else ""
    rows.append(_fmt_row("mirrors", "Health score", f"{icon} {score}/100{issue_text}"))
    return rows


def _parse_btrfs_info(data: Dict[str, Any]) -> List[Row]:
    rows = []
    if "error_type" in data:
        rows.append(_fmt_row("btrfs", "Filesystem", f"Skipped: {data.get('message', '')}"))
        return rows

    label = data.get("label", "?")
    devices = data.get("devices", [])
    dev_count = data.get("total_devices", len(devices))
    rows.append(_fmt_row("btrfs", "Label", label))
    rows.append(_fmt_row("btrfs", "Devices", str(dev_count)))
    return rows


def _parse_btrfs_device_stats(data: Dict[str, Any]) -> List[Row]:
    rows = []
    if "error_type" in data:
        rows.append(_fmt_row("btrfs", "Device errors", f"Skipped: {data.get('message', '')}"))
        return rows

    devices = data.get("devices", [])
    total_errors = 0
    for dev in devices:
        stats = dev.get("error_stats", {})
        for val in stats.values():
            if isinstance(val, (int, float)):
                total_errors += int(val)

    ok = total_errors == 0
    icon = _status_icon(ok)
    rows.append(_fmt_row("btrfs", "Device errors", f"{icon} {total_errors}"))
    return rows


def _parse_btrfs_scrub(data: Dict[str, Any]) -> List[Row]:
    rows = []
    if "error_type" in data:
        rows.append(_fmt_row("btrfs", "Scrub", f"Skipped: {data.get('message', '')}"))
        return rows

    scrub = data.get("scrub", {})
    status = scrub.get("status", "?")
    error_summary = scrub.get("error_summary", "?")
    ok = "no errors" in str(error_summary).lower()
    icon = _status_icon(ok)
    rows.append(_fmt_row("btrfs", f"Scrub ({status})", f"{icon} {error_summary}"))
    return rows


def _parse_boot_status(data: Dict[str, Any]) -> List[Row]:
    rows = []
    if "error_type" in data:
        rows.append(_fmt_row("boot", "Boot order", f"Skipped: {data.get('message', '')}"))
        return rows

    boot_order = data.get("boot_order", "?")
    sequence = data.get("sequence", [])
    decoded_labels = [item.get("label", item) if isinstance(item, dict) else item for item in sequence]
    devices_dict = data.get("available_devices", {})
    devices = [k for k, v in devices_dict.items() if v]
    rows.append(_fmt_row("boot", "BOOT_ORDER", boot_order))
    rows.append(_fmt_row("boot", "Boot sequence", " → ".join(decoded_labels)))
    rows.append(_fmt_row("boot", "Available devices", ", ".join(devices) if devices else "none"))
    return rows


def _build_table(rows: List[Row]) -> str:
    if not rows:
        return "| Программа | Что значит | Результат |\n|-----------|------------|------------|\n| — | No data | — |\n"

    lines = [
        "| Программа | Что значит | Результат |",
        "|-----------|------------|------------|",
    ]
    for prog, meaning, res in rows:
        lines.append(f"| {prog} | {meaning} | {res} |")
    return "\n".join(lines)


async def generate_report(action: str = "full") -> Dict[str, Any]:
    """
    Generate a bare markdown table health report by aggregating results from
    system monitoring, package status, BTRFS, mirrors, and boot tools.

    Args:
        action: Report scope — "full", "system", "packages", "storage",
                "btrfs", "mirrors", "config"

    Returns:
        Dict with "report" (markdown table string) and "sections" (list of dicts)
    """
    logger.info(f"Generating report: action={action}")

    sections_data = []  # type: List[Dict[str, Any]]

    try:
        # --- System section ---
        if action in ("full", "system"):
            tasks = [
                system.get_system_info(),
                system.diagnose_system(action="failed_services"),
            ]
            results = await asyncio.gather(*tasks, return_exceptions=True)

            sys_rows = []
            for i, r in enumerate(results):
                if isinstance(r, Exception):
                    continue
                data = r if isinstance(r, dict) else {}
                if i == 0:
                    sys_rows.extend(_parse_system_info(data))
                elif i == 1:
                    sys_rows.extend(_parse_failed_services(data))

            if sys_rows:
                sections_data.append({"name": "System", "rows": sys_rows})

        # --- Storage section ---
        if action in ("full", "storage"):
            tasks = [
                system.analyze_storage(action="disk_usage"),
                system.analyze_storage(action="cache_stats"),
            ]
            results = await asyncio.gather(*tasks, return_exceptions=True)

            stor_rows = []
            for i, r in enumerate(results):
                if isinstance(r, Exception):
                    continue
                data = r if isinstance(r, dict) else {}
                if i == 0:
                    stor_rows.extend(_parse_disk_usage(data))
                elif i == 1:
                    stor_rows.extend(_parse_cache_stats(data))

            if stor_rows:
                sections_data.append({"name": "Storage", "rows": stor_rows})

        # --- Packages section ---
        if action in ("full", "packages"):
            tasks = [
                pacman.check_updates_dry_run(),
                pacman.manage_orphans(action="list"),
                pacman.check_database_freshness(),
            ]
            results = await asyncio.gather(*tasks, return_exceptions=True)

            pkg_rows = []
            for i, r in enumerate(results):
                if isinstance(r, Exception):
                    continue
                data = r if isinstance(r, dict) else {}
                if i == 0:
                    pkg_rows.extend(_parse_updates(data))
                elif i == 1:
                    pkg_rows.extend(_parse_orphans(data))
                elif i == 2:
                    pkg_rows.extend(_parse_db_freshness(data))

            if pkg_rows:
                sections_data.append({"name": "Packages", "rows": pkg_rows})

        # --- Mirrors section ---
        if action in ("full", "mirrors"):
            result = await mirrors.optimize_mirrors(action="health")
            mirror_rows = _parse_mirror_health(result)
            if mirror_rows:
                sections_data.append({"name": "Mirrors", "rows": mirror_rows})

        # --- BTRFS section ---
        if action in ("full", "btrfs"):
            tasks = [
                btrfs.analyze_btrfs(action="filesystem_info"),
                btrfs.analyze_btrfs(action="device_stats"),
                btrfs.analyze_btrfs(action="scrub_status"),
            ]
            results = await asyncio.gather(*tasks, return_exceptions=True)

            btrfs_rows = []
            for i, r in enumerate(results):
                if isinstance(r, Exception):
                    continue
                data = r if isinstance(r, dict) else {}
                if i == 0:
                    btrfs_rows.extend(_parse_btrfs_info(data))
                elif i == 1:
                    btrfs_rows.extend(_parse_btrfs_device_stats(data))
                elif i == 2:
                    btrfs_rows.extend(_parse_btrfs_scrub(data))

            if btrfs_rows:
                sections_data.append({"name": "BTRFS", "rows": btrfs_rows})

        # --- Boot section ---
        if action in ("full", "system"):
            result = await boot.manage_boot(action="status")
            boot_rows = _parse_boot_status(result)
            if boot_rows:
                sections_data.append({"name": "Boot", "rows": boot_rows})

        # --- Config section ---
        if action in ("full", "config"):
            result = await config.analyze_pacman_conf(focus="full")
            # Simplified — just note if we have a response
            if "error_type" not in result:
                sections_data.append({
                    "name": "Config",
                    "rows": [
                        _fmt_row("pacman.conf", "Config available", _status_icon(True)),
                    ],
                })
            else:
                sections_data.append({
                    "name": "Config",
                    "rows": [
                        _fmt_row("pacman.conf", "Config", f"Skipped: {result.get('message', '')}"),
                    ],
                })

        # Build combined table
        all_rows = []
        for section in sections_data:
            # Section header
            all_rows.append(_fmt_row(f"**{section['name']}**", "", ""))
            all_rows.extend(section["rows"])

        report_md = _build_table(all_rows)

        return {
            "report": report_md,
            "sections": sections_data,
            "row_count": len(all_rows),
        }

    except Exception as e:
        logger.error(f"Failed to generate report: {e}")
        return {
            "report": f"| Программа | Что значит | Результат |\n|-----------|------------|------------|\n| Error | Report generation | {str(e)} |\n",
            "error": str(e),
        }
