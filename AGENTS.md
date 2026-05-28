# AGENTS.md

## Project

MCP server bridging AI assistants with the Arch Linux ecosystem (Wiki, AUR, official repos, system management). Dual-licensed: GPL-3.0-only OR MIT.

## Commands

- **Install dev deps:** `uv pip install -e ".[dev]"`
- **Install with HTTP support:** `uv pip install -e ".[dev,http]"`
- **Run tests:** `pytest` (async mode auto, coverage included)
- **Build:** `uv build`
- **Run (STDIO):** `arch-ops-server`
- **Run (HTTP):** `arch-ops-server-http`

No lint, typecheck, or format commands are configured. Do not fabricate them.

## Architecture

- **`src/arch_ops_server/server.py`** — MCP server: 22 tools, 8 prompts, resources. The core file (~2000 lines).
- **`src/arch_ops_server/http_server.py`** — HTTP/SSE transport (Starlette + uvicorn) for Smithery/cloud clients.
- **`src/arch_ops_server/pacman.py`** — Hybrid local/remote: tries `pacman -Si` first, falls back to archlinux.org API.
- **`src/arch_ops_server/aur.py`** — AUR search, PKGBUILD retrieval, security audit (50+ red flags).
- **Remaining modules** — `wiki.py`, `system.py`, `news.py`, `mirrors.py`, `config.py`, `logs.py`, `groups.py`, `journal.py`, `system_health_check.py`, `utils.py`, `tool_metadata.py`.

## Key conventions

- **Platform gating:** Arch-only tools check `IS_ARCH` (from `utils.py`). Non-Arch hosts get descriptive error messages, not crashes.
- **Unified action pattern:** Many MCP tools use an `action` parameter to multiplex operations (e.g. `manage_orphans(action='list'|'remove')`, `query_file_ownership(mode='file_to_package'|'package_to_files'|'filename_search')`).
- **Python target:** 3.11+ (`.python-version` says 3.13).
- **CI:** GitHub Actions run only on `v*.*.*` tag pushes — publish to PyPI and GHCR. No PR/test CI.

## Testing

- Pytest with `pytest-asyncio` in `auto` mode. Use `await` freely in test functions.
- Tests mock external deps (httpx, subprocess, file I/O). No real Arch system needed.
- Arch-gated tests use `@pytest.mark.skipif(not IS_ARCH, ...)`.
- Standalone HTTP integration test: `python test_http_server.py` (expects server on localhost:8080).
