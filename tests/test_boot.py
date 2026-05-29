# SPDX-License-Identifier: GPL-3.0-only OR MIT
"""Tests for RPi bootloader management."""

from unittest.mock import patch

import pytest

from arch_ops_server.boot import (
    manage_boot,
    _decode_boot_order,
    _get_eeprom_config,
    _detect_boot_devices,
    _install_restore_service,
    _BOOT_MODE_LABELS,
    _ORDER_PRESETS,
    _STATE_FILE,
)


EEPROM_OUTPUT = """[all]
BOOT_UART=1
POWER_OFF_ON_HALT=0
BOOT_ORDER=0xf416
"""

EEPROM_OTHER_ORDER = """[all]
BOOT_UART=1
BOOT_ORDER=0xf14
"""

LSBLK_OUTPUT = """mmcblk0  disk
mmcblk0p1  part
mmcblk0p2  part
nvme0n1  disk
nvme0n1p1  part
nvme0n1p2  part
"""

LSBLK_NVME_ONLY = """nvme0n1  disk
nvme0n1p1  part
"""


class TestDecodeBootOrder:
    def test_sd_nvme_usb_default(self):
        seq = _decode_boot_order("0xf416")
        assert len(seq) == 4
        assert seq[0]["mode"] == "0x6"
        assert seq[0]["label"] == "NVMe"
        assert seq[1]["mode"] == "0x1"
        assert seq[1]["label"] == "SD"
        assert seq[2]["mode"] == "0x4"
        assert seq[2]["label"] == "USB"
        assert seq[3]["mode"] == "0xf"
        assert seq[3]["label"] == "RESTART"

    def test_nvme_only(self):
        seq = _decode_boot_order("0xf6")
        assert len(seq) == 2
        assert seq[0]["mode"] == "0x6"
        assert seq[0]["label"] == "NVMe"
        assert seq[1]["mode"] == "0xf"
        assert seq[1]["label"] == "RESTART"

    def test_unknown_mode(self):
        seq = _decode_boot_order("0xa")
        assert seq[0]["label"] == "UNKNOWN(0xa)"

    def test_without_prefix(self):
        seq = _decode_boot_order("f416")
        assert seq[0]["mode"] == "0x6"
        assert seq[1]["mode"] == "0x1"


class TestGetEepromConfig:
    @pytest.mark.asyncio
    async def test_success(self):
        async def mock_run(cmd, timeout=10, check=False):
            if "rpi-eeprom-config" in cmd:
                return (0, EEPROM_OUTPUT, "")
            return (1, "", "error")

        with patch("arch_ops_server.boot.run_command", mock_run):
            config = await _get_eeprom_config()
            assert config["BOOT_ORDER"] == "0xf416"
            assert config["BOOT_UART"] == "1"

    @pytest.mark.asyncio
    async def test_command_fails(self):
        async def mock_run(cmd, timeout=10, check=False):
            return (1, "", "not found")

        with patch("arch_ops_server.boot.run_command", mock_run):
            config = await _get_eeprom_config()
            assert config == {}


class TestDetectBootDevices:
    @pytest.mark.asyncio
    async def test_sd_and_nvme(self):
        async def mock_run(cmd, timeout=10, check=False):
            return (0, LSBLK_OUTPUT, "")

        with patch("arch_ops_server.boot.run_command", mock_run):
            devices = await _detect_boot_devices()
            assert devices["sd"] is True
            assert devices["nvme"] is True

    @pytest.mark.asyncio
    async def test_nvme_only(self):
        async def mock_run(cmd, timeout=10, check=False):
            return (0, LSBLK_NVME_ONLY, "")

        with patch("arch_ops_server.boot.run_command", mock_run):
            devices = await _detect_boot_devices()
            assert devices["sd"] is False
            assert devices["nvme"] is True

    @pytest.mark.asyncio
    async def test_command_fails(self):
        async def mock_run(cmd, timeout=10, check=False):
            return (1, "", "error")

        with patch("arch_ops_server.boot.run_command", mock_run):
            devices = await _detect_boot_devices()
            assert devices == {"sd": False, "nvme": False, "usb": False}


class TestManageBootStatus:
    @pytest.mark.asyncio
    async def test_success(self, monkeypatch):
        monkeypatch.setattr("arch_ops_server.boot.IS_ARCH", True)
        monkeypatch.setattr("arch_ops_server.boot.check_command_exists", lambda x: True)

        call_count = 0

        async def mock_run(cmd, timeout=10, check=False):
            nonlocal call_count
            call_count += 1
            if "rpi-eeprom-config" in cmd:
                return (0, EEPROM_OUTPUT, "")
            if "lsblk" in cmd:
                return (0, LSBLK_OUTPUT, "")
            return (1, "", "error")

        with patch("arch_ops_server.boot.run_command", mock_run):
            result = await manage_boot(action="status")
            assert "error" not in result
            assert result["boot_order"] == "0xf416"
            assert len(result["sequence"]) == 4
            assert result["available_devices"]["sd"] is True
            assert result["available_devices"]["nvme"] is True
            assert result["has_pending_restore"] is False

    @pytest.mark.asyncio
    async def test_not_arch(self, monkeypatch):
        monkeypatch.setattr("arch_ops_server.boot.IS_ARCH", False)
        result = await manage_boot(action="status")
        assert result["error"] is True

    @pytest.mark.asyncio
    async def test_no_rpi_eeprom(self, monkeypatch):
        monkeypatch.setattr("arch_ops_server.boot.IS_ARCH", True)
        monkeypatch.setattr("arch_ops_server.boot.check_command_exists", lambda x: False)
        result = await manage_boot(action="status")
        assert result["error"] is True

    @pytest.mark.asyncio
    async def test_eeprom_config_failure(self, monkeypatch):
        monkeypatch.setattr("arch_ops_server.boot.IS_ARCH", True)
        monkeypatch.setattr("arch_ops_server.boot.check_command_exists", lambda x: True)

        async def mock_run(cmd, timeout=10, check=False):
            return (1, "", "error reading EEPROM")

        with patch("arch_ops_server.boot.run_command", mock_run):
            result = await manage_boot(action="status")
            assert result["error"] is True


class TestManageBootSetBootOrder:
    @pytest.mark.asyncio
    async def test_success_with_preset(self, monkeypatch):
        monkeypatch.setattr("arch_ops_server.boot.IS_ARCH", True)
        monkeypatch.setattr("arch_ops_server.boot.check_command_exists", lambda x: True)

        calls = []

        async def mock_run(cmd, timeout=10, check=False):
            calls.append(cmd)
            if "--apply" in cmd:
                return (0, "", "")
            return (1, "", "")

        with patch("arch_ops_server.boot.run_command", mock_run):
            result = await manage_boot(action="set_boot_order", order="nvme_first")
            assert result["set"] is True
            assert result["boot_order"] == "0xf46"

    @pytest.mark.asyncio
    async def test_success_with_raw_hex(self, monkeypatch):
        monkeypatch.setattr("arch_ops_server.boot.IS_ARCH", True)
        monkeypatch.setattr("arch_ops_server.boot.check_command_exists", lambda x: True)

        async def mock_run(cmd, timeout=10, check=False):
            if "--apply" in cmd:
                return (0, "", "")
            return (1, "", "")

        with patch("arch_ops_server.boot.run_command", mock_run):
            result = await manage_boot(action="set_boot_order", order="0xf416")
            assert result["set"] is True
            assert result["boot_order"] == "0xf416"

    @pytest.mark.asyncio
    async def test_missing_order(self, monkeypatch):
        monkeypatch.setattr("arch_ops_server.boot.IS_ARCH", True)
        monkeypatch.setattr("arch_ops_server.boot.check_command_exists", lambda x: True)

        result = await manage_boot(action="set_boot_order")
        assert result["error"] is True

    @pytest.mark.asyncio
    async def test_invalid_order(self, monkeypatch):
        monkeypatch.setattr("arch_ops_server.boot.IS_ARCH", True)
        monkeypatch.setattr("arch_ops_server.boot.check_command_exists", lambda x: True)

        result = await manage_boot(action="set_boot_order", order="bad_value")
        assert result["error"] is True

    @pytest.mark.asyncio
    async def test_eeprom_update_fails(self, monkeypatch):
        monkeypatch.setattr("arch_ops_server.boot.IS_ARCH", True)
        monkeypatch.setattr("arch_ops_server.boot.check_command_exists", lambda x: True)

        async def mock_run(cmd, timeout=10, check=False):
            if "--apply" in cmd:
                return (1, "", "permission denied")
            return (1, "", "")

        with patch("arch_ops_server.boot.run_command", mock_run):
            result = await manage_boot(action="set_boot_order", order="sd_first")
            assert result["error"] is True


class TestManageBootNextBoot:
    @pytest.mark.asyncio
    async def test_success_no_reboot(self, monkeypatch, tmp_path):
        monkeypatch.setattr("arch_ops_server.boot.IS_ARCH", True)
        monkeypatch.setattr("arch_ops_server.boot.check_command_exists", lambda x: True)

        state_dir = tmp_path / "arch-ops-server"
        state_dir.mkdir(parents=True)
        service_dir = tmp_path / "systemd/system"
        service_dir.mkdir(parents=True)
        monkeypatch.setattr("arch_ops_server.boot._STATE_FILE", state_dir / "boot_order_restore")
        monkeypatch.setattr("arch_ops_server.boot._SERVICE_FILE", service_dir / "arch-ops-boot-restore.service")

        calls = []

        async def mock_run(cmd, timeout=10, check=False):
            calls.append(cmd)
            if "rpi-eeprom-config" in cmd and "--apply" not in cmd:
                return (0, EEPROM_OUTPUT, "")
            if "--apply" in cmd:
                return (0, "", "")
            if "systemctl" in cmd and "daemon-reload" in cmd:
                return (0, "", "")
            if "systemctl" in cmd and "enable" in cmd:
                return (0, "", "")
            return (1, "", "")

        with patch("arch_ops_server.boot.run_command", mock_run):
            result = await manage_boot(action="next_boot", device="sd")
            assert result["temp_order_set"] is True
            assert result["temp_order"] == "0xf1"
            assert result["original_order_saved"] == "0xf416"
            assert result["restore_service_installed"] is True
            assert "rebooting" not in result

    @pytest.mark.asyncio
    async def test_missing_device(self, monkeypatch):
        monkeypatch.setattr("arch_ops_server.boot.IS_ARCH", True)
        monkeypatch.setattr("arch_ops_server.boot.check_command_exists", lambda x: True)

        result = await manage_boot(action="next_boot")
        assert result["error"] is True

    @pytest.mark.asyncio
    async def test_invalid_device(self, monkeypatch):
        monkeypatch.setattr("arch_ops_server.boot.IS_ARCH", True)
        monkeypatch.setattr("arch_ops_server.boot.check_command_exists", lambda x: True)

        result = await manage_boot(action="next_boot", device="network")
        assert result["error"] is True

    @pytest.mark.asyncio
    async def test_eeprom_read_fails(self, monkeypatch):
        monkeypatch.setattr("arch_ops_server.boot.IS_ARCH", True)
        monkeypatch.setattr("arch_ops_server.boot.check_command_exists", lambda x: True)

        async def mock_run(cmd, timeout=10, check=False):
            return (1, "", "error")

        with patch("arch_ops_server.boot.run_command", mock_run):
            result = await manage_boot(action="next_boot", device="nvme")
            assert result["error"] is True


class TestManageBootInvalidAction:
    @pytest.mark.asyncio
    async def test_invalid_action(self, monkeypatch):
        monkeypatch.setattr("arch_ops_server.boot.IS_ARCH", True)
        monkeypatch.setattr("arch_ops_server.boot.check_command_exists", lambda x: True)

        result = await manage_boot(action="bogus")
        assert result["error"] is True

    @pytest.mark.asyncio
    async def test_restore_service_file_written(self, monkeypatch, tmp_path):
        monkeypatch.setattr("arch_ops_server.boot.IS_ARCH", True)
        monkeypatch.setattr("arch_ops_server.boot.check_command_exists", lambda x: True)

        state_dir = tmp_path / "arch-ops-server"
        service_dir = tmp_path / "systemd/system"
        monkeypatch.setattr("arch_ops_server.boot._STATE_FILE", state_dir / "boot_order_restore")
        monkeypatch.setattr("arch_ops_server.boot._SERVICE_FILE", service_dir / "arch-ops-boot-restore.service")

        calls = []

        async def mock_run(cmd, timeout=10, check=False):
            calls.append(cmd)
            if "rpi-eeprom-config" in cmd and "--apply" not in cmd:
                return (0, EEPROM_OUTPUT, "")
            if "--apply" in cmd:
                return (0, "", "")
            if "systemctl" in cmd:
                return (0, "", "")
            return (1, "", "")

        with patch("arch_ops_server.boot.run_command", mock_run):
            await manage_boot(action="next_boot", device="nvme")

        assert (state_dir / "boot_order_restore").read_text() == "0xf416"
        svc_content = (service_dir / "arch-ops-boot-restore.service").read_text()
        assert "BOOT_ORDER=0xf416" in svc_content
        assert "arch-ops-boot-restore.service" in svc_content


class TestPresets:
    def test_all_presets_valid_hex(self):
        for name, value in _ORDER_PRESETS.items():
            assert value.startswith("0x"), f"Preset {name} should start with 0x"
            hex_part = value[2:]
            assert all(c in "0123456789abcdef" for c in hex_part), f"Preset {name} should be valid hex"

    def test_sd_first_means_sd_then(self):
        seq = _decode_boot_order(_ORDER_PRESETS["sd_first"])
        assert seq[0]["mode"] == "0x1"
        assert seq[0]["label"] == "SD"

    def test_nvme_first_means_nvme_then(self):
        seq = _decode_boot_order(_ORDER_PRESETS["nvme_first"])
        assert seq[0]["mode"] == "0x6"
        assert seq[0]["label"] == "NVMe"
