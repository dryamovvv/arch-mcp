# SPDX-License-Identifier: GPL-3.0-only OR MIT
"""Tests for BTRFS filesystem monitoring and management."""

from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from arch_ops_server.btrfs import (
    analyze_btrfs,
    manage_btrfs_snapshots,
    manage_btrfs_scrub,
    get_filesystem_info,
    get_filesystem_df,
    get_filesystem_usage,
    list_subvolumes,
    get_subvolume_info,
    get_device_stats,
    get_device_usage,
    get_properties,
    get_scrub_status,
    start_scrub,
    cancel_scrub,
    list_snapshots,
    get_snapper_configs,
    create_snapshot,
    delete_snapshot,
)


BTRFS_SHOW_OUTPUT = """Label: 'archlinux'  uuid: 1d56d663-330d-460e-a902-8e7487a99f57
\tTotal devices 1 FS bytes used 18.28GiB
\tdevid    1 size 931.01GiB used 19.04GiB path /dev/nvme0n1p2
"""

BTRFS_DF_OUTPUT = """Data, single: total=18.50GiB, used=18.00GiB
System, DUP: total=8.00MiB, used=16.00KiB
Metadata, DUP: total=256.00MiB, used=96.00MiB
GlobalReserve, single: total=64.00MiB, used=0.00B
"""

BTRFS_USAGE_OUTPUT = """Overall:
    Device size:\t\t 931.01GiB
    Device allocated:\t\t  19.04GiB
    Device unallocated:\t\t 911.97GiB
    Device missing:\t\t     0.00B
    Used:\t\t\t  18.10GiB
    Free (estimated):\t\t 910.04GiB
    Data ratio:\t\t\t      1.00
    Metadata ratio:\t\t      2.00
    Global reserve:\t\t  64.00MiB

Data,single: Size:18.50GiB, Used:18.00GiB (97.30%)
   /dev/nvme0n1p2\t  18.50GiB

Metadata,DUP: Size:256.00MiB, Used:96.00MiB (37.50%)
   /dev/nvme0n1p2\t 512.00MiB

System,DUP: Size:8.00MiB, Used:16.00KiB (0.20%)
   /dev/nvme0n1p2\t  16.00MiB

Unallocated:
   /dev/nvme0n1p2\t 911.97GiB
"""

BTRFS_SUBVOL_LIST = """ID\tgen\ttop level\tpath
--\t---\t---------\t----
257\t22\t5\t@home
258\t571\t5\t@snapshots
259\t587\t5\t@var_log
"""

BTRFS_SUBVOL_SHOW = """@home
\tName: \t\t\t@home
\tUUID: \t\t\t12345678
\tParent UUID: \t\t-
\tCreation time: \t\t2024-01-01 00:00:00
\tSubvolume ID: \t\t257
\tGeneration: \t\t22
"""

BTRFS_DEVICE_STATS = """[/dev/nvme0n1p2].write_io_errs    0
[/dev/nvme0n1p2].read_io_errs     0
[/dev/nvme0n1p2].corruption_errs  0
"""

BTRFS_DEVICE_USAGE = """/dev/nvme0n1p2, ID: 1
   Device size:           931.01GiB
   Device slack:              0.00B
   Data,single:            18.50GiB
   Unallocated:           911.97GiB
"""

BTRFS_PROPERTIES = """label=archlinux
ro=false
"""

BTRFS_SCRUB_STATUS = """UUID:             1d56d663-330d-460e-a902-8e7487a99f57
Scrub started:    Fri May 29 00:04:31 2026
Status:           finished
Duration:         0:00:03
Total to scrub:   2.38GiB
Rate:             811.13MiB/s
Error summary:    no errors found
"""

BTRFS_SCRUB_START = "scrub done for 1d56d663-330d-460e-a902-8e7487a99f57\n"

SNAPPER_LIST = """ # │ Type   │ Pre # │ Date                     │ User │ Cleanup  │ Description
---+--------+-------+--------------------------+------+----------+-------------
 0 │ single │       │                          │ root │          │ current
 1 │ single │       │ Thu May 28 06:10:46 2026 │ root │          │ test
 3 │ pre    │       │ Thu May 28 12:13:54 2026 │ root │ number   │ update
"""

SNAPPER_LIST_CONFIGS = """Config │ Subvolume
-------+---------
root   │ /
home   │ /home
"""

SNAPPER_GET_CONFIG = """ALLOW_GROUPS  │
ALLOW_USERS   │
TIMELINE_CREATE │ yes
TIMELINE_CLEANUP │ yes
"""


# ============================================================================
# Helper to create mock run_command
# ============================================================================

def make_run_command(stdout: str = "", stderr: str = "", exit_code: int = 0):
    async def mock_run(cmd, timeout=10, check=False):
        return (exit_code, stdout, stderr)
    return mock_run


# ============================================================================
# Test filesystem_info
# ============================================================================

class TestFilesystemInfo:
    @pytest.mark.asyncio
    async def test_success(self):
        async def mock_run(cmd, timeout=10, check=False):
            if "btrfs" in cmd:
                return (0, BTRFS_SHOW_OUTPUT, "")
            elif "stat" in cmd:
                return (0, "btrfs\n", "")
            return (1, "", "error")

        with patch("arch_ops_server.btrfs.run_command", mock_run), \
             patch("arch_ops_server.btrfs.check_command_exists", return_value=True):
            result = await get_filesystem_info()
            assert result["label"] == "archlinux"
            assert result["uuid"] == "1d56d663-330d-460e-a902-8e7487a99f57"
            assert result["total_devices"] == 1
            assert result["devices"][0]["path"] == "/dev/nvme0n1p2"
            assert result["devices"][0]["size"] == "931.01GiB"

    @pytest.mark.asyncio
    async def test_not_btrfs(self):
        async def mock_run(cmd, timeout=10, check=False):
            if "stat" in cmd:
                return (0, "ext4\n", "")
            return (1, "", "error")

        with patch("arch_ops_server.btrfs.run_command", mock_run), \
             patch("arch_ops_server.btrfs.check_command_exists", return_value=True):
            result = await get_filesystem_info()
            assert result["error"] is True


# ============================================================================
# Test filesystem_df
# ============================================================================

class TestFilesystemDf:
    @pytest.mark.asyncio
    async def test_success(self):
        async def mock_run(cmd, timeout=10, check=False):
            if "btrfs" in cmd and "df" in cmd:
                return (0, BTRFS_DF_OUTPUT, "")
            elif "stat" in cmd:
                return (0, "btrfs\n", "")
            return (1, "", "error")

        with patch("arch_ops_server.btrfs.run_command", mock_run), \
             patch("arch_ops_server.btrfs.check_command_exists", return_value=True):
            result = await get_filesystem_df()
            assert "profiles" in result
            assert "Data, single" in result["profiles"]
            assert "Metadata, DUP" in result["profiles"]


# ============================================================================
# Test filesystem_usage
# ============================================================================

class TestFilesystemUsage:
    @pytest.mark.asyncio
    async def test_success(self):
        async def mock_run(cmd, timeout=10, check=False):
            if "btrfs" in cmd and "usage" in cmd:
                return (0, BTRFS_USAGE_OUTPUT, "")
            elif "stat" in cmd:
                return (0, "btrfs\n", "")
            return (1, "", "error")

        with patch("arch_ops_server.btrfs.run_command", mock_run), \
             patch("arch_ops_server.btrfs.check_command_exists", return_value=True):
            result = await get_filesystem_usage()
            assert "overall" in result
            assert "unallocated" in result


# ============================================================================
# Test subvolumes
# ============================================================================

class TestSubvolumes:
    @pytest.mark.asyncio
    async def test_success(self):
        async def mock_run(cmd, timeout=10, check=False):
            if "btrfs" in cmd and "subvolume" in cmd and "list" in cmd:
                return (0, BTRFS_SUBVOL_LIST, "")
            elif "stat" in cmd:
                return (0, "btrfs\n", "")
            return (1, "", "error")

        with patch("arch_ops_server.btrfs.run_command", mock_run), \
             patch("arch_ops_server.btrfs.check_command_exists", return_value=True):
            result = await list_subvolumes()
            assert result["subvolume_count"] == 3

    @pytest.mark.asyncio
    async def test_subvolume_info_success(self):
        async def mock_run(cmd, timeout=10, check=False):
            if "btrfs" in cmd and "subvolume" in cmd and "show" in cmd:
                return (0, BTRFS_SUBVOL_SHOW, "")
            elif "stat" in cmd:
                return (0, "btrfs\n", "")
            return (1, "", "error")

        with patch("arch_ops_server.btrfs.run_command", mock_run), \
             patch("arch_ops_server.btrfs.check_command_exists", return_value=True):
            from arch_ops_server.btrfs import get_subvolume_info
            result = await get_subvolume_info()
            assert "info" in result


# ============================================================================
# Test device stats
# ============================================================================

class TestDeviceStats:
    @pytest.mark.asyncio
    async def test_success(self):
        async def mock_run(cmd, timeout=10, check=False):
            if "btrfs" in cmd and "device" in cmd and "stats" in cmd:
                return (0, BTRFS_DEVICE_STATS, "")
            elif "stat" in cmd:
                return (0, "btrfs\n", "")
            return (1, "", "error")

        with patch("arch_ops_server.btrfs.run_command", mock_run), \
             patch("arch_ops_server.btrfs.check_command_exists", return_value=True):
            result = await get_device_stats()
            assert "devices" in result
            assert "/dev/nvme0n1p2" in result["devices"]
            assert result["has_errors"] == False

    @pytest.mark.asyncio
    async def test_device_usage_success(self):
        async def mock_run(cmd, timeout=10, check=False):
            if "btrfs" in cmd and "device" in cmd and "usage" in cmd:
                return (0, BTRFS_DEVICE_USAGE, "")
            elif "stat" in cmd:
                return (0, "btrfs\n", "")
            return (1, "", "error")

        with patch("arch_ops_server.btrfs.run_command", mock_run), \
             patch("arch_ops_server.btrfs.check_command_exists", return_value=True):
            result = await get_device_usage()
            assert "devices" in result


# ============================================================================
# Test properties
# ============================================================================

class TestProperties:
    @pytest.mark.asyncio
    async def test_success(self):
        async def mock_run(cmd, timeout=10, check=False):
            if "btrfs" in cmd and "property" in cmd:
                return (0, BTRFS_PROPERTIES, "")
            elif "stat" in cmd:
                return (0, "btrfs\n", "")
            return (1, "", "error")

        with patch("arch_ops_server.btrfs.run_command", mock_run), \
             patch("arch_ops_server.btrfs.check_command_exists", return_value=True):
            result = await get_properties()
            assert "properties" in result
            assert result["properties"]["label"] == "archlinux"


# ============================================================================
# Test scrub
# ============================================================================

class TestScrub:
    @pytest.mark.asyncio
    async def test_status_success(self):
        async def mock_run(cmd, timeout=10, check=False):
            if "btrfs" in cmd and "scrub" in cmd and "status" in cmd:
                return (0, BTRFS_SCRUB_STATUS, "")
            elif "stat" in cmd:
                return (0, "btrfs\n", "")
            return (1, "", "error")

        with patch("arch_ops_server.btrfs.run_command", mock_run), \
             patch("arch_ops_server.btrfs.check_command_exists", return_value=True):
            result = await get_scrub_status()
            assert "scrub" in result

    @pytest.mark.asyncio
    async def test_start_success(self):
        async def mock_run(cmd, timeout=10, check=False):
            if "btrfs" in cmd and "scrub" in cmd and "start" in cmd:
                return (0, BTRFS_SCRUB_START, "")
            elif "stat" in cmd:
                return (0, "btrfs\n", "")
            return (1, "", "error")

        with patch("arch_ops_server.btrfs.run_command", mock_run), \
             patch("arch_ops_server.btrfs.check_command_exists", return_value=True):
            result = await start_scrub()
            assert result["started"] == True

    @pytest.mark.asyncio
    async def test_start_failure(self):
        async def mock_run(cmd, timeout=10, check=False):
            if "btrfs" in cmd and "scrub" in cmd and "start" in cmd:
                return (1, "", "scrub error")
            elif "stat" in cmd:
                return (0, "btrfs\n", "")
            return (1, "", "error")

        with patch("arch_ops_server.btrfs.run_command", mock_run), \
             patch("arch_ops_server.btrfs.check_command_exists", return_value=True):
            result = await start_scrub()
            assert "error" in result

    @pytest.mark.asyncio
    async def test_cancel_success(self):
        async def mock_run(cmd, timeout=10, check=False):
            if "btrfs" in cmd and "scrub" in cmd and "cancel" in cmd:
                return (0, "", "")
            elif "stat" in cmd:
                return (0, "btrfs\n", "")
            return (1, "", "error")

        with patch("arch_ops_server.btrfs.run_command", mock_run), \
             patch("arch_ops_server.btrfs.check_command_exists", return_value=True):
            result = await cancel_scrub()
            assert result["cancelled"] == True


# ============================================================================
# Test snapshots
# ============================================================================

class TestSnapshots:
    @pytest.mark.asyncio
    async def test_list_success(self):
        async def mock_run(cmd, timeout=10, check=False):
            if "snapper" in cmd and "list" in cmd and "list-configs" not in " ".join(cmd):
                return (0, SNAPPER_LIST, "")
            return (1, "", "error")

        with patch("arch_ops_server.btrfs.run_command", mock_run), \
             patch("arch_ops_server.btrfs.check_command_exists", return_value=True):
            result = await list_snapshots()
            assert result["snapshot_count"] == 3
            assert result["snapshots"][0]["description"] == "current"

    @pytest.mark.asyncio
    async def test_snapper_not_installed(self):
        with patch("arch_ops_server.btrfs.check_command_exists", return_value=False):
            result = await list_snapshots()
            assert "error" in result

    @pytest.mark.asyncio
    async def test_configs_success(self):
        async def mock_run(cmd, timeout=10, check=False):
            if "snapper" in cmd and "list-configs" in " ".join(cmd):
                return (0, SNAPPER_LIST_CONFIGS, "")
            elif "snapper" in cmd and "get-config" in " ".join(cmd):
                return (0, SNAPPER_GET_CONFIG, "")
            return (1, "", "error")

        with patch("arch_ops_server.btrfs.run_command", mock_run), \
             patch("arch_ops_server.btrfs.check_command_exists", return_value=True):
            result = await get_snapper_configs()
            assert result["config_count"] == 2

    @pytest.mark.asyncio
    async def test_create_success(self):
        async def mock_run(cmd, timeout=10, check=False):
            if "snapper" in cmd and "create" in cmd:
                return (0, "45\n", "")
            return (1, "", "error")

        with patch("arch_ops_server.btrfs.run_command", mock_run), \
             patch("arch_ops_server.btrfs.check_command_exists", return_value=True):
            result = await create_snapshot("test-snap")
            assert result["created"] == True
            assert result["snapshot_id"] == "45"

    @pytest.mark.asyncio
    async def test_delete_success(self):
        async def mock_run(cmd, timeout=10, check=False):
            if "snapper" in cmd and "delete" in cmd:
                return (0, "", "")
            return (1, "", "error")

        with patch("arch_ops_server.btrfs.run_command", mock_run), \
             patch("arch_ops_server.btrfs.check_command_exists", return_value=True):
            result = await delete_snapshot(45)
            assert result["deleted"] == True
            assert result["snapshot_id"] == 45


# ============================================================================
# Test unified action dispatchers
# ============================================================================

class TestUnifiedAnalyzeBtrfs:
    @pytest.mark.asyncio
    async def test_invalid_action(self):
        result = await analyze_btrfs(action="invalid")
        assert "error" in result

    @pytest.mark.asyncio
    async def test_filesystem_info_action(self):
        async def mock_run(cmd, timeout=10, check=False):
            if "btrfs" in cmd and "show" in cmd:
                return (0, BTRFS_SHOW_OUTPUT, "")
            elif "stat" in cmd:
                return (0, "btrfs\n", "")
            return (1, "", "error")

        with patch("arch_ops_server.btrfs.run_command", mock_run), \
             patch("arch_ops_server.btrfs.check_command_exists", return_value=True):
            result = await analyze_btrfs(action="filesystem_info")
            assert "label" in result
            assert result["label"] == "archlinux"


class TestUnifiedManageSnapshots:
    @pytest.mark.asyncio
    async def test_invalid_action(self):
        result = await manage_btrfs_snapshots(action="invalid")
        assert "error" in result

    @pytest.mark.asyncio
    async def test_create_missing_description(self):
        result = await manage_btrfs_snapshots(action="create")
        assert "error" in result

    @pytest.mark.asyncio
    async def test_delete_missing_id(self):
        result = await manage_btrfs_snapshots(action="delete")
        assert "error" in result

    @pytest.mark.asyncio
    async def test_list_action(self):
        async def mock_run(cmd, timeout=10, check=False):
            if "snapper" in cmd and "list" in cmd:
                return (0, SNAPPER_LIST, "")
            return (1, "", "error")

        with patch("arch_ops_server.btrfs.run_command", mock_run), \
             patch("arch_ops_server.btrfs.check_command_exists", return_value=True):
            result = await manage_btrfs_snapshots(action="list")
            assert "snapshots" in result


class TestUnifiedManageScrub:
    @pytest.mark.asyncio
    async def test_invalid_action(self):
        result = await manage_btrfs_scrub(action="invalid")
        assert "error" in result

    @pytest.mark.asyncio
    async def test_status_action(self):
        async def mock_run(cmd, timeout=10, check=False):
            if "btrfs" in cmd and "scrub" in cmd and "status" in cmd:
                return (0, BTRFS_SCRUB_STATUS, "")
            elif "stat" in cmd:
                return (0, "btrfs\n", "")
            return (1, "", "error")

        with patch("arch_ops_server.btrfs.run_command", mock_run), \
             patch("arch_ops_server.btrfs.check_command_exists", return_value=True):
            result = await manage_btrfs_scrub(action="status")
            assert "scrub" in result

    @pytest.mark.asyncio
    async def test_start_action(self):
        async def mock_run(cmd, timeout=10, check=False):
            if "btrfs" in cmd and "scrub" in cmd and "start" in cmd:
                return (0, BTRFS_SCRUB_START, "")
            elif "stat" in cmd:
                return (0, "btrfs\n", "")
            return (1, "", "error")

        with patch("arch_ops_server.btrfs.run_command", mock_run), \
             patch("arch_ops_server.btrfs.check_command_exists", return_value=True):
            result = await manage_btrfs_scrub(action="start")
            assert result["started"] == True

    @pytest.mark.asyncio
    async def test_cancel_action(self):
        async def mock_run(cmd, timeout=10, check=False):
            if "btrfs" in cmd and "scrub" in cmd and "cancel" in cmd:
                return (0, "", "")
            elif "stat" in cmd:
                return (0, "btrfs\n", "")
            return (1, "", "error")

        with patch("arch_ops_server.btrfs.run_command", mock_run), \
             patch("arch_ops_server.btrfs.check_command_exists", return_value=True):
            result = await manage_btrfs_scrub(action="cancel")
            assert result["cancelled"] == True


# ============================================================================
# Test btrfs-progs not installed
# ============================================================================

class TestBtrfsNotInstalled:
    @pytest.mark.asyncio
    async def test_filesystem_info_no_btrfs(self):
        with patch("arch_ops_server.btrfs.check_command_exists", return_value=False):
            result = await get_filesystem_info()
            assert "error" in result
            assert "btrfs-progs" in result.get("message", "")

    @pytest.mark.asyncio
    async def test_scrub_no_btrfs(self):
        with patch("arch_ops_server.btrfs.check_command_exists", return_value=False):
            result = await get_scrub_status()
            assert "error" in result
