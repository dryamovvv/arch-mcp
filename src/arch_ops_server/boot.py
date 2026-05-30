# SPDX-License-Identifier: GPL-3.0-only OR MIT
"""
Raspberry Pi bootloader management module.
Provides BOOT_ORDER inspection, permanent changes, and one-time next-boot device selection.
"""

import logging
import re
from pathlib import Path
from typing import Dict, Any, List, Optional

from .utils import (
    IS_ARCH,
    run_command,
    create_error_response,
    check_command_exists,
)

logger = logging.getLogger(__name__)

_STATE_FILE = Path("/var/lib/arch-ops-server/boot_order_restore")
_SERVICE_FILE = Path("/etc/systemd/system/arch-ops-boot-restore.service")

_BOOT_MODE_LABELS = {
    "0x0": "STOP",
    "0x1": "SD",
    "0x2": "NETWORK",
    "0x3": "RPIBOOT",
    "0x4": "USB",
    "0x5": "BCM-USB",
    "0x6": "NVMe",
    "0x7": "HTTP",
    "0xe": "STOP",
    "0xf": "RESTART",
}

_ORDER_PRESETS = {
    "sd_first": "0xf41",
    "nvme_first": "0xf46",
    "usb_first": "0xf14",
    "sd_nvme": "0xf16",
    "nvme_sd": "0xf61",
    "sd_only": "0xf1",
    "nvme_only": "0xf6",
    "usb_only": "0xf4",
}


def _decode_boot_order(boot_order: str) -> List[Dict[str, str]]:
    """Decode a BOOT_ORDER hex string into a list of human-readable modes."""
    hex_val = boot_order.lower().replace("0x", "")
    sequence = []
    for ch in reversed(hex_val):
        key = f"0x{ch}"
        sequence.append({
            "mode": key,
            "label": _BOOT_MODE_LABELS.get(key, f"UNKNOWN({key})"),
        })
    return sequence


async def _get_eeprom_config() -> Dict[str, str]:
    """Parse current EEPROM config into a dict. Returns empty dict on failure."""
    exit_code, stdout, _ = await run_command(
        ["rpi-eeprom-config"],
        timeout=10,
        check=False,
    )
    if exit_code != 0:
        return {}

    config = {}
    for line in stdout.strip().splitlines():
        line = line.strip()
        if "=" in line and not line.startswith("#"):
            key, _, val = line.partition("=")
            config[key.strip()] = val.strip()
    return config


async def _run_eeprom_update(key: str, value: str) -> Dict[str, Any]:
    """Update a single key in the EEPROM config via temp file."""
    exit_code, stdout, stderr = await run_command(
        ["rpi-eeprom-config"],
        timeout=10,
        check=False,
    )
    if exit_code != 0:
        return create_error_response("CommandError", f"Failed to read EEPROM: {stderr}")

    import tempfile
    with tempfile.NamedTemporaryFile(mode="w", suffix=".txt", delete=False) as tf:
        for line in stdout.strip().splitlines():
            line = line.strip()
            if line.startswith(f"{key}="):
                tf.write(f"{key}={value}\n")
            elif line and not line.startswith("#"):
                tf.write(f"{line}\n")
            elif line.startswith("#"):
                tf.write(f"{line}\n")
        tf.flush()
        tmp_path = tf.name

    exit_code, _, stderr = await run_command(
        ["rpi-eeprom-config", "--apply", tmp_path],
        timeout=15,
        check=False,
    )
    Path(tmp_path).unlink(missing_ok=True)
    if exit_code != 0:
        return create_error_response("CommandError", f"Failed to update EEPROM: {stderr}")
    return {"ok": True}


async def _detect_boot_devices() -> Dict[str, bool]:
    """Detect which boot devices are available (SD, NVMe, USB)."""
    devices = {"sd": False, "nvme": False, "usb": False}
    exit_code, stdout, _ = await run_command(
        ["lsblk", "-o", "NAME,TYPE,TRAN", "-n"],
        timeout=5,
        check=False,
    )
    if exit_code != 0:
        return devices

    for line in stdout.strip().splitlines():
        parts = line.strip().split()
        if len(parts) < 2:
            continue
        name = parts[0]
        if name.startswith("mmcblk"):
            devices["sd"] = True
        elif name.startswith("nvme"):
            devices["nvme"] = True
        elif name.startswith("sd"):
            devices["usb"] = True

    return devices


async def _install_restore_service(original_order: str) -> Dict[str, Any]:
    """Install a oneshot systemd service that restores BOOT_ORDER on next boot."""

    service_content = f"""[Unit]
Description=Restore RPi BOOT_ORDER after one-time boot
DefaultDependencies=no
After=local-fs.target
Before=basic.target

[Service]
Type=oneshot
ExecStart=/usr/bin/rpi-eeprom-config --apply BOOT_ORDER={original_order}
ExecStart=/usr/bin/rm /var/lib/arch-ops-server/boot_order_restore
ExecStart=/usr/bin/systemctl disable arch-ops-boot-restore.service
RemainAfterExit=no

[Install]
WantedBy=basic.target
"""

    _STATE_FILE.parent.mkdir(parents=True, exist_ok=True)
    _STATE_FILE.write_text(original_order)

    try:
        _SERVICE_FILE.parent.mkdir(parents=True, exist_ok=True)
        _SERVICE_FILE.write_text(service_content)
    except OSError as e:
        return create_error_response("WriteError", f"Failed to write service file: {str(e)}")

    exit_code, _, stderr = await run_command(
        ["systemctl", "daemon-reload"],
        timeout=5,
        check=False,
    )
    if exit_code != 0:
        return create_error_response("SystemdError", f"daemon-reload failed: {stderr}")

    exit_code, _, stderr = await run_command(
        ["systemctl", "enable", "arch-ops-boot-restore.service"],
        timeout=5,
        check=False,
    )
    if exit_code != 0:
        return create_error_response("SystemdError", f"enable failed: {stderr}")

    return {"restore_service_installed": True, "original_order": original_order}


async def manage_boot(
    action: str,
    order: Optional[str] = None,
    device: Optional[str] = None,
    reboot: bool = False,
) -> Dict[str, Any]:
    """
    Manage Raspberry Pi bootloader configuration.

    Actions:
      status         — read current BOOT_ORDER, decode it, detect available boot devices
      set_boot_order — permanently change BOOT_ORDER (use preset name or raw hex)
      next_boot      — set next boot device as one-time, reboot
    """
    if not IS_ARCH:
        return create_error_response(
            "NotSupported",
            f"manage_boot(action='{action}') requires Arch Linux running on physical RPi hardware.",
        )

    if not check_command_exists("rpi-eeprom-config"):
        return create_error_response(
            "NotSupported",
            "rpi-eeprom-config is not installed. Install with: sudo pacman -S rpi-eeprom",
        )

    if action == "status":
        logger.info("Reading bootloader configuration")
        config = await _get_eeprom_config()
        if not config:
            return create_error_response("CommandError", "Failed to read EEPROM configuration")

        boot_order = config.get("BOOT_ORDER", "unknown")
        sequence = _decode_boot_order(boot_order)
        devices = await _detect_boot_devices()
        has_restore = _STATE_FILE.exists()

        return {
            "boot_order": boot_order,
            "sequence": sequence,
            "available_devices": devices,
            "has_pending_restore": has_restore,
            "config_keys": list(config.keys()),
        }

    elif action == "set_boot_order":
        if not order:
            return create_error_response("MissingArgument", "order parameter is required")

        if order in _ORDER_PRESETS:
            boot_order = _ORDER_PRESETS[order]
        else:
            if not re.match(r"^0x[0-9a-fA-F]+$", order):
                return create_error_response(
                    "InvalidValue",
                    f"Invalid boot order: '{order}'. Use a preset ({', '.join(_ORDER_PRESETS.keys())}) or raw hex like 0xf416.",
                )
            boot_order = order

        logger.info(f"Setting BOOT_ORDER to {boot_order}")
        result = await _run_eeprom_update("BOOT_ORDER", boot_order)
        if "error" in result:
            return result

        sequence = _decode_boot_order(boot_order)
        return {
            "set": True,
            "boot_order": boot_order,
            "sequence": sequence,
            "message": f"BOOT_ORDER set to {boot_order}. Reboot to apply.",
        }

    elif action == "next_boot":
        if not device:
            return create_error_response("MissingArgument", "device parameter is required (sd, nvme, usb)")

        device = device.lower()
        if device not in ("sd", "nvme", "usb"):
            return create_error_response(
                "InvalidValue",
                f"Invalid device: '{device}'. Use: sd, nvme, usb.",
            )

        config = await _get_eeprom_config()
        if not config:
            return create_error_response("CommandError", "Failed to read EEPROM configuration")

        original_order = config.get("BOOT_ORDER", "0xf41")
        logger.info(f"Saving original BOOT_ORDER: {original_order}")

        device_nibble = {"sd": "1", "nvme": "6", "usb": "4"}[device]
        temp_order = f"0xf{device_nibble}"

        result = await _run_eeprom_update("BOOT_ORDER", temp_order)
        if "error" in result:
            return result

        result = await _install_restore_service(original_order)
        if "error" in result:
            return result

        if not reboot:
            return {
                "temp_order_set": True,
                "temp_order": temp_order,
                "original_order_saved": original_order,
                "restore_service_installed": True,
                "message": f"Next boot set to {device.upper()} ({temp_order}). Reboot manually to apply.",
            }

        logger.warning(f"Rebooting to {device.upper()} device")
        exit_code, _, stderr = await run_command(
            ["reboot"],
            timeout=5,
            check=False,
        )
        if exit_code != 0:
            return create_error_response("CommandError", f"Reboot failed: {stderr}")

        return {
            "rebooting": True,
            "device": device,
            "temp_order": temp_order,
            "message": f"Rebooting to {device.upper()} device. Original BOOT_ORDER ({original_order}) will be restored automatically.",
        }

    else:
        return create_error_response(
            "InvalidAction",
            f"Unknown action: '{action}'. Valid actions: status, set_boot_order, next_boot.",
        )
