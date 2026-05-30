# SPDX-License-Identifier: GPL-3.0-only OR MIT
"""
BTRFS filesystem monitoring and management module.
Provides filesystem info, subvolume listing, device stats, snapshots, and scrub.
"""

import logging
from pathlib import Path
from typing import Dict, Any, Optional

from .utils import (
    IS_ARCH,
    run_command,
    create_error_response,
    check_command_exists,
)

logger = logging.getLogger(__name__)


async def _check_btrfs(path: str = "/") -> Optional[Dict[str, Any]]:
    """Verify path is on a BTRFS filesystem. Returns error dict or None."""
    if not check_command_exists("btrfs"):
        return create_error_response(
            "NotSupported",
            "btrfs-progs is not installed. Install with: sudo pacman -S btrfs-progs",
        )

    exit_code, stdout, _ = await run_command(
        ["stat", "-f", "--format=%T", path], timeout=5, check=False
    )
    if exit_code != 0 or stdout.strip() != "btrfs":
        return create_error_response(
            "NotBtrfs",
            f"Path '{path}' is not on a BTRFS filesystem",
            details=f"Filesystem type: {stdout.strip() if exit_code == 0 else 'unknown'}",
        )

    return None


async def _run_btrfs(cmd: list[str], timeout: int = 15) -> Dict[str, Any]:
    """Run a btrfs command and return parsed result or error."""
    exit_code, stdout, stderr = await run_command(cmd, timeout=timeout, check=False)

    if exit_code != 0:
        return create_error_response(
            "CommandError",
            f"btrfs command failed: {' '.join(cmd)}",
            details=stderr.strip(),
        )

    return {"stdout": stdout, "stderr": stderr}


# ============================================================================
# Filesystem Information
# ============================================================================


async def get_filesystem_info(path: str = "/") -> Dict[str, Any]:
    logger.info(f"Getting BTRFS filesystem info for {path}")

    err = await _check_btrfs(path)
    if err:
        return err

    exit_code, stdout, stderr = await run_command(
        ["btrfs", "filesystem", "show", path], timeout=10, check=False
    )

    if exit_code != 0:
        return create_error_response(
            "CommandError", f"btrfs filesystem show failed: {stderr.strip()}"
        )

    devices = []
    label = None
    uuid = None

    for line in stdout.strip().split("\n"):
        line = line.strip()
        if not line:
            continue

        if line.startswith("Label:"):
            parts = line.split()
            if len(parts) >= 2:
                label = parts[1].strip("'\"")
            for i, p in enumerate(parts):
                if p.lower() == "uuid:" and i + 1 < len(parts):
                    uuid = parts[i + 1]
                    break

        elif line.startswith("devid"):
            parts = line.split()
            dev = {}
            for i, p in enumerate(parts):
                if p == "devid" and i + 1 < len(parts):
                    dev["devid"] = parts[i + 1]
                elif p == "size" and i + 1 < len(parts):
                    dev["size"] = parts[i + 1]
                elif p == "used" and i + 1 < len(parts):
                    dev["used"] = parts[i + 1]
                elif p == "path" and i + 1 < len(parts):
                    dev["path"] = parts[i + 1]
            if dev:
                devices.append(dev)

    result = {
        "label": label,
        "uuid": uuid,
        "total_devices": len(devices),
        "devices": devices,
    }

    logger.info(f"BTRFS filesystem info: label={label}, uuid={uuid}")
    return result


async def get_filesystem_df(path: str = "/") -> Dict[str, Any]:
    logger.info(f"Getting BTRFS df for {path}")

    err = await _check_btrfs(path)
    if err:
        return err

    exit_code, stdout, stderr = await run_command(
        ["btrfs", "filesystem", "df", path], timeout=10, check=False
    )

    if exit_code != 0:
        return create_error_response(
            "CommandError", f"btrfs filesystem df failed: {stderr.strip()}"
        )

    profiles = {}
    for line in stdout.strip().split("\n"):
        line = line.strip()
        if not line:
            continue

        parts = line.split()
        if len(parts) < 3:
            continue

        raw_name = " ".join(parts[:2])
        profile_name = raw_name.rstrip(":,")
        total_str = (
            parts[2].split("=")[-1] if len(parts) > 2 and "=" in parts[2] else ""
        )
        used_str = parts[4].split("=")[-1] if len(parts) > 4 and "=" in parts[4] else ""

        profiles[profile_name] = {
            "total": total_str,
            "used": used_str,
        }

    logger.info(f"BTRFS df: {len(profiles)} profiles")
    return {"profiles": profiles}


async def get_filesystem_usage(path: str = "/") -> Dict[str, Any]:
    logger.info(f"Getting BTRFS usage for {path}")

    err = await _check_btrfs(path)
    if err:
        return err

    exit_code, stdout, stderr = await run_command(
        ["btrfs", "filesystem", "usage", path], timeout=10, check=False
    )

    if exit_code != 0:
        return create_error_response(
            "CommandError", f"btrfs filesystem usage failed: {stderr.strip()}"
        )

    result = {
        "overall": {},
        "data": {},
        "metadata": {},
        "system": {},
        "unallocated": {},
    }

    current_section = "overall"
    for line in stdout.strip().split("\n"):
        line = line.strip()
        if not line:
            continue

        if line.startswith("Overall:"):
            current_section = "overall"
            continue
        elif line.startswith("Data,"):
            current_section = "data"
            continue
        elif line.startswith("Metadata,"):
            current_section = "metadata"
            continue
        elif line.startswith("System,"):
            current_section = "system"
            continue
        elif line.startswith("Unallocated:"):
            current_section = "unallocated"
            continue
        elif line.startswith("GlobalReserve") or line.startswith("Multiple profiles"):
            continue

        parts = line.split()
        if current_section == "overall" and len(parts) >= 2:
            key = parts[0].rstrip(":")
            val = " ".join(parts[1:])
            result["overall"][key] = val

        elif current_section == "unallocated" and parts:
            if parts[0].startswith("/dev/"):
                result["unallocated"][parts[0]] = parts[-1] if len(parts) > 1 else ""

    logger.info(f"BTRFS usage: overall={len(result['overall'])} keys")
    return result


# ============================================================================
# Subvolumes
# ============================================================================


async def list_subvolumes(path: str = "/") -> Dict[str, Any]:
    logger.info(f"Listing BTRFS subvolumes for {path}")

    err = await _check_btrfs(path)
    if err:
        return err

    exit_code, stdout, stderr = await run_command(
        ["btrfs", "subvolume", "list", "-t", path], timeout=10, check=False
    )

    if exit_code != 0:
        return create_error_response(
            "CommandError", f"btrfs subvolume list failed: {stderr.strip()}"
        )

    subvolumes = []
    lines = stdout.strip().split("\n")

    found_header = False
    for line in lines:
        line_stripped = line.strip()
        if not line_stripped:
            continue

        if line_stripped.startswith("ID") and "gen" in line_stripped.lower():
            found_header = True
            continue

        if line_stripped.startswith("-") and not found_header:
            continue

        if not found_header:
            continue

        parts = line_stripped.split()
        if len(parts) >= 4:
            try:
                subvol = {
                    "id": int(parts[0]),
                    "gen": int(parts[1]),
                    "top_level": int(parts[2]),
                    "path": parts[3],
                }
                subvolumes.append(subvol)
            except (ValueError, IndexError):
                continue

    logger.info(f"BTRFS subvolumes: {len(subvolumes)} found")
    return {"subvolume_count": len(subvolumes), "subvolumes": subvolumes}


async def get_subvolume_info(path: str = "/") -> Dict[str, Any]:
    logger.info(f"Getting BTRFS subvolume info for {path}")

    err = await _check_btrfs(path)
    if err:
        return err

    exit_code, stdout, stderr = await run_command(
        ["btrfs", "subvolume", "show", path], timeout=10, check=False
    )

    if exit_code != 0:
        return create_error_response(
            "CommandError", f"btrfs subvolume show failed: {stderr.strip()}"
        )

    info = {}
    snapshots = []
    in_snapshots = False

    for line in stdout.strip().split("\n"):
        line = line.strip()
        if not line:
            continue

        if line.startswith("Snapshot(s):"):
            in_snapshots = True
            continue

        if in_snapshots:
            snap_line = line.strip()
            if snap_line and not snap_line.startswith("Quota"):
                snapshots.append(snap_line)
            elif snap_line.startswith("Quota"):
                in_snapshots = False
                key, _, val = snap_line.partition(":")
                info[key.strip().lower().replace(" ", "_")] = val.strip()
            continue

        if ":" in line:
            key, _, val = line.partition(":")
            info[key.strip().lower().replace(" ", "_")] = val.strip()

    result = {"info": info}
    if snapshots:
        result["snapshot_count"] = len(snapshots)
        result["snapshots"] = snapshots

    logger.info(f"BTRFS subvolume info: name={info.get('name', 'unknown')}")
    return result


# ============================================================================
# Device Statistics
# ============================================================================


async def get_device_stats(path: str = "/") -> Dict[str, Any]:
    logger.info(f"Getting BTRFS device stats for {path}")

    err = await _check_btrfs(path)
    if err:
        return err

    exit_code, stdout, stderr = await run_command(
        ["btrfs", "device", "stats", path], timeout=10, check=False
    )

    if exit_code != 0:
        return create_error_response(
            "CommandError", f"btrfs device stats failed: {stderr.strip()}"
        )

    devices = {}
    for line in stdout.strip().split("\n"):
        line = line.strip()
        if not line:
            continue

        if line.startswith("[") and "]" in line:
            dev_close = line.index("]")
            device_path = line[1:dev_close]
            rest = line[dev_close + 1 :].lstrip(".")
            parts = rest.split()
            if parts:
                err_name = parts[0]
                err_val = parts[1] if len(parts) > 1 else "0"

                if device_path not in devices:
                    devices[device_path] = {}

                devices[device_path][err_name] = int(err_val)

    has_errors = any(any(v != 0 for v in dev.values()) for dev in devices.values())

    logger.info(f"BTRFS device stats: {len(devices)} devices, has_errors={has_errors}")
    return {
        "devices": devices,
        "has_errors": has_errors,
    }


async def get_device_usage(path: str = "/") -> Dict[str, Any]:
    logger.info(f"Getting BTRFS device usage for {path}")

    err = await _check_btrfs(path)
    if err:
        return err

    exit_code, stdout, stderr = await run_command(
        ["btrfs", "device", "usage", path], timeout=10, check=False
    )

    if exit_code != 0:
        return create_error_response(
            "CommandError", f"btrfs device usage failed: {stderr.strip()}"
        )

    devices = {}
    current_device = None

    for line in stdout.strip().split("\n"):
        line = line.strip()
        if not line:
            continue

        if line.startswith("/dev/") and "," in line:
            dev_name = line.split(",")[0]
            current_device = dev_name
            if current_device not in devices:
                devices[current_device] = {}
            parts = line.split()
            if len(parts) >= 4:
                devices[current_device]["device_size"] = parts[3]
        elif current_device and ":" in line:
            parts = line.split()
            if len(parts) >= 2:
                key = parts[0].rstrip(":")
                val = parts[1]
                devices[current_device][key] = val
        elif current_device and line.startswith("Unallocated:"):
            parts = line.split()
            if len(parts) >= 2:
                devices[current_device]["unallocated"] = parts[1]

    logger.info(f"BTRFS device usage: {len(devices)} devices")
    return {"devices": devices}


# ============================================================================
# Properties
# ============================================================================


async def get_properties(path: str = "/") -> Dict[str, Any]:
    logger.info(f"Getting BTRFS properties for {path}")

    err = await _check_btrfs(path)
    if err:
        return err

    exit_code, stdout, stderr = await run_command(
        ["btrfs", "property", "get", path], timeout=10, check=False
    )

    if exit_code != 0:
        return create_error_response(
            "CommandError", f"btrfs property get failed: {stderr.strip()}"
        )

    properties = {}
    for line in stdout.strip().split("\n"):
        line = line.strip()
        if "=" in line:
            key, _, val = line.partition("=")
            properties[key.strip()] = val.strip()

    logger.info(f"BTRFS properties: {len(properties)} found")
    return {"properties": properties}


# ============================================================================
# Scrub
# ============================================================================


async def get_scrub_status(path: str = "/") -> Dict[str, Any]:
    logger.info(f"Getting BTRFS scrub status for {path}")

    err = await _check_btrfs(path)
    if err:
        return err

    exit_code, stdout, stderr = await run_command(
        ["btrfs", "scrub", "status", path], timeout=10, check=False
    )

    if exit_code != 0:
        return create_error_response(
            "CommandError", f"btrfs scrub status failed: {stderr.strip()}"
        )

    status = {}
    for line in stdout.strip().split("\n"):
        line = line.strip()
        if ":" in line:
            key, _, val = line.partition(":")
            status[key.strip().lower().replace(" ", "_")] = val.strip()

    logger.info(f"BTRFS scrub status: {status.get('uuid', 'no scrub data')}")
    return {"scrub": status}


async def start_scrub(path: str = "/", background: bool = True) -> Dict[str, Any]:
    logger.info(f"Starting BTRFS scrub on {path}")

    err = await _check_btrfs(path)
    if err:
        return err

    cmd = ["btrfs", "scrub", "start"]
    if background:
        cmd.append("-B")
    cmd.append(path)

    timeout = 300 if not background else 30

    exit_code, stdout, stderr = await run_command(cmd, timeout=timeout, check=False)

    if exit_code != 0:
        return create_error_response(
            "ScrubError", f"Failed to start scrub: {stderr.strip()}"
        )

    logger.info("BTRFS scrub started")
    return {
        "started": True,
        "output": stdout.strip() if stdout else "scrub started in background",
        "background": not background,
    }


async def cancel_scrub(path: str = "/") -> Dict[str, Any]:
    logger.info(f"Cancelling BTRFS scrub on {path}")

    err = await _check_btrfs(path)
    if err:
        return err

    exit_code, stdout, stderr = await run_command(
        ["btrfs", "scrub", "cancel", path], timeout=10, check=False
    )

    if exit_code != 0:
        return create_error_response(
            "ScrubCancelError", f"Failed to cancel scrub: {stderr.strip()}"
        )

    logger.info("BTRFS scrub cancelled")
    return {"cancelled": True}


# ============================================================================
# Snapshots (snapper integration)
# ============================================================================


async def list_snapshots(config: str = "root") -> Dict[str, Any]:
    logger.info(f"Listing BTRFS snapshots via snapper (config={config})")

    if not check_command_exists("snapper"):
        return create_error_response(
            "NotSupported",
            "snapper is not installed. Install with: sudo pacman -S snapper",
        )

    exit_code, stdout, stderr = await run_command(
        ["snapper", "-c", config, "list"], timeout=15, check=False
    )

    if exit_code != 0:
        return create_error_response(
            "CommandError", f"snapper list failed: {stderr.strip()}"
        )

    snapshots = []
    lines = stdout.strip().split("\n")
    found_header = False

    for line in lines:
        line_stripped = line.strip()
        if not line_stripped:
            continue

        if line_stripped.startswith("#") or line_stripped.startswith("-"):
            found_header = True
            continue

        if not found_header:
            continue

        normalized = line_stripped.replace("│", "|")
        parts = normalized.split(" | ")
        if len(parts) >= 6:
            desc = parts[6].rstrip(" |") if len(parts) > 6 else ""
            snap = {
                "number": parts[0].strip(),
                "type": parts[1].strip(),
                "pre_number": parts[2].strip() if parts[2].strip() else None,
                "date": parts[3].strip(),
                "user": parts[4].strip(),
                "cleanup": parts[5].strip() if len(parts) > 5 else "",
                "description": desc.strip(),
            }
            snapshots.append(snap)

    logger.info(f"Snapshots: {len(snapshots)} found")
    return {"config": config, "snapshot_count": len(snapshots), "snapshots": snapshots}


async def get_snapper_configs() -> Dict[str, Any]:
    logger.info("Getting snapper configurations")

    if not check_command_exists("snapper"):
        return create_error_response("NotSupported", "snapper is not installed.")

    exit_code, stdout, _ = await run_command(
        ["snapper", "list-configs"], timeout=10, check=False
    )

    if exit_code != 0:
        return create_error_response("CommandError", "snapper list-configs failed")

    configs = []
    lines = stdout.strip().split("\n")
    found_header = False

    for line in lines:
        line_stripped = line.strip()
        if not line_stripped:
            continue
        if (
            line_stripped.startswith("Config")
            or line_stripped.startswith("-")
            or line_stripped.startswith("\u2500")
        ):
            found_header = True
            continue
        if not found_header:
            continue

        normalized = line_stripped.replace("│", "|")
        parts = normalized.split("|")
        if len(parts) >= 1:
            config_name = parts[0].strip()
            subvolume = parts[1].strip() if len(parts) > 1 else ""

            if config_name:
                config_info = {"name": config_name, "subvolume": subvolume}

                exit_code2, stdout2, _ = await run_command(
                    ["snapper", "-c", config_name, "get-config"],
                    timeout=10,
                    check=False,
                )

                if exit_code2 == 0:
                    cfg = {}
                    for cfg_line in stdout2.strip().split("\n"):
                        cfg_line = cfg_line.strip()
                        normalized_cfg = cfg_line.replace("│", "|")
                        if "|" in normalized_cfg:
                            k, _, v = normalized_cfg.partition("|")
                            cfg[k.strip()] = v.strip()
                    config_info["settings"] = cfg

                configs.append(config_info)

    logger.info(f"Snapper configs: {len(configs)} found")
    return {"config_count": len(configs), "configs": configs}


async def create_snapshot(
    description: str,
    snap_type: str = "single",
    pre_number: Optional[int] = None,
    config: str = "root",
    cleanup: str = "number",
) -> Dict[str, Any]:
    logger.info(f"Creating snapshot: {description} (type={snap_type})")

    if not check_command_exists("snapper"):
        return create_error_response("NotSupported", "snapper is not installed.")

    cmd = ["snapper", "-c", config, "create", "-p", "-d", description]

    if snap_type == "pre":
        cmd.append("-t")
        cmd.append("pre")
    elif snap_type == "post":
        cmd.append("-t")
        cmd.append("post")
        if pre_number is not None:
            cmd.extend(["--pre-number", str(pre_number)])

    if cleanup:
        cmd.extend(["-c", cleanup])

    exit_code, stdout, stderr = await run_command(cmd, timeout=30, check=False)

    if exit_code != 0:
        return create_error_response(
            "SnapshotError", f"Failed to create snapshot: {stderr.strip()}"
        )

    snapshot_id = stdout.strip()
    logger.info(f"Snapshot created: id={snapshot_id}")
    return {
        "created": True,
        "snapshot_id": snapshot_id,
        "description": description,
        "type": snap_type,
    }


async def delete_snapshot(snapshot_id: int, config: str = "root") -> Dict[str, Any]:
    logger.info(f"Deleting snapshot: {snapshot_id}")

    if not check_command_exists("snapper"):
        return create_error_response("NotSupported", "snapper is not installed.")

    exit_code, stdout, stderr = await run_command(
        ["snapper", "-c", config, "delete", str(snapshot_id)], timeout=30, check=False
    )

    if exit_code != 0:
        return create_error_response(
            "SnapshotDeleteError",
            f"Failed to delete snapshot {snapshot_id}: {stderr.strip()}",
        )

    logger.info(f"Snapshot deleted: {snapshot_id}")
    return {"deleted": True, "snapshot_id": snapshot_id}


# ============================================================================
# Unified action dispatchers
# ============================================================================


async def analyze_btrfs(
    action: str, path: str = "/", config: str = "root"
) -> Dict[str, Any]:
    if action == "filesystem_info":
        return await get_filesystem_info(path)
    elif action == "filesystem_df":
        return await get_filesystem_df(path)
    elif action == "filesystem_usage":
        return await get_filesystem_usage(path)
    elif action == "subvolumes":
        return await list_subvolumes(path)
    elif action == "subvolume_info":
        return await get_subvolume_info(path)
    elif action == "device_stats":
        return await get_device_stats(path)
    elif action == "device_usage":
        return await get_device_usage(path)
    elif action == "properties":
        return await get_properties(path)
    elif action == "scrub_status":
        return await get_scrub_status(path)
    elif action == "snapshots":
        return await list_snapshots(config)
    elif action == "snapper_configs":
        return await get_snapper_configs()
    else:
        return create_error_response(
            "InvalidAction",
            f"Unknown action: {action}. Use one of: filesystem_info, filesystem_df, "
            "filesystem_usage, subvolumes, subvolume_info, device_stats, device_usage, "
            "properties, scrub_status, snapshots, snapper_configs",
        )


async def _snapper_diff(snap1: int, snap2: int, config: str = "root") -> Dict[str, Any]:
    """Compare two snapper snapshots with snapper diff."""
    logger.info(f"snapper diff {snap1}..{snap2} (config={config})")
    if not check_command_exists("snapper"):
        return create_error_response("NotSupported", "snapper is not installed")
    exit_code, stdout, stderr = await run_command(
        ["snapper", "-c", config, "diff", str(snap1), ".." + str(snap2)],
        timeout=30,
        check=False,
    )
    if exit_code != 0:
        return create_error_response("CommandError", f"snapper diff failed: {stderr}")
    lines = [l.strip() for l in stdout.strip().splitlines() if l.strip()]
    return {
        "config": config,
        "snap1": snap1,
        "snap2": snap2,
        "change_count": len(lines),
        "changes": lines,
    }


async def _snapper_rollback(
    snap_num: int, config: str = "root", dry: bool = False
) -> Dict[str, Any]:
    """Perform snapper rollback to a snapshot."""
    logger.info(f"snapper rollback {snap_num} (config={config}, dry={dry})")
    if not check_command_exists("snapper"):
        return create_error_response("NotSupported", "snapper is not installed")

    if dry:
        exit_code, stdout, stderr = await run_command(
            ["snapper", "-c", config, "status", str(snap_num), "..0"],
            timeout=10,
            check=False,
        )
        if exit_code != 0:
            return create_error_response(
                "CommandError", f"snapper status failed: {stderr}"
            )
        lines = [l.strip() for l in stdout.strip().splitlines() if l.strip()]
        return {
            "dry_run": True,
            "config": config,
            "snap_num": snap_num,
            "changes_count": len(lines),
            "changes": lines,
            "message": f"Rollback preview for snapshot {snap_num}. {len(lines)} changes would be applied.",
        }

    exit_code, stdout, stderr = await run_command(
        ["snapper", "-c", config, "rollback", str(snap_num)],
        timeout=30,
        check=False,
    )
    if exit_code != 0:
        return create_error_response(
            "CommandError", f"snapper rollback failed: {stderr}"
        )
    return {
        "rolled_back": True,
        "config": config,
        "snap_num": snap_num,
        "output": stdout.strip(),
        "message": f"Rolled back to snapshot {snap_num}. The default subvolume has been set to the rollback snapshot. Reboot to apply changes.",
    }


async def _snapper_create_config(
    name: str, subvolume: str, timeline: bool = True
) -> Dict[str, Any]:
    """Create a new snapper configuration."""
    logger.info(f"snapper create-config {name} for {subvolume}")
    if not check_command_exists("snapper"):
        return create_error_response("NotSupported", "snapper is not installed")

    cmd = ["snapper", "-c", name, "create-config", subvolume]
    exit_code, stdout, stderr = await run_command(cmd, timeout=10, check=False)
    if exit_code != 0:
        return create_error_response(
            "CommandError", f"snapper create-config failed: {stderr}"
        )

    if not timeline:
        exit_code2, _, err2 = await run_command(
            ["snapper", "-c", name, "set-config", "TIMELINE_CREATE=no"],
            timeout=5,
            check=False,
        )
        if exit_code2 != 0:
            logger.warning(f"Failed to disable timeline: {err2}")

    return {
        "created": True,
        "name": name,
        "subvolume": subvolume,
        "timeline_enabled": timeline,
        "message": f"Snapper config '{name}' created for {subvolume}",
    }


async def manage_btrfs_snapshots(
    action: str,
    description: str = "",
    snap_type: str = "single",
    pre_number: Optional[int] = None,
    snapshot_id: Optional[int] = None,
    config: str = "root",
    cleanup: str = "number",
    snap1: Optional[int] = None,
    snap2: Optional[int] = None,
    snap_num: Optional[int] = None,
    name: Optional[str] = None,
    subvolume: Optional[str] = None,
    timeline: bool = True,
) -> Dict[str, Any]:
    if action == "list":
        return await list_snapshots(config)
    elif action == "configs":
        return await get_snapper_configs()
    elif action == "create":
        if not description:
            return create_error_response(
                "MissingArgument", "description is required for create action"
            )
        return await create_snapshot(
            description, snap_type, pre_number, config, cleanup
        )
    elif action == "delete":
        if snapshot_id is None:
            return create_error_response(
                "MissingArgument", "snapshot_id is required for delete action"
            )
        return await delete_snapshot(snapshot_id, config)
    elif action == "diff":
        if snap1 is None or snap2 is None:
            return create_error_response(
                "MissingArgument", "snap1 and snap2 are required for diff action"
            )
        return await _snapper_diff(snap1, snap2, config)
    elif action == "rollback":
        if snap_num is None:
            return create_error_response(
                "MissingArgument", "snap_num is required for rollback action"
            )
        return await _snapper_rollback(snap_num, config, dry=False)
    elif action == "rollback_dry":
        if snap_num is None:
            return create_error_response(
                "MissingArgument", "snap_num is required for rollback_dry action"
            )
        return await _snapper_rollback(snap_num, config, dry=True)
    elif action == "create_config":
        if not name or not subvolume:
            return create_error_response(
                "MissingArgument",
                "name and subvolume are required for create_config action",
            )
        return await _snapper_create_config(name, subvolume, timeline)
    else:
        return create_error_response(
            "InvalidAction",
            f"Unknown action: {action}. Use one of: list, configs, create, delete, diff, rollback, rollback_dry, create_config",
        )


async def manage_btrfs_scrub(
    action: str, path: str = "/", background: bool = True
) -> Dict[str, Any]:
    if action == "status":
        return await get_scrub_status(path)
    elif action == "start":
        return await start_scrub(path, background)
    elif action == "cancel":
        return await cancel_scrub(path)
    else:
        return create_error_response(
            "InvalidAction",
            f"Unknown action: {action}. Use one of: status, start, cancel",
        )
