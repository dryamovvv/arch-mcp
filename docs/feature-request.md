# Feature Requests for MCP Server (OS Build Testing)

Анализ доступных инструментов [arch-ops-server](https://github.com/dryamovvv/arch-mcp)
против потребностей тестирования сборки RPi5 Arch Linux образа.

## Уже реализовано (27 инструментов)

Полный набор для управления пакетами, BTRFS, bootloader, репозиториями, логами, новостями.
Пробелов в базовых операциях нет.

## Предлагаемые новые инструменты

### 1. `verify_boot_artifacts` — валидация boot-раздела

**Зачем:** После сборки или после загрузки подтвердить, что все boot-файлы на месте.

**Действия:**
- `list` — перечислить файлы на /boot или ESP
- `check` — проверить наличие обязательных: `kernel8.img`, `initramfs-linux.img`, `bcm2712-rpi-5-b.dtb`, `config.txt`, `cmdline.txt`
- `cmdline` — распарсить cmdline.txt: проверить `root=UUID=...`, `rootflags=subvol=@`, отсутствие `__ROOT_UUID__`

**Приоритет:** HIGH — замена ручного `mcopy -i ...@@1048576`

### 2. `verify_service_health` — агрегация состояния сервисов

**Зачем:** После первого бута проверить, что все обязательные сервисы запущены.

**Действия:**
- `status` — список сервисов с состоянием: active/inactive/failed/not-found
- `compare` — сравнить с ожидаемым списком (из build.conf, шаблона, или дефолтного набора)
- `checklist` — быстрый да/нет по критическим сервисам:
  ```
  systemd-homed.service  → active ✅
  sshd.service           → active ✅
  systemd-networkd       → active ✅
  fail2ban.service       → active ✅
  arch-ops-mcp.service   → active ✅
  snapper-timeline.timer → active ✅
  ```

**Приоритет:** HIGH — 90% проверок после бута сводятся к этому

### 3. `verify_homectl_user` — проверка пользователя homectl

**Зачем:** Убедиться, что пользователь создан через systemd-homed, а не useradd.

**Действия:**
- `status` — вывод `homectl list -j`, `loginctl show-user $USER`, linger status
- `check` — проверить: storage=subvolume, member-of=wheel, password-change-now=yes, linger=yes
- `snapper` — проверить `snapper -c user_home list`

**Приоритет:** HIGH — гомект — ключевое изменение в сборке

### 4. `compare_packages` — сравнение пакетов с build.conf

**Зачем:** После сборки или обновления убедиться, что все пакеты из `BUILD_PACKAGES` установлены.

**Действия:**
- `diff` — сравнить установленные пакеты (или список из `pacstrap`-лога) с `BUILD_PACKAGES` из `build.conf`
- `missing` — показать только отсутствующие
- `extra` — показать лишние (установленные, но не в списке)

**Приоритет:** MEDIUM — полезно для отладки CI-сборок

### 5. `check_rpi_hardware` — специфичные для RPi5 проверки

**Зачем:** Подтвердить, что железо работает корректно после сборки.

**Действия:**
- `eeprom` — версия и канал EEPROM, конфиг BOOT_ORDER
- `temperature` — текущая температура + throttling status (`vcgencmd get_throttled`)
- `frequencies` — текущие частоты CPU/GPU
- `voltage` — напряжение ядра
- `memory_split` — gpu_mem из config.txt

**Приоритет:** MEDIUM — для QA после заливки образа

### 6. `benchmark_quick` — быстрый бенчмарк

**Зачем:** Базовая проверка производительности после сборки.

**Действия:**
- `disk` — `hdparm -Tt` или `fio --rw=read` на корневом разделе
- `cpu` — `openssl speed` или `stress-ng --cpu 1 --timeout 5s`
- `memory` — `stress-ng --vm 1 --timeout 5s`
- `network` — `curl -o /dev/null -w '%{speed_download}' http://mirror.archlinuxarm.org/`

**Приоритет:** LOW — nice-to-have, не критично для тестирования

### 7. `check_security_posture` — аудит безопасности после сборки

**Зачем:** Убедиться, что базовые настройки безопасности применены.

**Действия:**
- `sshd` — PermitRootLogin, PasswordAuthentication, AllowUsers, PermitEmptyPasswords
- `fail2ban` — статус, активные jails
- `passwords` — проверить, что root и user имеют пароли (не пустые), user имеет `--password-change-now=yes`
- `sudo` — проверить `/etc/sudoers.d/10-wheel`
- `mcp` — API-ключ не дефолтный, порт слушает, сервис запущен

**Приоритет:** MEDIUM

### 8. `compare_fstab` — валидация fstab

**Зачем:** Убедиться, что fstab соответствует btrfs subvolume layout.

**Действия:**
- `check` — распарсить fstab, проверить: 8 subvolume-маунтов, `@`, `@home`, `@snapshots`, `@swap`, `@var_log`, `@var_cache`, `@var_tmp`, `@var_lib`
- `options` — проверить корректность опций монтирования: `compress=zstd` для данных, `nodatacow` для cache/tmp/swap
- `nofail` — проверить `nofail` на /boot (ESP)

**Приоритет:** HIGH — критично для btrfs, ошибка в fstab = незагружаемая система

## Сводка приоритетов

| Приоритет | Инструмент | Зачем |
|-----------|-----------|-------|
| **HIGH** | `verify_boot_artifacts` | Замена mcopy-костылей в CI |
| **HIGH** | `verify_service_health` | 90% QA после бута |
| **HIGH** | `verify_homectl_user` | Ключевое изменение сборки |
| **HIGH** | `compare_fstab` | btrfs subvolume layout |
| **MEDIUM** | `compare_packages` | Отладка CI-сборок |
| **MEDIUM** | `check_security_posture` | Базовая безопасность |
| **MEDIUM** | `check_rpi_hardware` | QA железа RPi5 |
| **LOW** | `benchmark_quick` | Производительность |

## Реализация

Все инструменты — Python-функции в `arch-ops-server`, выполняющие shell-команды
на целевой системе. Не требуют новых зависимостей (все утилиты уже в образе:
`systemd`, `btrfs-progs`, `snapper`, `homectl`, `pacman`, `vcgencmd`, `hdparm`).

Добавление в MCP-сервер: форкнуть https://github.com/dryamovvv/arch-mcp,
добавить инструменты в `src/arch_ops_server/tools/`, обновить `server.py` для
регистрации, выпустить релиз. Образ сам подхватит при следующей сборке
(`uv tool install --from git+https://github.com/dryamovvv/arch-mcp`).
