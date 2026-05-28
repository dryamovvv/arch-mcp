# SPDX-License-Identifier: GPL-3.0-only OR MIT
"""
System journal log management module.
Provides filtering and retrieval of systemd journal logs.
"""

import logging
from typing import Dict, Any, Optional

from .utils import (
    IS_ARCH,
    run_command,
    create_error_response,
    check_command_exists
)

logger = logging.getLogger(__name__)

PRIORITY_MAP = {
    "emerg": "0",
    "alert": "1",
    "crit": "2",
    "err": "3",
    "warning": "4",
    "notice": "5",
    "info": "6",
    "debug": "7",
}


async def manage_logs(
    unit: Optional[str] = None,
    priority: Optional[str] = None,
    lines: int = 50,
    since: Optional[str] = None,
    until: Optional[str] = None,
    grep: Optional[str] = None,
    boot: bool = True,
) -> Dict[str, Any]:
    """
    Retrieve and filter systemd journal logs.

    Args:
        unit: Filter by systemd unit name (e.g. "sshd", "systemd-networkd")
        priority: Log level filter (emerg, alert, crit, err, warning, notice, info, debug)
        lines: Number of log lines to return (default: 50)
        since: Start time (e.g. "5 minutes ago", "2 hours ago", "yesterday")
        until: End time (e.g. "now", "10 minutes ago")
        grep: Keyword filter to search within log messages
        boot: If True, show only current boot logs (default: True)

    Returns:
        Dict with log lines and metadata
    """
    if not check_command_exists("journalctl"):
        return create_error_response(
            "NotSupported",
            "journalctl not available (systemd-based system required)"
        )

    cmd = ["journalctl", "--no-pager", "-o", "short-iso"]

    if boot:
        cmd.extend(["-b"])

    if unit:
        cmd.extend(["-u", unit])

    if priority:
        prio_num = PRIORITY_MAP.get(priority.lower())
        if prio_num is None:
            return create_error_response(
                "InvalidPriority",
                f"Unknown priority: {priority}. "
                f"Use one of: {', '.join(PRIORITY_MAP.keys())}"
            )
        cmd.extend(["-p", prio_num])

    if since:
        cmd.extend(["--since", since])

    if until:
        cmd.extend(["--until", until])

    cmd.extend(["-n", str(lines)])

    logger.info(f"Running journalctl: {' '.join(cmd)}")

    try:
        exit_code, stdout, stderr = await run_command(
            cmd, timeout=15, check=False
        )

        if exit_code != 0:
            return create_error_response(
                "CommandError",
                f"journalctl failed: {stderr}"
            )

        log_lines = stdout.strip().split('\n')

        if grep:
            log_lines = [l for l in log_lines if grep.lower() in l.lower()]

        result = {
            "line_count": len(log_lines),
            "unit": unit,
            "priority": priority,
            "boot_only": boot,
            "since": since,
            "until": until,
            "grep": grep,
        }

        if log_lines:
            result["logs"] = log_lines
        else:
            result["logs"] = []
            result["message"] = "No matching log entries found"

        return result

    except Exception as e:
        logger.error(f"Failed to retrieve logs: {e}")
        return create_error_response(
            "LogRetrievalError",
            f"Failed to retrieve logs: {str(e)}"
        )
