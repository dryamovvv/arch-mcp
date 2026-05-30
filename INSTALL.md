# INSTALL.md — для AI-агентов

Ты устанавливаешь arch-linux MCP сервер и сопутствующие файлы. Установка двухкомпонентная:

1. **MCP сервер** — на целевую машину (та, которой будем управлять)
2. **Skills и commands** — в твою конфигурацию (локально, чтобы ты знал что умеешь)

---

## 1. Установка MCP сервера на целевую машину

Сервер должен работать на той машине, которой управляем (RPi5, десктоп, сервер). Не на клиенте AI.

### uvx (любая система, universal wheel)

```bash
uvx --from https://github.com/dryamovvv/arch-mcp/releases/latest/download/arch_ops_server-3.4.3-py3-none-any.whl arch-ops-server
```

Или установка как инструмент:

```bash
uv tool install https://github.com/dryamovvv/arch-mcp/releases/latest/download/arch_ops_server-3.4.3-py3-none-any.whl
```

### Arch Linux (pacman) — systemd service included

Готовый пакет из GitHub Releases:

```bash
curl -fsSL https://raw.githubusercontent.com/dryamovvv/arch-mcp/master/scripts/install.sh | bash
```

Или собрать локально (рекомендуется для RPi5 aarch64):

```bash
git clone https://github.com/dryamovvv/arch-mcp.git
cd arch-mcp/packaging/arch
makepkg -si
```

Пакет: `packaging/arch/PKGBUILD`

Что ставит:
- `/usr/bin/arch-ops-server` — STDIO сервер
- `/usr/bin/arch-ops-server-http` — HTTP сервер (:8080)
- Systemd сервис `arch-ops-server.service` (включён и запущен)

---

## 2. Установка skills и commands в твою конфигурацию

Skills учат тебя пользоваться сервером. Commands — готовые сценарии для запуска.

### Что куда копировать

| Источник | Назначение | Куда |
|----------|-----------|------|
| `.agents/skills/arch-system/SKILL.md` | Управление системой RPi (30+ инструментов) | `skills/arch-system.md` |
| `.agents/skills/arch-report/SKILL.md` | Генерация отчётов о здоровье системы | `skills/arch-report.md` |
| `.agents/commands/run-audit.md` | Команда: полный аудит системы | `commands/run-audit.md` |
| `.agents/commands/manage-boot.md` | Команда: настройка загрузчика RPi | `commands/manage-boot.md` |

### Opencode

```bash
cp .agents/skills/arch-system/SKILL.md ~/.config/opencode/skills/arch-system.md
cp .agents/skills/arch-report/SKILL.md ~/.config/opencode/skills/arch-report.md
cp .agents/commands/run-audit.md ~/.config/opencode/commands/run-audit.md
cp .agents/commands/manage-boot.md ~/.config/opencode/commands/manage-boot.md
```

### Claude Code

```bash
cp .agents/skills/arch-system/SKILL.md ~/.claude/skills/arch-system/SKILL.md
cp .agents/skills/arch-report/SKILL.md ~/.claude/skills/arch-report/SKILL.md
# commands — см. документацию Claude Code по custom slash commands
```

Если не знаешь куда класть — спроси пользователя где его конфигурация агента.

---

## 3. Подключение MCP сервера к клиенту

После установки сервера на целевую машину добавь в конфиг MCP-клиента:

```json
{
  "mcpServers": {
    "arch-linux": {
      "command": "arch-ops-server"
    }
  }
}
```

**Имя сервера** (`arch-linux`) — это идентификатор машины. Можно подключить несколько:

```json
{
  "mcpServers": {
    "rpi5":              { "command": "ssh", "args": ["rpi5.local", "arch-ops-server"] },
    "vps-arch":          { "url": "http://10.0.0.5:8080/mcp",     "transport": "sse" },
    "ubuntu-box":        { "url": "http://192.168.1.50:8080/mcp", "transport": "sse" }
  }
}
```

Имена произвольные: `rpi5`, `vps-arch`, `ubuntu-box`, `192.168.1.50` — что угодно. Каждое имя становится префиксом инструментов: `rpi5_get_system_info`, `vps-arch_check_updates_dry_run` и т.д.

> Навыки (`arch-system`, `arch-report`) ждут префикс `arch-linux`. Если переименуешь сервер — сообщи об этом навыкам или переименуй их под свой префикс.

Если сервер на удалённой машине — используй SSE транспорт:

```json
{
  "mcpServers": {
    "arch-linux": {
      "url": "http://<target-ip>:8080/mcp",
      "transport": "sse",
      "headers": {
        "Authorization": "Bearer <token>"
      }
    }
  }
}
```

> `transport` зависит от MCP-клиента: `sse` (Claude Desktop, Opencode), `streamable-http` (новые клиенты). Smithery подключается без transport — шлёт POST напрямую.

Токен задаётся на сервере через переменную окружения:

```bash
# На целевой машине, в /etc/systemd/system/arch-ops-server.service.d/override.conf
# или перед запуском:
export ARCH_OPS_SERVER_API_KEY="<token>"
```

Без токена HTTP сервер открыт для всех в сети. В production ставь обязательно.

Имя сервера `arch-linux` важно — skills ссылаются на него как на `arch-linux_TOOL_NAME`.

---

## 4. Проверка

```bash
# На целевой машине:
systemctl status arch-ops-server
arch-ops-server --help
```

Убедись что сервер запущен и слушает (для HTTP: порт 8080).

---

## Краткий чек-лист для агента

- [ ] Сервер установлен на **целевую** машину (не на клиенте AI)
- [ ] `arch-ops-server` работает (`systemctl status arch-ops-server`)
- [ ] Skills скопированы в конфигурацию агента
- [ ] MCP клиент подключён к серверу (STDIO или HTTP)
- [ ] Имя сервера в конфиге: `arch-linux`
