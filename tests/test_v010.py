# SPDX-License-Identifier: GPL-3.0-only OR MIT
"""Tests for v0.10 tools: luks, firewall, hardware, boot_config, backup, recovery, telegram, journal_gateway, snapper extended."""

import pytest
from unittest.mock import patch, AsyncMock, MagicMock


@pytest.mark.asyncio
async def test_manage_luks_status(monkeypatch):
    from arch_ops_server.luks import manage_luks

    monkeypatch.setattr("arch_ops_server.luks.IS_ARCH", True)
    monkeypatch.setattr("arch_ops_server.luks.check_command_exists", lambda x: True)

    async def mock_run(cmd, timeout=5, check=False):
        if cmd[0] == "lsblk":
            return (0, "nvme0n1p2 crypto_LUKS\n", "")
        if cmd[0] == "cryptsetup" and "luksDump" in cmd:
            return (0, "LUKS header information\nVersion: 2\n", "")
        return (1, "", "error")

    monkeypatch.setattr("arch_ops_server.luks.run_command", mock_run)
    result = await manage_luks(action="status")
    assert "luks_device" in result
    assert result["luks_device"] == "/dev/nvme0n1p2"


@pytest.mark.asyncio
async def test_manage_luks_not_arch(monkeypatch):
    from arch_ops_server.luks import manage_luks

    monkeypatch.setattr("arch_ops_server.luks.IS_ARCH", False)
    result = await manage_luks(action="status")
    assert result.get("error") is True


@pytest.mark.asyncio
async def test_manage_luks_is_unlocked(monkeypatch):
    from arch_ops_server.luks import manage_luks

    monkeypatch.setattr("arch_ops_server.luks.IS_ARCH", True)
    monkeypatch.setattr("arch_ops_server.luks.check_command_exists", lambda x: True)

    async def mock_run(cmd, timeout=5, check=False):
        return (0, "crypt\n", "")

    monkeypatch.setattr("arch_ops_server.luks.run_command", mock_run)
    result = await manage_luks(action="is_unlocked")
    assert result["unlocked"] is True


@pytest.mark.asyncio
async def test_manage_luks_add_key_missing_args(monkeypatch):
    from arch_ops_server.luks import manage_luks

    monkeypatch.setattr("arch_ops_server.luks.IS_ARCH", True)
    monkeypatch.setattr("arch_ops_server.luks.check_command_exists", lambda x: True)
    result = await manage_luks(action="add_key")
    assert result.get("error") is True


@pytest.mark.asyncio
async def test_manage_firewall_list_rules(monkeypatch):
    from arch_ops_server.firewall import manage_firewall

    monkeypatch.setattr("arch_ops_server.firewall.IS_ARCH", True)
    monkeypatch.setattr("arch_ops_server.firewall.check_command_exists", lambda x: True)

    async def mock_run(cmd, timeout=5, check=False):
        return (
            0,
            "table inet filter {\n  chain INPUT {\n    type filter hook input priority 0;\n  }\n}",
            "",
        )

    monkeypatch.setattr("arch_ops_server.firewall.run_command", mock_run)
    result = await manage_firewall(action="list_rules")
    assert "ruleset" in result
    assert result["rule_count"] > 0


@pytest.mark.asyncio
async def test_manage_firewall_not_arch(monkeypatch):
    from arch_ops_server.firewall import manage_firewall

    monkeypatch.setattr("arch_ops_server.firewall.IS_ARCH", False)
    result = await manage_firewall(action="list_rules")
    assert result.get("error") is True


@pytest.mark.asyncio
async def test_manage_firewall_validate(monkeypatch):
    from arch_ops_server.firewall import manage_firewall

    monkeypatch.setattr("arch_ops_server.firewall.IS_ARCH", True)
    monkeypatch.setattr("arch_ops_server.firewall.check_command_exists", lambda x: True)

    async def mock_run(cmd, timeout=5, check=False):
        return (0, "", "")

    monkeypatch.setattr("arch_ops_server.firewall.run_command", mock_run)
    result = await manage_firewall(action="validate")
    assert result["valid"] is True


@pytest.mark.asyncio
async def test_manage_firewall_add_port_missing(monkeypatch):
    from arch_ops_server.firewall import manage_firewall

    monkeypatch.setattr("arch_ops_server.firewall.IS_ARCH", True)
    monkeypatch.setattr("arch_ops_server.firewall.check_command_exists", lambda x: True)
    result = await manage_firewall(action="add_port")
    assert result.get("error") is True


@pytest.mark.asyncio
async def test_manage_hardware_health(monkeypatch):
    from arch_ops_server.hardware import manage_hardware

    monkeypatch.setattr("arch_ops_server.hardware.IS_ARCH", True)
    monkeypatch.setattr("arch_ops_server.hardware.check_command_exists", lambda x: True)

    call_count = {"vcgencmd": 0}

    async def mock_run(cmd, timeout=5, check=False):
        if cmd[0] == "vcgencmd":
            call_count["vcgencmd"] += 1
            sub = cmd[1]
            if sub == "measure_temp":
                return (0, "temp=45.0'C\n", "")
            if sub == "get_throttled":
                return (0, "throttled=0x0\n", "")
            if sub == "measure_clock":
                return (0, f"frequency(45)={500000000 + call_count['vcgencmd']}\n", "")
            if sub == "measure_volts":
                return (0, "volt=1.20V\n", "")
        return (1, "", "error")

    monkeypatch.setattr("arch_ops_server.hardware.run_command", mock_run)
    result = await manage_hardware(action="health")
    assert "temperature" in result
    assert "throttled" in result
    assert "frequencies" in result


@pytest.mark.asyncio
async def test_manage_hardware_not_rpi(monkeypatch):
    from arch_ops_server.hardware import manage_hardware

    monkeypatch.setattr("arch_ops_server.hardware.IS_ARCH", True)
    monkeypatch.setattr(
        "arch_ops_server.hardware.check_command_exists", lambda x: False
    )
    result = await manage_hardware(action="health")
    assert result.get("error") is True


@pytest.mark.asyncio
async def test_manage_boot_config_read_config(tmp_path, monkeypatch):
    from arch_ops_server.boot_config import manage_boot_config

    config_content = "gpu_mem=16\ndtparam=audio=on\narm_freq=2400\n"
    config_file = tmp_path / "config.txt"
    config_file.write_text(config_content)

    monkeypatch.setattr("arch_ops_server.boot_config.IS_ARCH", True)
    monkeypatch.setattr("arch_ops_server.boot_config.CONFIG_TXT", str(config_file))

    result = await manage_boot_config(action="read_config")
    assert result["line_count"] == 3
    assert "gpu_mem" in result["overrides"]


@pytest.mark.asyncio
async def test_manage_boot_config_read_cmdline(tmp_path, monkeypatch):
    from arch_ops_server.boot_config import manage_boot_config

    cmdline = (
        "root=/dev/nvme0n1p2 rootflags=subvol=@ cryptdevice=/dev/nvme0n1p2:cryptroot"
    )
    cmdline_file = tmp_path / "cmdline.txt"
    cmdline_file.write_text(cmdline)

    monkeypatch.setattr("arch_ops_server.boot_config.IS_ARCH", True)
    monkeypatch.setattr("arch_ops_server.boot_config.CMDLINE_TXT", str(cmdline_file))

    result = await manage_boot_config(action="read_cmdline")
    assert result["has_encrypt"] is True
    assert result["has_btrfs_subvol"] is True


@pytest.mark.asyncio
async def test_manage_boot_config_verify_boot(tmp_path, monkeypatch):
    from arch_ops_server.boot_config import manage_boot_config
    import os

    boot_dir = tmp_path / "boot"
    boot_dir.mkdir()
    (boot_dir / "config.txt").write_text("")
    (boot_dir / "cmdline.txt").write_text("")
    (boot_dir / "kernel8.img").write_text("dummy")
    (boot_dir / "initramfs-linux.img").write_text("dummy")
    (boot_dir / "bcm2712-rpi-5-b.dtb").write_text("dummy")

    monkeypatch.setattr("arch_ops_server.boot_config.IS_ARCH", True)
    monkeypatch.setattr("arch_ops_server.boot_config.BOOT_DIR", str(boot_dir))

    result = await manage_boot_config(action="verify_boot")
    assert result["all_ok"] is True
    assert len(result["missing"]) == 0


@pytest.mark.asyncio
async def test_manage_backup_status(monkeypatch):
    from arch_ops_server.backup import manage_backup

    monkeypatch.setattr("arch_ops_server.backup.IS_ARCH", True)
    monkeypatch.setattr("arch_ops_server.backup.check_command_exists", lambda x: True)

    import tempfile, os

    conf_dir = tempfile.mkdtemp()
    open(os.path.join(conf_dir, "backup.conf"), "w").close()

    monkeypatch.setattr(
        "os.path.isdir", lambda p: p.startswith("/tmp") or p == "/etc/btrbk"
    )
    monkeypatch.setattr("os.path.isfile", lambda p: p.endswith(".conf"))
    monkeypatch.setattr(
        "os.listdir",
        lambda p: ["backup.conf"] if p.startswith("/tmp") or "btrbk" in p else [],
    )

    result = await manage_backup(action="status")
    assert result["btrbk_installed"] is True


@pytest.mark.asyncio
async def test_manage_backup_not_installed(monkeypatch):
    from arch_ops_server.backup import manage_backup

    monkeypatch.setattr("arch_ops_server.backup.IS_ARCH", True)
    monkeypatch.setattr("arch_ops_server.backup.check_command_exists", lambda x: False)
    result = await manage_backup(action="status")
    assert result.get("error") is True


@pytest.mark.asyncio
async def test_manage_backup_list(monkeypatch):
    from arch_ops_server.backup import manage_backup

    monkeypatch.setattr("arch_ops_server.backup.IS_ARCH", True)
    monkeypatch.setattr("arch_ops_server.backup.check_command_exists", lambda x: True)

    async def mock_run(cmd, timeout=5, check=False):
        return (0, "backup1\nbackup2\nbackup3\n", "")

    monkeypatch.setattr("arch_ops_server.backup.run_command", mock_run)
    result = await manage_backup(action="list")
    assert result["backup_count"] == 3


@pytest.mark.asyncio
async def test_manage_recovery_system_state(monkeypatch):
    from arch_ops_server.recovery import manage_recovery

    monkeypatch.setattr("arch_ops_server.recovery.IS_ARCH", True)

    call_no = [0]

    async def mock_run(cmd, timeout=5, check=False):
        if "is-system-running" in cmd:
            return (0, "running\n", "")
        if "list-units" in cmd:
            return (0, "", "")
        if cmd[0] == "uptime":
            return (0, "09:00:00 up 1 hour\n", "")
        return (1, "", "")
        call_no[0] += 1

    monkeypatch.setattr("arch_ops_server.recovery.run_command", mock_run)
    result = await manage_recovery(action="system_state")
    assert result["all_ok"] is True
    assert result["system_state"] == "running"


@pytest.mark.asyncio
async def test_manage_recovery_check_emergency(monkeypatch):
    from arch_ops_server.recovery import manage_recovery

    monkeypatch.setattr("arch_ops_server.recovery.IS_ARCH", True)

    async def mock_run(cmd, timeout=5, check=False):
        return (0, "running\n", "")

    monkeypatch.setattr("arch_ops_server.recovery.run_command", mock_run)
    result = await manage_recovery(action="check_emergency")
    assert result["emergency_mode"] is False


@pytest.mark.asyncio
async def test_manage_telegram_unlock_status(monkeypatch):
    from arch_ops_server.telegram import manage_telegram_unlock

    monkeypatch.setattr("arch_ops_server.telegram.IS_ARCH", True)

    async def mock_run(cmd, timeout=5, check=False):
        return (0, "telegram-unlock\n/usr/lib/initcpio/hooks/telegram-unlock\n", "")

    monkeypatch.setattr("arch_ops_server.telegram.run_command", mock_run)
    # Mock os.path.isdir and os.listdir
    monkeypatch.setattr("os.path.isdir", lambda p: False)
    monkeypatch.setattr("os.listdir", lambda p: [])

    result = await manage_telegram_unlock(action="status")
    assert result["hook_in_initramfs"] is False


@pytest.mark.asyncio
async def test_manage_telegram_unlock_test_bot_missing(monkeypatch):
    from arch_ops_server.telegram import manage_telegram_unlock

    monkeypatch.setattr("arch_ops_server.telegram.IS_ARCH", True)
    result = await manage_telegram_unlock(action="test_bot")
    assert result.get("error") is True


@pytest.mark.asyncio
async def test_manage_journal_gateway_status(monkeypatch):
    from arch_ops_server.journal import manage_journal_gateway

    monkeypatch.setattr("arch_ops_server.journal.IS_ARCH", True)
    monkeypatch.setattr("arch_ops_server.journal.check_command_exists", lambda x: True)

    async def mock_run(cmd, timeout=5, check=False):
        if "is-active" in cmd:
            return (0, "active\n", "")
        if "status" in cmd:
            return (0, "Active: active (running)\n", "")
        return (1, "", "")

    monkeypatch.setattr("arch_ops_server.journal.run_command", mock_run)
    result = await manage_journal_gateway(action="status")
    assert result["active"] is True


@pytest.mark.asyncio
async def test_manage_journal_gateway_not_available(monkeypatch):
    from arch_ops_server.journal import manage_journal_gateway

    monkeypatch.setattr("arch_ops_server.journal.check_command_exists", lambda x: False)
    result = await manage_journal_gateway(action="status")
    assert result.get("error") is True


@pytest.mark.asyncio
async def test_manage_btrfs_snapshots_diff(monkeypatch):
    from arch_ops_server.btrfs import manage_btrfs_snapshots

    monkeypatch.setattr("arch_ops_server.btrfs.check_command_exists", lambda x: True)
    monkeypatch.setattr("arch_ops_server.btrfs.IS_ARCH", True)

    async def mock_run(cmd, timeout=5, check=False):
        return (0, "+--- /etc/passwd\n-... /etc/hostname\n", "")

    monkeypatch.setattr("arch_ops_server.btrfs.run_command", mock_run)
    result = await manage_btrfs_snapshots(
        action="diff", snap1=1, snap2=5, config="root"
    )
    assert result["change_count"] > 0
    assert result["snap1"] == 1


@pytest.mark.asyncio
async def test_manage_btrfs_snapshots_rollback_dry(monkeypatch):
    from arch_ops_server.btrfs import manage_btrfs_snapshots

    monkeypatch.setattr("arch_ops_server.btrfs.check_command_exists", lambda x: True)
    monkeypatch.setattr("arch_ops_server.btrfs.IS_ARCH", True)

    async def mock_run(cmd, timeout=5, check=False):
        return (0, "+--- /etc/passwd\n-... /etc/hostname\n", "")

    monkeypatch.setattr("arch_ops_server.btrfs.run_command", mock_run)
    result = await manage_btrfs_snapshots(
        action="rollback_dry", snap_num=3, config="root"
    )
    assert result["dry_run"] is True
    assert result["snap_num"] == 3


@pytest.mark.asyncio
async def test_manage_btrfs_snapshots_create_config(monkeypatch):
    from arch_ops_server.btrfs import manage_btrfs_snapshots

    monkeypatch.setattr("arch_ops_server.btrfs.check_command_exists", lambda x: True)
    monkeypatch.setattr("arch_ops_server.btrfs.IS_ARCH", True)

    async def mock_run(cmd, timeout=5, check=False):
        return (0, "", "")

    monkeypatch.setattr("arch_ops_server.btrfs.run_command", mock_run)
    result = await manage_btrfs_snapshots(
        action="create_config", name="home", subvolume="/home"
    )
    assert result["created"] is True
    assert result["name"] == "home"
