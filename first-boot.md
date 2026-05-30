# First Boot Flow

## Порядок при первой загрузке

```
sysinit.target
  └─ systemd-firstboot.service   # интерактив: hostname, timezone, root password
  └─ systemd-repart.service      # расширяет root-раздел
  └─ systemd-growfs-root.service # расширяет ФС

multi-user.target
  └─ rpi5-firstboot.service      # useradd + chage -d 0
```

## systemd-firstboot

**При сборке** (`bootstrap::systemd_firstboot`): пишет locale, keymap, shell, machine-id.
Если `BUILD_HOSTNAME`/`BUILD_TIMEZONE`/`BUILD_ROOT_PASSWORD` заданы — пишет их тоже.
Если нет — файлы не создаются → systemd-firstboot **спросит** при загрузке.

**tty drop-in** (`src/conf/systemd/systemd-firstboot.service.d/prompt.conf`):
```ini
[Service]
StandardInput=tty
StandardOutput=tty
TTYPath=/dev/console
```
Захватывает консоль — промпты не перебиваются логами.

## rpi5-firstboot.service

**Скрипт** (`/usr/local/lib/rpi5-archlinux/firstboot.sh`):
```bash
#!/bin/bash
set -euo pipefail

if ! id -u "$user_name" >/dev/null 2>&1; then
    useradd -m -G wheel "$user_name"
    chage -d 0 "$user_name"
fi
```

Пароль не задается — пользователь обязан сменить при первом логине.

## partition/filesystem grow

Нативные systemd-юниты, включаются при сборке:
- `systemd-repart.service` — ищет `Type=root-arm64` в `/etc/repart.d/50-root.conf`
- `systemd-growfs-root.service` — расширяет ext4

## Локали

`locale-gen` запускается при сборке после `systemd-firstboot`.
`en_US.UTF-8` добавляется в `/etc/locale.gen` через `bootstrap::locale_gen_file()`.
