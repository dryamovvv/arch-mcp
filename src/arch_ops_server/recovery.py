# SPDX-License-Identifier: GPL-3.0-only OR MIT
"""
System recovery module.
Emergency mode detection, system state, boot issue analysis, fstab repair.
"""

import logging
from typing import Dict, Any

from .utils import (
    IS_ARCH,
    run_command,
    create_error_response,
    check_command_exists,
)

logger = logging.getLogger(__name__)


async def manage_recovery(
    action: str,
    dry_run: bool = True,
) -> Dict[str, Any]:
    """
    Manage system recovery: emergency mode detection, state analysis, boot issues, fstab repair.

    Actions:
      check_emergency  — check if system is in emergency mode
      system_state     — systemctl is-system-running + failed units
      last_boot_issues — analyze previous boot errors
      repair_fstab     — verify UUIDs in fstab against blkid
    """
    if not IS_ARCH:
        return create_error_response(
            "NotSupported", "This feature is only available on Arch Linux."
        )

    if action == "check_emergency":
        logger.info("Checking for emergency mode")

        exit_code, out, err = await run_command(
            ["systemctl", "is-system-running"],
            timeout=5,
            check=False,
        )
        state = out.strip() if exit_code == 0 else "unknown"

        emergency = state in ("emergency", "rescue", "maintenance", "stopped")

        return {
            "emergency_mode": emergency,
            "system_state": state,
            "message": "System is in emergency mode!"
            if emergency
            else "System is in normal state",
        }

    elif action == "system_state":
        logger.info("Checking system state")

        exit_code, out, err = await run_command(
            ["systemctl", "is-system-running"],
            timeout=5,
            check=False,
        )
        running_state = out.strip() if exit_code == 0 else "unknown"

        exit_code, out, err = await run_command(
            ["systemctl", "list-units", "--state=failed", "--no-legend", "--no-pager"],
            timeout=5,
            check=False,
        )
        failed_units = []
        if exit_code == 0:
            for line in out.strip().splitlines():
                if line.strip():
                    parts = line.strip().split()
                    if parts:
                        failed_units.append(
                            {
                                "unit": parts[0],
                                "load": parts[1] if len(parts) > 1 else "",
                                "active": parts[2] if len(parts) > 2 else "",
                                "sub": parts[3] if len(parts) > 3 else "",
                            }
                        )

        exit_code, out, err = await run_command(
            ["uptime"],
            timeout=5,
            check=False,
        )
        uptime_str = out.strip() if exit_code == 0 else "unknown"

        return {
            "system_state": running_state,
            "uptime": uptime_str,
            "failed_count": len(failed_units),
            "failed_units": failed_units,
            "all_ok": running_state == "running" and len(failed_units) == 0,
        }

    elif action == "last_boot_issues":
        logger.info("Analyzing previous boot issues")

        exit_code, out, err = await run_command(
            ["journalctl", "-b", "-1", "-p", "err", "--no-pager", "-n", "50"],
            timeout=15,
            check=False,
        )
        prev_boot_errors = out.strip().splitlines() if exit_code == 0 else []

        exit_code, out, err = await run_command(
            ["journalctl", "-b", "-1", "-p", "warn", "--no-pager", "-n", "30"],
            timeout=15,
            check=False,
        )
        prev_boot_warnings = out.strip().splitlines() if exit_code == 0 else []

        exit_code, out, err = await run_command(
            ["journalctl", "-b", "0", "--no-pager", "--list-boots"],
            timeout=10,
            check=False,
        )
        boot_list = out.strip().splitlines() if exit_code == 0 else []

        issues = 0
        critical_keywords = [
            "error",
            "failed",
            "timed out",
            "kernel panic",
            "oom",
            "segfault",
        ]
        for line in prev_boot_errors + prev_boot_warnings:
            if any(kw in line.lower() for kw in critical_keywords):
                issues += 1

        return {
            "prev_boot_errors_count": len(prev_boot_errors),
            "prev_boot_warnings_count": len(prev_boot_warnings),
            "critical_issues": issues,
            "prev_boot_errors": prev_boot_errors[-20:]
            if len(prev_boot_errors) > 20
            else prev_boot_errors,
            "prev_boot_warnings": prev_boot_warnings[-15:]
            if len(prev_boot_warnings) > 15
            else prev_boot_warnings,
            "boot_history": boot_list,
        }

    elif action == "repair_fstab":
        logger.info("Checking fstab UUID consistency")

        import os

        fstab_path = "/etc/fstab"
        if not os.path.exists(fstab_path):
            return create_error_response("NotFound", "fstab not found")

        exit_code, blk_out, err = await run_command(
            ["blkid", "-o", "export"],
            timeout=10,
            check=False,
        )
        blkid_map = {}
        if exit_code == 0:
            current_uuid = None
            for line in blk_out.strip().splitlines():
                if line.startswith("UUID="):
                    current_uuid = line.split("=")[1].strip()
                elif line.startswith("TYPE=") and current_uuid:
                    blkid_map[current_uuid] = line.split("=")[1].strip().strip('"')

        issues = []
        with open(fstab_path) as f:
            for lineno, line in enumerate(f, 1):
                stripped = line.strip()
                if not stripped or stripped.startswith("#"):
                    continue
                parts = stripped.split()
                if len(parts) < 2:
                    continue
                fs_spec = parts[0]
                if fs_spec.startswith("UUID="):
                    uuid = fs_spec[5:]
                    uuid_clean = uuid.strip('"').strip("'")
                    if uuid_clean and uuid_clean not in blkid_map:
                        issues.append(
                            {
                                "line": lineno,
                                "uuid": uuid_clean,
                                "mount": parts[1] if len(parts) > 1 else "?",
                                "issue": "UUID not found on any device",
                            }
                        )

        ok = len(issues) == 0
        result = {
            "fstab_path": fstab_path,
            "ok": ok,
            "issue_count": len(issues),
            "issues": issues if not ok else [],
            "message": "All fstab UUIDs match block devices"
            if ok
            else f"Found {len(issues)} UUID mismatch(es)",
            "dry_run": dry_run,
        }

        if not dry_run and not ok:
            result["action"] = (
                "Automatic repair not available. Remove stale entries manually."
            )

        return result

    else:
        return create_error_response(
            "InvalidAction",
            f"Unknown action: '{action}'. Valid actions: check_emergency, system_state, last_boot_issues, repair_fstab.",
        )
