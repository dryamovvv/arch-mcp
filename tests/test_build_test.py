# SPDX-License-Identifier: GPL-3.0-only OR MIT
"""
Tests for OS build testing tools.
"""

import pytest
from unittest.mock import AsyncMock, MagicMock, patch
from arch_ops_server.build_test import (
    verify_boot_artifacts,
    verify_service_health,
    verify_homectl_user,
    compare_fstab,
    compare_packages,
    check_security_posture,
    check_rpi_hardware,
    benchmark_quick,
    _grep_val,
)


# ─── verify_boot_artifacts ───────────────────────────────────────────────────

@pytest.mark.asyncio
async def test_verify_boot_artifacts_check(tmp_path, monkeypatch):
    """Test checking critical boot files."""
    boot = tmp_path / "boot"
    boot.mkdir()
    (boot / "kernel8.img").touch()
    (boot / "config.txt").touch()
    (boot / "cmdline.txt").touch()

    monkeypatch.setattr("arch_ops_server.build_test._BOOT_PATH", boot)
    monkeypatch.setattr("arch_ops_server.build_test.IS_ARCH", True)

    result = await verify_boot_artifacts(action="check")
    assert result["all_ok"] is False  # missing initramfs, dtb
    assert "initramfs-linux.img" in result["missing"]


@pytest.mark.asyncio
async def test_verify_boot_artifacts_cmdline(tmp_path, monkeypatch):
    """Test cmdline.txt parsing."""
    boot = tmp_path / "boot"
    boot.mkdir()
    (boot / "cmdline.txt").write_text("root=UUID=abc-123 rootflags=subvol=@ quiet")

    monkeypatch.setattr("arch_ops_server.build_test._BOOT_PATH", boot)
    monkeypatch.setattr("arch_ops_server.build_test.IS_ARCH", True)

    result = await verify_boot_artifacts(action="cmdline")
    assert result["has_root_uuid"] is True
    assert result["has_subvol"] is True
    assert result["all_ok"] is True


@pytest.mark.asyncio
async def test_verify_boot_artifacts_cmdline_placeholder(tmp_path, monkeypatch):
    """Test cmdline.txt with unresolved placeholder."""
    boot = tmp_path / "boot"
    boot.mkdir()
    (boot / "cmdline.txt").write_text("root=UUID=__ROOT_UUID__")

    monkeypatch.setattr("arch_ops_server.build_test._BOOT_PATH", boot)
    monkeypatch.setattr("arch_ops_server.build_test.IS_ARCH", True)

    result = await verify_boot_artifacts(action="cmdline")
    assert result["has_placeholder"] is True
    assert result["all_ok"] is False


@pytest.mark.asyncio
async def test_verify_boot_artifacts_not_arch(monkeypatch):
    """Test error on non-Arch system."""
    monkeypatch.setattr("arch_ops_server.build_test.IS_ARCH", False)
    result = await verify_boot_artifacts(action="check")
    assert result.get("error") is True


# ─── verify_service_health ───────────────────────────────────────────────────

@pytest.mark.asyncio
async def test_verify_service_health_checklist(monkeypatch):
    """Test checklist mode with mocked systemctl."""
    monkeypatch.setattr("arch_ops_server.build_test.IS_ARCH", True)

    async def mock_run(cmd, timeout=5, check=False):
        if cmd[0] == "systemctl" and "is-active" in cmd:
            svc = cmd[-1]
            if svc == "sshd.service":
                return (0, "", "")
            return (3, "", "")
        return (0, "", "")

    monkeypatch.setattr("arch_ops_server.build_test.run_command", mock_run)

    result = await verify_service_health(action="checklist")
    assert result["results"]["sshd.service"] == "active"
    assert result["results"]["systemd-networkd.service"] == "inactive"
    assert result["all_ok"] is False


# ─── verify_homectl_user ─────────────────────────────────────────────────────

@pytest.mark.asyncio
async def test_verify_homectl_user_not_available(monkeypatch):
    """Test with homectl not installed."""
    monkeypatch.setattr("arch_ops_server.build_test.IS_ARCH", True)
    monkeypatch.setattr("arch_ops_server.build_test.check_command_exists", lambda x: False)
    result = await verify_homectl_user(action="check")
    assert result.get("error") is True


# ─── compare_fstab ───────────────────────────────────────────────────────────

@pytest.mark.asyncio
async def test_compare_fstab_full(tmp_path, monkeypatch):
    """Test fstab with correct BTRFS subvolume layout."""
    fstab_content = """
UUID=abc /          btrfs subvol=@,compress=zstd 0 1
UUID=abc /home      btrfs subvol=@home,compress=zstd 0 2
UUID=abc /.snapshots btrfs subvol=@snapshots 0 0
UUID=abc /swap      btrfs subvol=@swap,nodatacow 0 0
UUID=abc /var/log   btrfs subvol=@var_log,nodatacow 0 0
UUID=abc /var/cache btrfs subvol=@var_cache,nodatacow 0 0
UUID=abc /var/tmp   btrfs subvol=@var_tmp,nodatacow 0 0
UUID=abc /var/lib   btrfs subvol=@var_lib 0 0
UUID=def /boot      vfat defaults,nofail 0 2
"""
    fstab = tmp_path / "fstab"
    fstab.write_text(fstab_content.strip())
    monkeypatch.setattr("arch_ops_server.build_test.Path", lambda x: fstab if str(x) == "/etc/fstab" else type(fstab)(x))
    result = await compare_fstab(action="check")
    assert result["all_ok"] is True
    assert len(result["found_subvolumes"]) >= 8


@pytest.mark.asyncio
async def test_compare_fstab_missing_subvols(tmp_path, monkeypatch):
    """Test fstab with missing subvolumes."""
    fstab_content = """
UUID=abc / btrfs subvol=@ 0 1
"""
    fstab = tmp_path / "fstab"
    fstab.write_text(fstab_content.strip())

    result = await compare_fstab(action="check")
    assert result["all_ok"] is False
    assert len(result["missing_subvolumes"]) > 0


# ─── compare_packages ────────────────────────────────────────────────────────

@pytest.mark.asyncio
async def test_compare_packages(monkeypatch):
    """Test package comparison with mocked pacman."""
    monkeypatch.setattr("arch_ops_server.build_test.IS_ARCH", True)

    async def mock_run(cmd, timeout=5, check=False):
        if cmd[0] == "pacman":
            return (0, "base\nlinux\nvim\n", "")
        return (0, "", "")

    monkeypatch.setattr("arch_ops_server.build_test.run_command", mock_run)

    result = await compare_packages(action="diff", build_conf_path="/nonexistent")
    assert result.get("error") is True  # build.conf not found


# ─── check_security_posture ──────────────────────────────────────────────────

def test_grep_val():
    """Test _grep_val utility."""
    text = "PermitRootLogin no\nPasswordAuthentication yes"
    assert _grep_val(text, r"PermitRootLogin\s+(.+)") == "no"
    assert _grep_val(text, r"PasswordAuthentication\s+(.+)") == "yes"


@pytest.mark.asyncio
async def test_check_security_posture_sshd(tmp_path, monkeypatch):
    """Test sshd config audit with tmp file."""
    sshd = tmp_path / "sshd_config"
    sshd.write_text("PermitRootLogin no\nPasswordAuthentication no\nAllowUsers admin\n")

    monkeypatch.setattr("arch_ops_server.build_test.IS_ARCH", True)

    # Patch Path to return our tmp sshd_config
    def _mock_path(p):
        if str(p) == "/etc/ssh/sshd_config":
            return sshd
        from pathlib import Path
        return Path(p)
    monkeypatch.setattr("arch_ops_server.build_test.Path", _mock_path)

    async def mock_run(cmd, timeout=5, check=False):
        return (0, "", "")

    monkeypatch.setattr("arch_ops_server.build_test.run_command", mock_run)

    result = await check_security_posture(action="sshd")
    assert result["sshd"]["PermitRootLogin"] == "no"


# ─── check_rpi_hardware ──────────────────────────────────────────────────────

@pytest.mark.asyncio
async def test_check_rpi_hardware_no_vcgencmd(monkeypatch):
    """Test error when vcgencmd missing."""
    monkeypatch.setattr("arch_ops_server.build_test.IS_ARCH", True)
    monkeypatch.setattr("arch_ops_server.build_test.check_command_exists", lambda x: False)
    result = await check_rpi_hardware(action="temperature")
    assert result.get("error") is True


# ─── benchmark_quick ─────────────────────────────────────────────────────────

@pytest.mark.asyncio
async def test_benchmark_quick_disk_not_available(monkeypatch):
    """Test benchmark with missing hdparm."""
    monkeypatch.setattr("arch_ops_server.build_test.IS_ARCH", True)
    monkeypatch.setattr("arch_ops_server.build_test.check_command_exists", lambda x: False)
    result = await benchmark_quick(action="disk")
    assert "not installed" in result["disk"]
