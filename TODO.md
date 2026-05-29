# TODO

## CI-отчеты для сборок

Хочется иметь отчет в виде голой таблицы:

```
| Программа | Что значит | Результат |
|-----------|------------|-----------|
| ...       | ...        | ...       |
```

Можно запускать в CI и получать отчеты по каждому билду.

### План

1. **MCP-инструмент `generate_report`** — генерирует табличный отчет (bare table).
   - Вход: список проверок или автоопределение (все health-инструменты).
   - Выход: markdown-таблица `| Программа | Что значит | Результат |`.
   - Для CI: можно дергать через HTTP API сервера.

2. **SKILL `audit-report`** — генерирует отчет в свободной форме по ситуации.
   - Использует существующие MCP-инструменты (health_check, analyze_* , diagnose_*).
   - Сам решает, на чем сделать акцент, в зависимости от найденных проблем.
   - Выход: человекочитаемый markdown с секциями (System, Packages, Storage, Security, Recommendations).

3. **CI workflow** (`.github/workflows/audit.yml`):
   - Запускается по расписанию (cron) или на PR/push.
   - Подключается к RPi5 по SSH.
   - Дергает `generate_report` через HTTP API.
   - Сохраняет результат как артефакт или публикует в GitHub Pages.

### Команды (CLI)

- `/audit report` — табличный отчет (bare table, CI-friendly).
- `/audit full` — полный отчет в свободной форме (через SKILL).

### Что нужно сделать

- [x] `generate_report` MCP-инструмент (bare table)
- [x] SKILL `audit-report` (свободная форма)
- [x] CI workflow `.github/workflows/audit.yml`
- [x] Интеграция с существующими health/analyze/diagnose инструментами
