---
description: RPi boot management — check/set boot order, one-time reboot
agent: general
subtask: false
---

Use the `arch-linux` MCP `manage_boot` tool to handle RPi5 bootloader operations.

## Actions

### Check status
```
arch-linux_manage_boot action="status"
```
Shows current BOOT_ORDER, decoded sequence (NVMe → SD → USB → ...), available devices, and pending restore.

### Change boot order permanently
```
arch-linux_manage_boot action="set_boot_order" order="nvme_first"
```
Presets: `sd_first`, `nvme_first`, `usb_first`, `sd_nvme`, `nvme_sd`, `sd_only`, `nvme_only`, `usb_only`. Or raw hex: `order="0xf416"`.

### One-time boot from specific device (without reboot)
```
arch-linux_manage_boot action="next_boot" device="sd"
```
Saves current BOOT_ORDER, sets temporary, installs restore service. Reboot manually.

### One-time boot + immediate reboot
```
arch-linux_manage_boot action="next_boot" device="nvme" reboot=True
```
Same as above plus immediate reboot. Original BOOT_ORDER restored automatically on next boot.

## Preset reference

| Preset | BOOT_ORDER | Sequence |
|--------|-----------|----------|
| sd_first | 0xf41 | SD → USB → restart |
| nvme_first | 0xf46 | NVMe → USB → restart |
| usb_first | 0xf14 | USB → SD → restart |
| sd_nvme | 0xf16 | SD → NVMe → restart |
| nvme_sd | 0xf61 | NVMe → SD → restart |
| sd_only | 0xf1 | SD → restart |
| nvme_only | 0xf6 | NVMe → restart |
| usb_only | 0xf4 | USB → restart |

## Safety

- `set_boot_order` is permanent — confirm before applying.
- `next_boot` is one-time — original order restored automatically via systemd oneshot.
- If restore fails, run `rpi-eeprom-config` manually or re-run `manage_boot action="set_boot_order"`.
