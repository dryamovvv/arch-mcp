# INSTALL.md — для AI-агентов

Ты устанавливаешь `arch-opsd` (Rust MCP сервер для Arch Linux) на целевую машину.

## 1. Установка сервера на целевую машину

### Быстрая установка (pre-built бинарник)

```bash
# Скачать последний релиз (aarch64 или x86_64)
curl -fsSL https://github.com/dryamovvv/arch-mcp/releases/latest/download/arch-opsd-linux-aarch64 -o /usr/local/bin/arch-opsd
chmod +x /usr/local/bin/arch-opsd
```

### Сборка из исходников

```bash
git clone https://github.com/dryamovvv/arch-mcp.git
cd arch-mcp
cargo build --release
cp target/release/arch-opsd /usr/local/bin/
```

### Кросс-компиляция для RPi5 (aarch64)

```bash
# На x86_64 машине:
cargo install cross
cross build --release --target aarch64-unknown-linux-musl
# Бинарник: target/aarch64-unknown-linux-musl/release/arch-opsd
# Скопировать на RPi5
```

### Systemd сервис (опционально)

```ini
# /etc/systemd/system/arch-opsd.service
[Unit]
Description=Arch MCP Server (arch-opsd)

[Service]
ExecStart=/usr/local/bin/arch-opsd stdio
Restart=always
User=root

[Install]
WantedBy=multi-user.target
```

```bash
systemctl daemon-reload
systemctl enable --now arch-opsd
```

## 2. Установка skills и commands в твою конфигурацию

| Источник | Куда |
|----------|------|
| `.agents/skills/arch-system/SKILL.md` | `skills/arch-system.md` |
| `.agents/skills/arch-report/SKILL.md` | `skills/arch-report.md` |
| `.agents/commands/run-audit.md` | `commands/run-audit.md` |
| `.agents/commands/manage-boot.md` | `commands/manage-boot.md` |

## 3. Подключение MCP сервера к клиенту

```json
{
  "mcpServers": {
    "arch-linux": {
      "command": "arch-opsd",
      "args": ["stdio"]
    }
  }
}
```

Через SSH:

```json
{
  "mcpServers": {
    "rpi5": {
      "command": "ssh",
      "args": ["rpi5.local", "arch-opsd", "stdio"]
    }
  }
}
```

## 4. Проверка

```bash
arch-opsd stdio --help
# Или:
echo '{"id":1,"method":"initialize","params":{}}' | arch-opsd stdio
# Должен ответить JSON-RPC с capabilities
```

## Чек-лист

- [ ] Бинарник `arch-opsd` на целевой машине
- [ ] Skills скопированы в конфигурацию агента
- [ ] MCP клиент подключён (STDIO или SSH)
- [ ] Имя сервера в конфиге: `arch-linux`
