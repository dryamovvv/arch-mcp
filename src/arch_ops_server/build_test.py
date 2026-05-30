# SPDX-License-Identifier: GPL-3.0-only OR MIT
"""
OS build testing tools — validate RPi5 Arch Linux images after build/boot.
"""

import json
import logging
import re
from pathlib import Path
from typing import Any, Dict, List, Optional

from .utils import IS_ARCH, check_command_exists, create_error_response, run_command

logger = logging.getLogger(__name__)

_BOOT_PATH = Path("/boot")
_ESP_PATH = Path("/boot/efi")
_CRITICAL_BOOT_FILES = [
    "kernel8.img",
    "initramfs-linux.img",
    "bcm2712-rpi-5-b.dtb",
    "config.txt",
    "cmdline.txt",
]

_DEFAULT_CRITICAL_SERVICES = [
    "sshd.service",
    "systemd-networkd.service",
    "systemd-resolved.service",
    "snapper-timeline.timer",
]

_FSTAB_EXPECTED_SUBVOLS = [
    "@", "@home", "@snapshots", "@swap",
    "@var_log", "@var_cache", "@var_tmp", "@var_lib",
]


# ─── Tool 1: verify_boot_artifacts ───────────────────────────────────────────

async def verify_boot_artifacts(
    action: str = "check",
) -> Dict[str, Any]:
    """
    Validate boot partition files.

    Actions:
      list    — list all files on /boot
      check   — verify critical files exist
      cmdline — parse cmdline.txt
    """
    if not IS_ARCH:
        return create_error_response(
            "NotSupported",
            "verify_boot_artifacts requires Arch Linux (access to /boot).",
        )

    boot_path = _BOOT_PATH if _BOOT_PATH.exists() else _ESP_PATH
    if not boot_path.exists():
        return create_error_response("NotFound", f"Boot partition not found at {boot_path}")

    if action == "list":
        files = sorted([f.name for f in boot_path.iterdir()])
        return {"action": "list", "boot_path": str(boot_path), "file_count": len(files), "files": files}

    elif action == "check":
        present = []
        missing = []
        for fname in _CRITICAL_BOOT_FILES:
            path = boot_path / fname
            (present if path.exists() else missing).append(fname)
        return {
            "action": "check",
            "boot_path": str(boot_path),
            "present": present,
            "missing": missing,
            "all_ok": len(missing) == 0,
        }

    elif action == "cmdline":
        cmdline_path = boot_path / "cmdline.txt"
        if not cmdline_path.exists():
            return create_error_response("NotFound", "cmdline.txt not found on boot partition")

        raw = cmdline_path.read_text().strip()
        issues = []
        has_root_uuid = bool(re.search(r"root=UUID=[a-f0-9\-]+", raw))
        has_subvol = "rootflags=subvol=@" in raw or "subvol=@" in raw
        has_placeholder = "__ROOT_UUID__" in raw

        if not has_root_uuid:
            issues.append("Missing root=UUID=...")
        if not has_subvol:
            issues.append("Missing rootflags=subvol=@")
        if has_placeholder:
            issues.append("Contains __ROOT_UUID__ placeholder (unresolved)")

        return {
            "action": "cmdline",
            "raw": raw,
            "has_root_uuid": has_root_uuid,
            "has_subvol": has_subvol,
            "has_placeholder": has_placeholder,
            "issues": issues,
            "all_ok": len(issues) == 0,
        }

    else:
        return create_error_response("InvalidAction", f"Unknown action: {action}. Use: list, check, cmdline")


# ─── Tool 2: verify_service_health ──────────────────────────────────────────

async def verify_service_health(
    action: str = "status",
    checklist: Optional[List[str]] = None,
) -> Dict[str, Any]:
    """
    Aggregate systemd service health.

    Actions:
      status   — list services with state
      compare  — compare against checklist
      checklist — quick yes/no on critical services
    """
    if not IS_ARCH:
        return create_error_response("NotSupported", "verify_service_health requires Arch Linux.")

    if action == "status":
        services = []
        exit_code, stdout, stderr = await run_command(
            ["systemctl", "list-units", "--type=service", "--no-pager", "--no-legend"],
            timeout=10,
            check=False,
        )
        if exit_code != 0:
            return create_error_response("CommandError", f"systemctl failed: {stderr}")

        for line in stdout.strip().splitlines():
            parts = line.split()
            if len(parts) >= 4:
                services.append({"unit": parts[0], "loaded": parts[1], "active": parts[2], "sub": parts[3]})

        failed = [s for s in services if s["active"] == "failed"]
        return {
            "action": "status",
            "total": len(services),
            "failed_count": len(failed),
            "failed": failed,
            "services": services[:50],
        }

    elif action == "checklist":
        targets = checklist or _DEFAULT_CRITICAL_SERVICES
        results = {}
        for svc in targets:
            exit_code, _, _ = await run_command(
                ["systemctl", "is-active", "--quiet", svc],
                timeout=5,
                check=False,
            )
            results[svc] = "active" if exit_code == 0 else "inactive"
        all_ok = all(v == "active" for v in results.values())
        return {
            "action": "checklist",
            "results": results,
            "all_ok": all_ok,
            "checked": len(targets),
            "passed": sum(1 for v in results.values() if v == "active"),
        }

    elif action == "compare":
        if not checklist:
            return create_error_response("MissingArgument", "checklist parameter required for compare action")
        result = await verify_service_health(action="checklist", checklist=checklist)
        result["action"] = "compare"
        return result

    else:
        return create_error_response("InvalidAction", f"Unknown action: {action}. Use: status, checklist, compare")


# ─── Tool 3: verify_homectl_user ─────────────────────────────────────────────

async def verify_homectl_user(
    action: str = "check",
) -> Dict[str, Any]:
    """
    Verify systemd-homed user configuration.

    Actions:
      status  — dump homectl + loginctl info
      check   — validate storage=subvolume, group membership, linger, snapper
    """
    if not IS_ARCH:
        return create_error_response("NotSupported", "verify_homectl_user requires Arch Linux.")

    result: Dict[str, Any] = {"action": action}

    if not check_command_exists("homectl"):
        return create_error_response("NotSupported", "homectl not installed")

    exit_code, stdout, stderr = await run_command(
        ["homectl", "list", "--json=short"],
        timeout=10,
        check=False,
    )
    if exit_code != 0:
        return create_error_response("CommandError", f"homectl list failed: {stderr}")

    users = []
    try:
        for line in stdout.strip().splitlines():
            if line.strip():
                users.append(json.loads(line))
    except json.JSONDecodeError:
        users = []

    result["user_count"] = len(users)
    result["users"] = users

    if action == "status":
        return result

    elif action == "check":
        issues = []
        checks = {}

        for user in users:
            uname = user.get("userName", "?")
            storage = user.get("storage", "")
            member_of = user.get("memberOf", [])

            checks[f"{uname}.storage"] = storage == "subvolume"
            if not checks[f"{uname}.storage"]:
                issues.append(f"{uname}: storage={storage}, expected subvolume")

            checks[f"{uname}.in_wheel"] = "wheel" in member_of
            if not checks[f"{uname}.in_wheel"]:
                issues.append(f"{uname}: not in wheel group")

            # Check lingering
            exit_code, _, _ = await run_command(
                ["loginctl", "show-user", uname, "-p", "Linger"],
                timeout=5,
                check=False,
            )
            lingering = exit_code == 0
            checks[f"{uname}.linger"] = lingering

            # Check snapper config
            exit_code, _, _ = await run_command(
                ["snapper", "-c", f"user_{uname}", "list"],
                timeout=5,
                check=False,
            )
            checks[f"{uname}.snapper"] = exit_code == 0
            if not checks[f"{uname}.snapper"]:
                issues.append(f"{uname}: no snapper config user_{uname}")

        result["checks"] = checks
        result["issues"] = issues
        result["all_ok"] = len(issues) == 0
        return result

    else:
        return create_error_response("InvalidAction", f"Unknown action: {action}. Use: status, check")


# ─── Tool 4: compare_fstab ───────────────────────────────────────────────────

async def compare_fstab(
    action: str = "check",
) -> Dict[str, Any]:
    """
    Validate /etc/fstab against expected BTRFS subvolume layout.

    Actions:
      check   — parse fstab and verify subvolumes and mount options
      options — detailed mount option check
    """
    fstab_path = Path("/etc/fstab")
    if not fstab_path.exists():
        return create_error_response("NotFound", "/etc/fstab not found")

    raw = fstab_path.read_text()
    entries = []
    for line in raw.strip().splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        parts = line.split()
        if len(parts) >= 4:
            entries.append({"device": parts[0], "mountpoint": parts[1], "fstype": parts[2], "options": parts[3]})

    found_subvols = set()
    issues = []
    options_ok = True

    for entry in entries:
        opts = entry["options"]
        mp = entry["mountpoint"]
        mtype = entry["fstype"]

        # Check for subvol= in options
        subvol_match = re.search(r"subvol=([^,\s]+)", opts)
        if subvol_match:
            found_subvols.add(subvol_match.group(1))

        # ESP /boot check
        if mp == "/boot" and mtype == "vfat":
            if "nofail" not in opts:
                issues.append("/boot (ESP): missing nofail")

        if action == "options" and mtype == "btrfs":
            if mp in ("/var/cache", "/var/tmp", "/var/log", "/swap"):
                if "nodatacow" not in opts:
                    issues.append(f"{mp}: missing nodatacow")
                    options_ok = False
            elif mp in ("/", "/home"):
                if "compress=zstd" not in opts:
                    issues.append(f"{mp}: missing compress=zstd")
                    options_ok = False

    missing_subvols = [s for s in _FSTAB_EXPECTED_SUBVOLS if s not in found_subvols]
    if missing_subvols:
        issues.append(f"Missing subvolumes: {', '.join(missing_subvols)}")

    return {
        "action": action,
        "entry_count": len(entries),
        "found_subvolumes": sorted(found_subvols),
        "expected_subvolumes": _FSTAB_EXPECTED_SUBVOLS,
        "missing_subvolumes": missing_subvols,
        "issues": issues,
        "all_ok": len(issues) == 0,
        "entries": entries,
    }


# ─── Tool 5: compare_packages ────────────────────────────────────────────────

async def compare_packages(
    action: str = "diff",
    build_conf_path: str = "/etc/build.conf",
) -> Dict[str, Any]:
    """
    Compare installed packages against build.conf BUILD_PACKAGES.

    Actions:
      diff    — full diff (missing + extra)
      missing — only missing packages
      extra   — only extra packages
    """
    if not IS_ARCH:
        return create_error_response("NotSupported", "compare_packages requires Arch Linux.")

    build_conf = Path(build_conf_path)
    if not build_conf.exists():
        return create_error_response("NotFound", f"Build config not found: {build_conf_path}")

    conf_text = build_conf.read_text()
    match = re.search(r"BUILD_PACKAGES\s*=\s*\(([^)]*)\)", conf_text, re.DOTALL)
    if not match:
        return create_error_response("ParseError", "BUILD_PACKAGES not found in build.conf")

    expected = []
    for pkg in re.findall(r'"([^"]+)"', match.group(1)):
        expected.append(pkg)

    exit_code, stdout, _ = await run_command(
        ["pacman", "-Qq"],
        timeout=10,
        check=False,
    )
    installed = stdout.strip().splitlines() if exit_code == 0 else []

    installed_set = set(installed)
    expected_set = set(expected)

    missing = sorted(expected_set - installed_set)
    extra = sorted(installed_set - expected_set)

    result = {
        "action": action,
        "expected_count": len(expected),
        "installed_count": len(installed),
        "missing_count": len(missing),
        "extra_count": len(extra),
    }

    if action == "diff":
        result["missing"] = missing
        result["extra"] = extra
    elif action == "missing":
        result["missing"] = missing
    elif action == "extra":
        result["extra"] = extra
    else:
        return create_error_response("InvalidAction", f"Unknown action: {action}. Use: diff, missing, extra")

    result["all_ok"] = len(missing) == 0
    return result


# ─── Tool 6: check_security_posture ──────────────────────────────────────────

async def check_security_posture(
    action: str = "full",
) -> Dict[str, Any]:
    """
    Audit security settings after build.

    Actions:
      full      — all checks
      sshd      — sshd config only
      fail2ban  — fail2ban status
      sudo      — sudo config
      mcp       — MCP server check
    """
    if not IS_ARCH:
        return create_error_response("NotSupported", "check_security_posture requires Arch Linux.")

    def _ssh_audit() -> Dict[str, Any]:
        sshd_cfg = Path("/etc/ssh/sshd_config")
        if not sshd_cfg.exists():
            return {"error": "sshd_config not found"}
        dd = sshd_cfg.read_text()
        return {
            "PermitRootLogin": _grep_val(dd, r"^PermitRootLogin\s+(.+)"),
            "PasswordAuthentication": _grep_val(dd, r"^PasswordAuthentication\s+(.+)"),
            "PermitEmptyPasswords": _grep_val(dd, r"^PermitEmptyPasswords\s+(.+)"),
            "AllowUsers": _grep_val(dd, r"^AllowUsers\s+(.+)"),
        }

    async def _fail2ban_check() -> Dict[str, Any]:
        if not check_command_exists("fail2ban-client"):
            return {"status": "not_installed"}
        exit_code, stdout, _ = await run_command(
            ["fail2ban-client", "status"],
            timeout=10,
            check=False,
        )
        return {"status": "active" if exit_code == 0 else "error", "output": stdout.strip()[:500]}

    async def _sudo_check() -> Dict[str, Any]:
        sudo_d = Path("/etc/sudoers.d/10-wheel")
        if sudo_d.exists():
            content = sudo_d.read_text().strip()
            return {"found": True, "content": content, "wheel_nopasswd": "NOPASSWD" in content}
        return {"found": False}

    async def _mcp_check() -> Dict[str, Any]:
        exit_code, _, _ = await run_command(
            ["systemctl", "is-active", "--quiet", "arch-ops-server"],
            timeout=5,
            check=False,
        )
        api_key = bool(Path("/etc/arch-ops-server/api-key").exists())
        return {"service_active": exit_code == 0, "api_key_set": api_key}

    result: Dict[str, Any] = {"action": action}

    if action in ("full", "sshd"):
        result["sshd"] = _ssh_audit()
    if action in ("full", "fail2ban"):
        result["fail2ban"] = await _fail2ban_check()
    if action in ("full", "sudo"):
        result["sudo"] = await _sudo_check()
    if action in ("full", "mcp"):
        result["mcp"] = await _mcp_check()

    return result


def _grep_val(text: str, pattern: str) -> Optional[str]:
    m = re.search(pattern, text, re.MULTILINE)
    return m.group(1).strip() if m else None


# ─── Tool 7: check_rpi_hardware ──────────────────────────────────────────────

async def check_rpi_hardware(
    action: str = "full",
) -> Dict[str, Any]:
    """
    RPi5-specific hardware checks via vcgencmd.

    Actions:
      full        — all checks
      eeprom      — EEPROM version + BOOT_ORDER
      temperature — temperature + throttling
      frequencies — CPU/GPU clock speeds
      voltage     — core voltage
      memory      — GPU memory split
    """
    if not IS_ARCH:
        return create_error_response("NotSupported", "check_rpi_hardware requires Arch Linux on RPi.")

    vcgen = "vcgencmd"
    if not check_command_exists(vcgen):
        return create_error_response("NotSupported", "vcgencmd not found (not RPi hardware?)")

    async def _vc(cmd: str) -> str:
        exit_code, stdout, _ = await run_command(
            [vcgen, "commands" if cmd == "commands" else cmd],
            timeout=5,
            check=False,
        )
        return stdout.strip() if exit_code == 0 else ""

    result: Dict[str, Any] = {"action": action}

    if action in ("full", "eeprom"):
        exit_code, stdout, _ = await run_command(
            ["rpi-eeprom-config"],
            timeout=5,
            check=False,
        )
        if exit_code == 0:
            for line in stdout.strip().splitlines():
                if "BOOT_ORDER" in line:
                    result["boot_order"] = line.split("=")[-1].strip()
                if "WAKE_ON_GPIO" in line:
                    result["wake_on_gpio"] = line.split("=")[-1].strip()

    if action in ("full", "temperature"):
        temp = await _vc("measure_temp")
        throttle = await _vc("get_throttled")
        result["temperature"] = temp.replace("temp=", "")
        result["throttled_raw"] = throttle.replace("throttled=", "")
        result["throttled"] = throttle != "throttled=0x0"

    if action in ("full", "frequencies"):
        result["cpu_freq"] = (await _vc("measure_clock arm")).replace("frequency(45)=", "")
        result["gpu_freq"] = (await _vc("measure_clock core")).replace("frequency(1)=", "")

    if action in ("full", "voltage"):
        result["core_voltage"] = (await _vc("measure_volts core")).replace("volt=", "")

    if action in ("full", "memory"):
        exit_code, stdout, _ = await run_command(
            ["grep", "gpu_mem", "/boot/config.txt"],
            timeout=5,
            check=False,
        )
        result["gpu_mem"] = stdout.strip() if exit_code == 0 else "not set"

    return result


# ─── Tool 8: benchmark_quick ─────────────────────────────────────────────────

async def benchmark_quick(
    action: str = "full",
) -> Dict[str, Any]:
    """
    Quick performance benchmarks.

    Actions:
      full    — all benchmarks
      disk    — disk read test (hdparm)
      cpu     — openssl speed
      memory  — memory bandwidth (stress-ng)
      network — download speed
    """
    if not IS_ARCH:
        return create_error_response("NotSupported", "benchmark_quick requires Arch Linux.")

    result: Dict[str, Any] = {"action": action}

    if action in ("full", "disk"):
        if check_command_exists("hdparm"):
            exit_code, stdout, _ = await run_command(
                ["hdparm", "-Tt", "/dev/nvme0n1"],
                timeout=30,
                check=False,
            )
            result["disk"] = stdout.strip() if exit_code == 0 else "hdparm failed"
        else:
            result["disk"] = "hdparm not installed"

    if action in ("full", "cpu"):
        if check_command_exists("openssl"):
            exit_code, stdout, _ = await run_command(
                ["openssl", "speed", "-evp", "aes-256-gcm", "-seconds", "3"],
                timeout=15,
                check=False,
            )
            if exit_code == 0:
                lines = stdout.strip().splitlines()
                result["cpu_speed"] = lines[-1] if lines else stdout.strip()[:200]
            else:
                result["cpu_speed"] = "openssl failed"
        else:
            result["cpu_speed"] = "openssl not installed"

    if action in ("full", "memory"):
        if check_command_exists("stress-ng"):
            exit_code, stdout, _ = await run_command(
                ["stress-ng", "--vm", "1", "--vm-bytes", "256M", "--timeout", "5s", "--metrics-brief"],
                timeout=15,
                check=False,
            )
            result["memory"] = stdout.strip().splitlines()[-1] if stdout.strip() else "stress-ng ran"
        else:
            result["memory"] = "stress-ng not installed"

    if action in ("full", "network"):
        exit_code, stdout, _ = await run_command(
            ["curl", "-o", "/dev/null", "-w", "%{speed_download}", "-s", "http://mirror.archlinuxarm.org/"],
            timeout=15,
            check=False,
        )
        if exit_code == 0 and stdout.strip():
            result["network_speed"] = f"{stdout.strip()} bytes/sec"
        else:
            result["network_speed"] = "curl failed"

    return result
