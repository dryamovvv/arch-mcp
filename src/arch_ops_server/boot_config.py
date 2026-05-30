# SPDX-License-Identifier: GPL-3.0-only OR MIT
"""
Boot configuration management module.
Reads/writes config.txt, cmdline.txt, verifies boot artifacts, checks initramfs hooks.
"""

import logging
from pathlib import Path
from typing import Dict, Any

from .utils import (
    IS_ARCH,
    run_command,
    create_error_response,
    check_command_exists,
)

logger = logging.getLogger(__name__)

BOOT_DIR = "/boot"
CONFIG_TXT = f"{BOOT_DIR}/config.txt"
CMDLINE_TXT = f"{BOOT_DIR}/cmdline.txt"


async def manage_boot_config(
    action: str,
) -> Dict[str, Any]:
    """
    Manage RPi5 boot configuration: config.txt, cmdline.txt, initramfs hooks, boot order.

    Actions:
      read_config           — read /boot/config.txt
      read_cmdline          — read /boot/cmdline.txt
      verify_boot           — check for kernel, initramfs, dtb, config, cmdline
      check_initramfs_hooks — list initramfs hooks (encrypt, telegram, boot-mount)
      check_boot_order      — read BOOT_ORDER from EEPROM
    """
    if not IS_ARCH:
        return create_error_response(
            "NotSupported", "This feature is only available on Arch Linux."
        )

    if action == "read_config":
        logger.info("Reading boot config.txt")
        conf = Path(CONFIG_TXT)
        if not conf.exists():
            return create_error_response("NotFound", f"{CONFIG_TXT} not found")

        content = conf.read_text()
        return {
            "path": CONFIG_TXT,
            "content": content,
            "line_count": len(content.splitlines()),
            "overrides": _parse_config_txt(content),
        }

    elif action == "read_cmdline":
        logger.info("Reading boot cmdline.txt")
        cmdline = Path(CMDLINE_TXT)
        if not cmdline.exists():
            return create_error_response("NotFound", f"{CMDLINE_TXT} not found")

        content = cmdline.read_text().strip()
        params = {}
        for part in content.split():
            if "=" in part:
                k, v = part.split("=", 1)
                params[k] = v
            else:
                params[part] = ""

        return {
            "path": CMDLINE_TXT,
            "content": content,
            "parsed_params": params,
            "has_encrypt": "cryptdevice" in content.lower()
            or "rd.luks" in content.lower(),
            "has_rootflags": "rootflags" in content.lower(),
            "has_btrfs_subvol": "subvol=" in content.lower(),
        }

    elif action == "verify_boot":
        logger.info("Verifying boot artifacts in /boot")

        expected = [
            "config.txt",
            "cmdline.txt",
            "kernel8.img",
        ]

        result = {"present": [], "missing": [], "size_bytes": {}, "boot_dir": BOOT_DIR}

        import os

        # Check initramfs
        for entry in os.listdir(BOOT_DIR) if os.path.isdir(BOOT_DIR) else []:
            if entry.startswith("initramfs") and entry.endswith(".img"):
                expected.append(entry)
                break
        else:
            result["missing"].append("initramfs-linux*.img")

        # Check DTB
        for entry in os.listdir(BOOT_DIR) if os.path.isdir(BOOT_DIR) else []:
            if entry.endswith(".dtb") and "bcm2712" in entry:
                expected.append(entry)
                break
        else:
            result["missing"].append("bcm2712*.dtb")

        for name in expected:
            p = Path(BOOT_DIR) / name
            if name.startswith("initramfs") or name.startswith("bcm2712"):
                found = list(Path(BOOT_DIR).glob(name))
                if found:
                    result["present"].append(str(found[0].name))
                    result["size_bytes"][str(found[0].name)] = found[0].stat().st_size
            elif p.exists():
                result["present"].append(name)
                result["size_bytes"][name] = p.stat().st_size
            else:
                result["missing"].append(name)

        result["all_ok"] = len(result["missing"]) == 0
        return result

    elif action == "check_initramfs_hooks":
        logger.info("Checking initramfs hooks")

        result = {"hooks": [], "found": []}

        import os

        for entry in os.listdir(BOOT_DIR) if os.path.isdir(BOOT_DIR) else []:
            if entry.startswith("initramfs") and entry.endswith(".img"):
                img_path = f"{BOOT_DIR}/{entry}"
                break
        else:
            return create_error_response(
                "NotFound", "No initramfs image found in /boot"
            )

        exit_code, out, err = await run_command(
            ["lsinitcpio", img_path],
            timeout=10,
            check=False,
        )
        if exit_code != 0:
            return create_error_response("CommandError", f"lsinitcpio failed: {err}")

        for line in out.strip().splitlines():
            result["hooks"].append(line.strip())

        targets = [
            "boot-mount",
            "sd-encrypt",
            "telegram",
            "encrypt",
            "lvm2",
            "mdadm",
            "resume",
        ]
        for target in targets:
            for line in result["hooks"]:
                if target in line.lower():
                    result["found"].append(target)
                    break

        return result

    elif action == "check_boot_order":
        logger.info("Checking BOOT_ORDER from EEPROM")

        if not check_command_exists("rpi-eeprom-config"):
            return create_error_response(
                "NotSupported", "rpi-eeprom-config not installed."
            )

        exit_code, out, err = await run_command(
            ["rpi-eeprom-config"],
            timeout=10,
            check=False,
        )
        if exit_code != 0:
            return create_error_response(
                "CommandError", f"rpi-eeprom-config failed: {err}"
            )

        config = {}
        boot_order = "unknown"
        for line in out.strip().splitlines():
            line = line.strip()
            if "=" in line and not line.startswith("#"):
                k, _, v = line.partition("=")
                config[k.strip()] = v.strip()
                if k.strip() == "BOOT_ORDER":
                    boot_order = v.strip()

        return {
            "boot_order": boot_order,
            "full_config": config,
        }

    else:
        return create_error_response(
            "InvalidAction",
            f"Unknown action: '{action}'. Valid actions: read_config, read_cmdline, verify_boot, check_initramfs_hooks, check_boot_order.",
        )


def _parse_config_txt(content: str) -> dict:
    """Parse config.txt overrides (non-commented key=value pairs)."""
    overrides = {}
    for line in content.splitlines():
        stripped = line.strip()
        if stripped and not stripped.startswith("#") and "=" in stripped:
            k, v = stripped.split("=", 1)
            overrides[k.strip()] = v.strip()
    return overrides
