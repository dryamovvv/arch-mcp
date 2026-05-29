# Arch MCP — Demo Report

**Тестовая машина:** 192.168.1.54 (archlinux-develop)  
**Дата:** 2026-05-30  
**MCP-сервер:** arch-linux (HTTP SSE, порт 8080)

---

## Установка

Сервер установлен через сборку Arch-пакета (`makepkg -si`) из `packaging/arch/PKGBUILD`:
- `/usr/bin/arch-ops-server` (STDIO)
- `/usr/bin/arch-ops-server-http` (HTTP :8080)
- Зависимости: btrfs-progs, snapper — установлены; rpi-eeprom — присутствует

**Проблема установки:** `uv pip install --target` перезаписывает кастомные wrapper-скрипты с PYTHONPATH. Сервер запущен вручную: `env PYTHONPATH=/opt/arch-ops-server/vendor python -m arch_ops_server.http_server`. Нужно починить PKGBUILD wrapper-ы.

---

## Инструменты — результаты тестирования

### System/Monitoring (6 тулов)

| # | Инструмент | Статус | Результат |
|---|-----------|--------|-----------|
| 1 | `get_system_info` | ✅ | Kernel 6.18.33-3-rpi-16k, aarch64, 16GB RAM |
| 2 | `analyze_storage disk_usage` | ✅ | 4 пути, 913G свободно на каждом, 2% |
| 3 | `analyze_storage cache_stats` | ✅ | 301 пакет, 602.84 MB |
| 4 | `diagnose_system failed_services` | ✅ | 0 failed services |
| 5 | `manage_logs` | ✅ | Логи systemd (SSH-сессии видны) |
| 6 | `run_system_health_check` | ✅ | Полный агрегированный отчёт |

### Package Tools (9 тулов)

| # | Инструмент | Статус | Результат |
|---|-----------|--------|-----------|
| 7 | `check_updates_dry_run` | ✅ | 6 пакетов: cpupower, mkinitcpio, systemd, systemd-libs… |
| 8 | `manage_orphans list` | ✅ | 0 orphaned packages |
| 9 | `check_database_freshness` | ✅ | 5 БД, все stale (community 26533h!) |
| 10 | `get_official_package_info` | ✅ | linux-rpi-16k 6.18.33-3, repo=core |
| 11 | `query_file_ownership` | ✅ | /usr/bin/pacman → pacman 7.1.0.r9 |
| 12 | `manage_groups list_groups` | ✅ | 100 групп (gnome, i3, cosmic…) |
| 13 | `manage_install_reason list` | ✅ | 45 explicit пакетов |
| 14 | `verify_package_integrity` | ✅ | Работает (0 missing files) |
| 15 | `query_package_history all` | ✅ | 0 транзакций (чистый лог) |

### Discovery (4 тула)

| # | Инструмент | Статус | Результат |
|---|-----------|--------|-----------|
| 16 | `search_archwiki "pacman"` | ✅ | 3 результата (Pacman, переводы) |
| 17 | `search_aur "yay"` | ✅ | 37 результатов (yay 12.5.7-1 первый) |
| 18 | `fetch_news critical` | ✅ | 2 критические новости (varnish→vinyl-cache) |
| 19 | `fetch_news latest` | ✅ | Работает |

### BTRFS (5 тулов)

| # | Инструмент | Статус | Результат |
|---|-----------|--------|-----------|
| 20 | `analyze_btrfs filesystem_info` | ✅ | label=archlinux, 1 device (nvme0n1p2) |
| 21 | `analyze_btrfs device_stats` | ✅ | 0 ошибок на всех устройствах |
| 22 | `analyze_btrfs scrub_status` | ✅ | no errors found, 18.30GiB total |
| 23 | `analyze_btrfs snapshots` | ⚠️ | snapper: No permissions (нужен sudo) |
| 24 | `manage_btrfs_scrub status` | ✅ | Scrub OK, no errors |

### Mirrors & Config (3 тула)

| # | Инструмент | Статус | Результат |
|---|-----------|--------|-----------|
| 25 | `optimize_mirrors health` | ✅ | Score 45/100, 1 активное зеркало |
| 26 | `analyze_pacman_conf` | ✅ | 4 репо (core, extra, alarm, aur), 5 parallel downloads |
| 27 | `analyze_makepkg_conf` | ✅ | aarch64, -O2, !debug, !lto |

### Boot (RPi only) — READ-ONLY

| # | Инструмент | Статус | Результат |
|---|-----------|--------|-----------|
| 28 | `manage_boot status` | ✅ | BOOT_ORDER=0xf416, NVMe→SD→USB, sd+NVMe available, restore disabled |

### Report

| # | Инструмент | Статус | Результат |
|---|-----------|--------|-----------|
| 29 | `generate_report full` | ✅ | 34 строки, 7 секций, все данные корректны |

---

## `generate_report` — результат

| Программа | Что значит | Результат |
|-----------|------------|------------|
| **System** | | |
| uname | Kernel | 6.18.33-3-rpi-16k |
| uname | Architecture | aarch64 |
| uname | Hostname | archlinux-develop |
| uname | Uptime | up 1 hour, 5 minutes |
| free | RAM | 15658M free / 16212M total |
| systemctl | Failed services | ✅ 0 |
| **Storage** | | |
| df | / (913G free) | ✅ 2.0% used |
| df | /home (913G free) | ✅ 2.0% used |
| df | /var (913G free) | ✅ 2.0% used |
| df | /var/cache/pacman/pkg (913G free) | ✅ 2.0% used |
| paccache | Pacman cache size | 602.84 MB (301 packages) |
| **Packages** | | |
| pacman | Pending updates | ❌ 6 packages |
| pacman | Orphaned packages | ✅ 0 |
| pacman | DB community | ❌ 26532h |
| pacman | DB aur | ❌ 244h |
| pacman | DB core | ❌ 51h |
| pacman | DB alarm | ❌ 47h |
| pacman | DB extra | ❌ 46h |
| **Mirrors** | | |
| mirrors | Health score | ❌ 45/100 (1 issues) |
| **BTRFS** | | |
| btrfs | Label | archlinux |
| btrfs | Devices | 1 |
| btrfs | Device errors | ✅ 0 |
| btrfs | Scrub | ✅ no errors found |
| **Boot** | | |
| boot | BOOT_ORDER | 0xf416 |
| boot | Boot sequence | NVMe → SD → USB → RESTART |
| boot | Available devices | sd, nvme |
| **Config** | | |
| pacman.conf | Config available | ✅ |

---

## Найденные баги (исправлены в процессе)

1. **report.py `_parse_boot_status`** — использовал несуществующие ключи (`boot_order_raw`, `boot_order_decoded`, `devices`) вместо реальных (`boot_order`, `sequence`, `available_devices`)
2. **report.py `_parse_db_freshness`** — `databases` приходит списком, а не dict → крах `'list' object has no attribute 'items'`
3. **report.py `_parse_mirror_health`** — `health_score` вложен в `assessment.*`
4. **report.py `_parse_btrfs_device_stats`** — `devices` приходит dict (путь→статистика), а не list
5. **report.py `_parse_system_info`** — ключи `memory_total_mb`/`memory_available_mb`, а не `memory.total`/`memory.used`
6. **report.py `_parse_disk_usage`** — возвращается dict `disk_usage` с путями, а не `filesystems` list
7. **report.py `_parse_cache_stats`** — ключи `total_size_mb`/`package_count`, а не `current_size`/`cached_packages`
8. **report.py `_parse_updates`** — ключ `count`, а не `update_count`

## Оставшиеся проблемы

1. **PKGBUILD wrapper-скрипты** — `uv pip install --target` перезаписывает кастомные wrapper-ы. Нужно генерировать wrapper-ы ПОСЛЕ `uv pip install`, а не ДО.
2. **snapshots** — `No permissions` для `snapper list`. Нужен sudo или права пользователя.
3. **Установка `fakeroot`** — скрипт `install.sh` требует `fakeroot` для сборки, но не предупреждает об этом.
