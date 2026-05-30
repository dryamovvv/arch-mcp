# SPDX-License-Identifier: GPL-3.0-only OR MIT
"""
LUKS encryption management module.
Provides cryptsetup-based LUKS operations: status, key management, unlock checks.
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

CRYPTROOT = "/dev/mapper/cryptroot"


async def _cryptsetup(args: list[str]) -> tuple[int, str, str]:
    return await run_command(["cryptsetup"] + args, timeout=15, check=False)


async def manage_luks(
    action: str,
    device: Optional[str] = None,
    old_pass: Optional[str] = None,
    new_pass: Optional[str] = None,
    key_file: Optional[str] = None,
    slot: Optional[int] = None,
) -> Dict[str, Any]:
    """
    Manage LUKS encryption: status, key management, and unlock checks.

    Actions:
      status          — cryptsetup luksDump + /dev/mapper/cryptroot status
      change_password — change LUKS passphrase (requires old_pass, new_pass)
      add_key         — add a new key slot (pass or key_file)
      remove_key      — remove a key from a slot
      is_unlocked     — check if /dev/mapper/cryptroot is active
    """
    if not IS_ARCH:
        return create_error_response(
            "NotSupported", "This feature is only available on Arch Linux."
        )

    if not check_command_exists("cryptsetup"):
        return create_error_response(
            "NotSupported",
            "cryptsetup not installed. Install with: sudo pacman -S cryptsetup",
        )

    if action == "status":
        logger.info("Reading LUKS status")
        if not device:
            result = {"mapper_active": False}

            exit_code, out, err = await run_command(
                ["lsblk", "-o", "NAME,TYPE,MOUNTPOINT", "-n"],
                timeout=5,
                check=False,
            )
            if exit_code == 0:
                for line in out.strip().splitlines():
                    if "crypt" in line.lower():
                        result["mapper_active"] = True
                        break

            luks_dev = await _find_luks_device()
            if not luks_dev:
                result["luks_device"] = None
                result["message"] = "No LUKS device found"
                return result

            result["luks_device"] = luks_dev
            device = luks_dev

        exit_code, out, err = await _cryptsetup(["luksDump", device])
        if exit_code != 0:
            return create_error_response(
                "CommandError", f"cryptsetup luksDump failed: {err}"
            )

        result = {
            "luks_device": device,
            "mapper_active": True,
            "luks_dump": out.strip(),
        }

        exit_code, out, err = await run_command(
            ["lsblk", "-o", "NAME,SIZE,TYPE,MOUNTPOINT", "-n", CRYPTROOT],
            timeout=5,
            check=False,
        )
        if exit_code == 0 and out.strip():
            result["cryptroot_info"] = out.strip()

        return result

    elif action == "change_password":
        if not device or not old_pass or not new_pass:
            return create_error_response(
                "MissingArgument",
                "device, old_pass, and new_pass are required for change_password",
            )
        logger.info("Changing LUKS password")
        cmd = ["luksChangeKey", device, "--key-file", "/dev/stdin"]
        exit_code, out, err = await run_command(
            [
                "bash",
                "-c",
                f"echo '{old_pass}\n{new_pass}' | cryptsetup luksChangeKey {device} --key-file /dev/stdin",
            ],
            timeout=15,
            check=False,
        )
        if exit_code != 0:
            return create_error_response(
                "CommandError", f"Failed to change password: {err}"
            )
        return {"ok": True, "message": "LUKS password changed successfully"}

    elif action == "add_key":
        if not device:
            return create_error_response("MissingArgument", "device is required")
        if not new_pass and not key_file:
            return create_error_response(
                "MissingArgument", "new_pass or key_file is required for add_key"
            )
        logger.info("Adding LUKS key")

        if key_file:
            exit_code, out, err = await _cryptsetup(
                ["luksAddKey", device, "--key-file", key_file]
            )
        else:
            cmd = f"echo '{new_pass}' | cryptsetup luksAddKey {device} --key-file /dev/stdin"
            exit_code, out, err = await run_command(
                ["bash", "-c", cmd],
                timeout=15,
                check=False,
            )

        if exit_code != 0:
            return create_error_response("CommandError", f"Failed to add key: {err}")
        return {"ok": True, "message": f"Key added to {device}"}

    elif action == "remove_key":
        if not device or not new_pass:
            return create_error_response(
                "MissingArgument", "device and pass are required for remove_key"
            )
        logger.info(
            f"Removing LUKS key from slot {slot if slot is not None else '? (let cryptsetup determine)'}"
        )

        slot_args = ["-S", str(slot)] if slot is not None else []
        cmd = f"echo '{new_pass}' | cryptsetup luksRemoveKey {device} {' '.join(slot_args)} --key-file /dev/stdin"
        exit_code, out, err = await run_command(
            ["bash", "-c", cmd],
            timeout=15,
            check=False,
        )

        if exit_code != 0:
            return create_error_response("CommandError", f"Failed to remove key: {err}")
        return {"ok": True, "message": f"Key removed from {device}"}

    elif action == "is_unlocked":
        logger.info("Checking if cryptroot is unlocked")
        exit_code, out, err = await run_command(
            ["lsblk", "-o", "NAME,TYPE,MOUNTPOINT", "-n"],
            timeout=5,
            check=False,
        )
        unlocked = exit_code == 0 and "crypt" in out.lower()
        return {
            "unlocked": unlocked,
            "cryptroot": CRYPTROOT if unlocked else None,
        }

    else:
        return create_error_response(
            "InvalidAction",
            f"Unknown action: '{action}'. Valid actions: status, change_password, add_key, remove_key, is_unlocked.",
        )


async def _find_luks_device() -> Optional[str]:
    """Find the LUKS device by scanning block devices."""
    exit_code, out, err = await run_command(
        ["lsblk", "-o", "NAME,TYPE,FSTYPE", "-n"],
        timeout=5,
        check=False,
    )
    if exit_code != 0:
        return None
    for line in out.strip().splitlines():
        if "crypto_LUKS" in line:
            parts = line.strip().split()
            name = parts[0] if parts else ""
            if name:
                return f"/dev/{name}"
    return None
