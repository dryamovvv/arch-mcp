# SPDX-License-Identifier: GPL-3.0-only OR MIT
"""
btrbk backup management module.
Manages btrbk-based BTRFS backups: run, list, status, restore info.
"""

import logging
from typing import Dict, Any, Optional

from .utils import (
    IS_ARCH,
    run_command,
    create_error_response,
    check_command_exists,
)

logger = logging.getLogger(__name__)


async def manage_backup(
    action: str,
    config: Optional[str] = None,
    dry_run: bool = False,
    snapshot_id: Optional[str] = None,
) -> Dict[str, Any]:
    """
    Manage btrbk backups: run, list, status, restore info.

    Actions:
      run          — execute btrbk (optionally with dry_run)
      list         — btrbk list — history of backups
      status       — check btrbk availability and config
      restore_info — show snapshot restore information
    """
    if not IS_ARCH:
        return create_error_response(
            "NotSupported", "This feature is only available on Arch Linux."
        )

    if not check_command_exists("btrbk"):
        return create_error_response(
            "NotSupported", "btrbk not installed. Install with: sudo pacman -S btrbk"
        )

    if action == "run":
        logger.info(f"Running btrbk (dry_run={dry_run})")
        cmd = ["btrbk", "run"]
        if config:
            cmd.extend(["-c", config])
        if dry_run:
            cmd.append("--dry-run")

        exit_code, out, err = await run_command(cmd, timeout=300, check=False)
        if exit_code != 0:
            return create_error_response("CommandError", f"btrbk run failed: {err}")

        return {
            "ok": True,
            "dry_run": dry_run,
            "output": out.strip(),
            "message": "btrbk completed successfully"
            if not dry_run
            else "btrbk dry run completed",
        }

    elif action == "list":
        logger.info("Listing btrbk backup history")
        cmd = ["btrbk", "list"]
        if config:
            cmd.extend(["-c", config])

        exit_code, out, err = await run_command(cmd, timeout=30, check=False)
        if exit_code != 0:
            return create_error_response("CommandError", f"btrbk list failed: {err}")

        lines = [l for l in out.strip().splitlines() if l.strip()]
        return {
            "backup_count": len(lines),
            "backups": lines,
        }

    elif action == "status":
        logger.info("Checking btrbk status")

        result = {"btrbk_installed": True, "configs": []}

        import os

        config_dirs = ["/etc/btrbk", "/etc/btrbk.conf"]
        for path in config_dirs:
            if os.path.isfile(path):
                result["configs"].append({"path": path, "type": "file"})
            elif os.path.isdir(path):
                for f in os.listdir(path):
                    if f.endswith(".conf"):
                        result["configs"].append(
                            {"path": os.path.join(path, f), "type": "file"}
                        )

        if not result["configs"]:
            result["warning"] = (
                "No btrbk config files found. Create /etc/btrbk.conf or /etc/btrbk/*.conf"
            )

        return result

    elif action == "restore_info":
        if not snapshot_id:
            return create_error_response(
                "MissingArgument", "snapshot_id is required for restore_info"
            )
        logger.info(f"Checking restore info for snapshot {snapshot_id}")

        exit_code, out, err = await run_command(
            ["btrbk", "list"],
            timeout=30,
            check=False,
        )
        if exit_code != 0:
            return create_error_response("CommandError", f"btrbk list failed: {err}")

        snapshot_lines = []
        for line in out.strip().splitlines():
            if snapshot_id in line:
                snapshot_lines.append(line.strip())

        return {
            "snapshot_id": snapshot_id,
            "matches": len(snapshot_lines),
            "snapshots": snapshot_lines,
            "restore_hint": (
                "To restore: mount the snapshot subvolume, then copy files back. "
                "Example: mount -t btrfs -o subvol=<snapshot_path> /dev/nvme0n1p2 /mnt/restore"
            ),
        }

    else:
        return create_error_response(
            "InvalidAction",
            f"Unknown action: '{action}'. Valid actions: run, list, status, restore_info.",
        )
