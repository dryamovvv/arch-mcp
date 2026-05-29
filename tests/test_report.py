"""Tests for report generation module."""

import pytest

from arch_ops_server.report import (
    _build_table,
    _fmt_row,
    _parse_btrfs_device_stats,
    _parse_btrfs_info,
    _parse_btrfs_scrub,
    _parse_boot_status,
    _parse_db_freshness,
    _parse_disk_usage,
    _parse_failed_services,
    _parse_mirror_health,
    _parse_orphans,
    _parse_system_info,
    _parse_updates,
    _status_icon,
)


def test_status_icon():
    assert _status_icon(True) == "\u2705"
    assert _status_icon(False) == "\u274c"


def test_fmt_row():
    row = _fmt_row("test", "meaning", "result")
    assert row == ("test", "meaning", "result")


def test_build_table_empty():
    table = _build_table([])
    assert "No data" in table


def test_build_table_with_rows():
    rows = [("prog", "mean", "ok")]
    table = _build_table(rows)
    assert "| prog | mean | ok |" in table
    assert "| Программа" in table


def test_parse_system_info_ok():
    data = {
        "kernel": "6.18",
        "architecture": "aarch64",
        "hostname": "rpi5",
        "uptime": "3 days",
        "memory": {"total": "16G", "used": "2G"},
    }
    rows = _parse_system_info(data)
    assert any("6.18" in r[2] for r in rows)
    assert any("aarch64" in r[2] for r in rows)


def test_parse_system_info_error():
    rows = _parse_system_info({"error_type": "TestError", "message": "boom"})
    assert "Error" in rows[0][2]


def test_parse_disk_usage_ok():
    data = {
        "filesystems": [
            {"filesystem": "/", "used_percent": 45, "available": "500G"},
        ]
    }
    rows = _parse_disk_usage(data)
    assert any("45%" in r[2] for r in rows)


def test_parse_disk_usage_empty():
    rows = _parse_disk_usage({"filesystems": []})
    assert "No filesystems" in rows[0][2]


def test_parse_disk_usage_error():
    rows = _parse_disk_usage({"error_type": "Oops"})
    assert "Error" in rows[0][2]


def test_parse_failed_services_ok():
    rows = _parse_failed_services({"failed_count": 0})
    assert "\u2705" in rows[0][2]


def test_parse_failed_services_bad():
    rows = _parse_failed_services({"failed_count": 3})
    assert "\u274c" in rows[0][2]


def test_parse_failed_services_error():
    rows = _parse_failed_services({"error_type": "x"})
    assert "Error" in rows[0][2]


def test_parse_updates_ok():
    rows = _parse_updates({"update_count": 0})
    assert "\u2705" in rows[0][2]


def test_parse_updates_pending():
    rows = _parse_updates({"update_count": 5})
    assert "\u274c" in rows[0][2]
    assert "5" in rows[0][2]


def test_parse_updates_error():
    rows = _parse_updates({"error_type": "x"})
    assert "Error" in rows[0][2]


def test_parse_orphans_ok():
    rows = _parse_orphans({"orphan_count": 0})
    assert "\u2705" in rows[0][2]


def test_parse_orphans_found():
    rows = _parse_orphans({"orphan_count": 2})
    assert "2" in rows[0][2]


def test_parse_orphans_error():
    rows = _parse_orphans({"error_type": "x"})
    assert "Error" in rows[0][2]


def test_parse_db_freshness_mixed():
    data = {
        "databases": {
            "core": {"hours_since_sync": 2, "status": "fresh"},
            "extra": {"hours_since_sync": 48, "status": "stale"},
        }
    }
    rows = _parse_db_freshness(data)
    assert len(rows) == 2
    assert "2h" in rows[0][2]
    assert "48h" in rows[1][2]


def test_parse_db_freshness_empty():
    rows = _parse_db_freshness({"databases": {}})
    assert "No databases" in rows[0][2]


def test_parse_db_freshness_error():
    rows = _parse_db_freshness({"error_type": "x"})
    assert "Error" in rows[0][2]


def test_parse_mirror_health_good():
    rows = _parse_mirror_health({"health_score": 85, "issues": []})
    assert "85/100" in rows[0][2]


def test_parse_mirror_health_bad():
    rows = _parse_mirror_health({"health_score": 40, "issues": ["mirror down"]})
    assert "40/100" in rows[0][2]
    assert "1 issues" in rows[0][2]


def test_parse_mirror_health_error():
    rows = _parse_mirror_health({"error_type": "x"})
    assert "Error" in rows[0][2]


def test_parse_btrfs_info():
    data = {"label": "archlinux", "total_devices": 1}
    rows = _parse_btrfs_info(data)
    assert "archlinux" in rows[0][2]


def test_parse_btrfs_info_skipped():
    rows = _parse_btrfs_info({"error_type": "x", "message": "no btrfs"})
    assert "Skipped" in rows[0][2]


def test_parse_btrfs_device_stats_clean():
    data = {"devices": [{"error_stats": {"write": 0, "read": 0, "corruption": 0}}]}
    rows = _parse_btrfs_device_stats(data)
    assert "\u2705 0" in rows[0][2]


def test_parse_btrfs_device_stats_errors():
    data = {"devices": [{"error_stats": {"write": 5}}]}
    rows = _parse_btrfs_device_stats(data)
    assert "5" in rows[0][2]


def test_parse_btrfs_scrub_ok():
    data = {"scrub": {"status": "finished", "error_summary": "no errors found"}}
    rows = _parse_btrfs_scrub(data)
    assert "\u2705" in rows[0][2]


def test_parse_btrfs_scrub_skipped():
    rows = _parse_btrfs_scrub({"error_type": "skip"})
    assert "Skipped" in rows[0][2]


def test_parse_boot_status():
    data = {
        "boot_order": "0xf416",
        "sequence": [
            {"mode": "0x1", "label": "SD"},
            {"mode": "0x6", "label": "NVMe"},
            {"mode": "0x4", "label": "USB"},
        ],
        "available_devices": {"sd": True, "nvme": True, "usb": False},
    }
    rows = _parse_boot_status(data)
    assert "0xf416" in rows[0][2]
    assert "SD → NVMe → USB" in rows[1][2]
    assert "sd, nvme" in rows[2][2]


def test_parse_boot_status_skipped():
    rows = _parse_boot_status({"error_type": "x", "message": "no eeprom"})
    assert "Skipped" in rows[0][2]
