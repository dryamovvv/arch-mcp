# SPDX-License-Identifier: GPL-3.0-only OR MIT
"""
RPi5 hardware management module.
Temperature, frequencies, throttling, EEPROM, NVMe info, disk benchmarks.
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


async def manage_hardware(
    action: str,
    channel: str = "default",
    path: str = "/",
    size: str = "1G",
) -> Dict[str, Any]:
    """
    Manage RPi5 hardware: health, EEPROM, NVMe, disk benchmarks.

    Actions:
      health          — temperature, throttling, frequencies via vcgencmd
      eeprom_info     — full rpi-eeprom-config output
      eeprom_update   — update EEPROM (requires channel)
      nvme_info       — nvme list + smart-log
      benchmark_disk  — fio-based disk benchmark
    """
    if not IS_ARCH:
        return create_error_response(
            "NotSupported", "This feature is only available on Arch Linux."
        )

    if action == "health":
        logger.info("Checking RPi5 hardware health")

        if not check_command_exists("vcgencmd"):
            return create_error_response(
                "NotSupported", "vcgencmd is only available on Raspberry Pi devices."
            )

        exit_code, out, err = await run_command(
            ["vcgencmd", "measure_temp"],
            timeout=5,
            check=False,
        )
        temp_raw = out.strip() if exit_code == 0 else "unavailable"

        exit_code, out, err = await run_command(
            ["vcgencmd", "get_throttled"],
            timeout=5,
            check=False,
        )
        throttled_raw = out.strip() if exit_code == 0 else "unavailable"
        throttled = _decode_throttled(throttled_raw)

        freq_info = {}
        for src in [
            "arm",
            "core",
            "h264",
            "isp",
            "v3d",
            "uart",
            "pwm",
            "emmc",
            "pixel",
            "vec",
            "hdmi",
            "dpi",
        ]:
            exit_code, out, err = await run_command(
                ["vcgencmd", "measure_clock", src],
                timeout=5,
                check=False,
            )
            if exit_code == 0 and out.strip():
                freq_info[src] = out.strip()

        volt_info = {}
        for src in ["core", "sdram_c", "sdram_i", "sdram_p"]:
            exit_code, out, err = await run_command(
                ["vcgencmd", "measure_volts", src],
                timeout=5,
                check=False,
            )
            if exit_code == 0 and out.strip():
                volt_info[src] = out.strip()

        return {
            "temperature": temp_raw,
            "throttled_raw": throttled_raw,
            "throttled": throttled,
            "frequencies": freq_info,
            "voltages": volt_info,
        }

    elif action == "eeprom_info":
        logger.info("Reading EEPROM configuration")
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
        return {"eeprom_config": out.strip()}

    elif action == "eeprom_update":
        logger.info(f"Updating EEPROM (channel={channel})")
        if not check_command_exists("rpi-eeprom-update"):
            return create_error_response(
                "NotSupported", "rpi-eeprom-update not installed."
            )
        exit_code, out, err = await run_command(
            ["rpi-eeprom-update", "-a"],
            timeout=60,
            check=False,
        )
        if exit_code != 0:
            return create_error_response("CommandError", f"EEPROM update failed: {err}")
        return {
            "ok": True,
            "channel": channel,
            "output": out.strip(),
            "message": "EEPROM update check completed",
        }

    elif action == "nvme_info":
        logger.info("Reading NVMe device info")
        if not check_command_exists("nvme"):
            return create_error_response(
                "NotSupported",
                "nvme-cli not installed. Install with: sudo pacman -S nvme-cli",
            )

        result = {}

        exit_code, out, err = await run_command(
            ["nvme", "list"],
            timeout=10,
            check=False,
        )
        if exit_code == 0:
            result["nvme_list"] = out.strip()

        exit_code, out, err = await run_command(
            ["nvme", "smart-log", "/dev/nvme0"],
            timeout=10,
            check=False,
        )
        if exit_code == 0:
            result["smart_log"] = out.strip()
        elif exit_code != 0:
            result["smart_error"] = err.strip()

        return result

    elif action == "benchmark_disk":
        logger.info(f"Running disk benchmark on {path} (size={size})")
        if not check_command_exists("fio"):
            return create_error_response(
                "NotSupported", "fio not installed. Install with: sudo pacman -S fio"
            )

        exit_code, out, err = await run_command(
            [
                "fio",
                f"--directory={path}",
                "--name=benchmark",
                f"--size={size}",
                "--rw=randread",
                "--bs=4k",
                "--direct=1",
                "--numjobs=1",
                "--time_based",
                "--runtime=10",
                "--group_reporting",
                "--output-format=json",
            ],
            timeout=30,
            check=False,
        )
        if exit_code != 0:
            return create_error_response("CommandError", f"fio benchmark failed: {err}")

        import json

        try:
            fio_data = json.loads(out)
            return {"benchmark": fio_data, "path": path, "size": size}
        except json.JSONDecodeError:
            return {"raw_output": out.strip(), "path": path, "size": size}

    else:
        return create_error_response(
            "InvalidAction",
            f"Unknown action: '{action}'. Valid actions: health, eeprom_info, eeprom_update, nvme_info, benchmark_disk.",
        )


def _decode_throttled(raw: str) -> Dict[str, bool]:
    """Decode vcgencmd get_throttled hex value."""
    throttled = {
        "under_voltage": False,
        "arm_freq_capped": False,
        "throttled": False,
        "soft_temp_limit": False,
        "under_voltage_occurred": False,
        "arm_freq_capped_occurred": False,
        "throttled_occurred": False,
        "soft_temp_limit_occurred": False,
    }
    try:
        val_str = raw.split("=")[-1].strip()
        val = int(val_str, 16)
        throttled["under_voltage"] = bool(val & (1 << 0))
        throttled["arm_freq_capped"] = bool(val & (1 << 1))
        throttled["throttled"] = bool(val & (1 << 2))
        throttled["soft_temp_limit"] = bool(val & (1 << 3))
        throttled["under_voltage_occurred"] = bool(val & (1 << 16))
        throttled["arm_freq_capped_occurred"] = bool(val & (1 << 17))
        throttled["throttled_occurred"] = bool(val & (1 << 18))
        throttled["soft_temp_limit_occurred"] = bool(val & (1 << 19))
    except (ValueError, IndexError):
        pass
    return throttled
