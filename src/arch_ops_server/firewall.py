# SPDX-License-Identifier: GPL-3.0-only OR MIT
"""
nftables firewall management module.
Manages nftables rules: listing, port management, validation, reload.
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

NFTABLES_CONF = "/etc/nftables.conf"


async def _nft(args: list[str]) -> tuple[int, str, str]:
    return await run_command(["nft"] + args, timeout=10, check=False)


async def manage_firewall(
    action: str,
    port: Optional[int] = None,
    proto: str = "tcp",
    interface: Optional[str] = None,
    chain: Optional[str] = None,
) -> Dict[str, Any]:
    """
    Manage nftables firewall: list rules, add/remove ports, validate config, reload.

    Actions:
      list_rules  — nft list ruleset or specific chain
      add_port    — add a temporary port to INPUT chain
      remove_port — remove a port from INPUT chain
      validate    — check nftables.conf syntax
      reload      — reload nftables ruleset
    """
    if not IS_ARCH:
        return create_error_response(
            "NotSupported", "This feature is only available on Arch Linux."
        )

    if not check_command_exists("nft"):
        return create_error_response(
            "NotSupported",
            "nftables not installed. Install with: sudo pacman -S nftables",
        )

    if action == "list_rules":
        logger.info("Listing nftables ruleset")
        cmd = ["nft"]
        if chain:
            cmd.extend(["list", "chain", "inet", "filter", chain])
        else:
            cmd.append("list ruleset")

        exit_code, out, err = await run_command(cmd, timeout=10, check=False)
        if exit_code != 0:
            return create_error_response("CommandError", f"nft failed: {err}")

        return {
            "ruleset": out.strip(),
            "rule_count": len(
                [
                    l
                    for l in out.splitlines()
                    if l.strip() and not l.strip().startswith("#")
                ]
            ),
        }

    elif action == "add_port":
        if not port:
            return create_error_response(
                "MissingArgument", "port is required for add_port"
            )
        proto = proto.lower()
        if proto not in ("tcp", "udp"):
            return create_error_response(
                "InvalidValue", f"Invalid protocol: {proto}. Use tcp or udp."
            )

        logger.info(f"Adding port {port}/{proto} to nftables INPUT")
        rule = f"tcp dport {port}" if proto == "tcp" else f"udp dport {port}"
        if interface:
            rule = f"iif {interface} {rule}"

        exit_code, out, err = await _nft(
            [
                "add",
                "rule",
                "inet",
                "filter",
                "INPUT",
                rule,
                "accept",
            ]
        )
        if exit_code != 0:
            return create_error_response("CommandError", f"Failed to add port: {err}")

        return {
            "ok": True,
            "port": port,
            "proto": proto,
            "interface": interface,
            "message": f"Port {port}/{proto} added to INPUT chain (temporary — write to {NFTABLES_CONF} to persist)",
        }

    elif action == "remove_port":
        if not port:
            return create_error_response(
                "MissingArgument", "port is required for remove_port"
            )
        proto = proto.lower()

        logger.info(f"Removing port {port}/{proto} from nftables INPUT")

        # nft delete rule by handle — find handle first
        if proto == "tcp":
            grep_pat = f"dport {port}"
        else:
            grep_pat = f"udp dport {port}"

        exit_code, out, err = await _nft(
            ["-a", "list", "chain", "inet", "filter", "INPUT"]
        )
        if exit_code != 0:
            return create_error_response("CommandError", f"Failed to list rules: {err}")

        handle = None
        for line in out.splitlines():
            if grep_pat in line and "accept" in line:
                parts = line.strip().split()
                if "handle" in parts:
                    idx = parts.index("handle")
                    if idx + 1 < len(parts):
                        handle = parts[idx + 1]
                        break

        if not handle:
            return create_error_response(
                "NotFound",
                f"No matching rule found for port {port}/{proto} in INPUT chain",
            )

        exit_code, out, err = await _nft(
            [
                "delete",
                "rule",
                "inet",
                "filter",
                "INPUT",
                "handle",
                handle,
            ]
        )
        if exit_code != 0:
            return create_error_response(
                "CommandError", f"Failed to delete rule: {err}"
            )

        return {
            "ok": True,
            "port": port,
            "proto": proto,
            "message": f"Port {port}/{proto} removed from INPUT chain",
        }

    elif action == "validate":
        logger.info("Validating nftables configuration")
        exit_code, out, err = await run_command(
            ["nft", "-c", "-f", NFTABLES_CONF],
            timeout=5,
            check=False,
        )
        valid = exit_code == 0
        return {
            "valid": valid,
            "config_file": NFTABLES_CONF,
            "error": err.strip() if not valid else None,
            "message": "Configuration is valid"
            if valid
            else f"Validation failed: {err.strip()}",
        }

    elif action == "reload":
        logger.info("Reloading nftables")
        exit_code, out, err = await run_command(
            ["systemctl", "reload", "nftables"],
            timeout=10,
            check=False,
        )
        if exit_code != 0:
            return create_error_response(
                "CommandError", f"Failed to reload nftables: {err}"
            )

        return {"ok": True, "message": "nftables reloaded successfully"}

    else:
        return create_error_response(
            "InvalidAction",
            f"Unknown action: '{action}'. Valid actions: list_rules, add_port, remove_port, validate, reload.",
        )
