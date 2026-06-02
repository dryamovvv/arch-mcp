# AGENTS.md

## Project

MCP server bridging AI assistants with the Arch Linux ecosystem (Wiki, AUR, official repos, system management). Dual-licensed: GPL-3.0-only OR MIT.

## Commands

- **Build (release):** `cargo build --release`
- **Build (http feature):** `cargo build --release --features http`
- **Check:** `cargo check`
- **Cross (aarch64):** `cross build --release --target aarch64-unknown-linux-musl`
- **Run (STDIO):** `cargo run -- stdio`
- **Run (HTTP):** `cargo run --features http -- http`

No lint, typecheck, format, or test commands are configured. Do not fabricate them.

## Architecture

- **`src/server.rs`** — STDIO transport + JSON-RPC dispatch (48 tools, 8 prompts, 24 resources) plus `handle_mcp_request()` for HTTP dispatch
- **`src/http_server.rs`** — HTTP/SSE transport (axum, feature-gated `--features http`)
- **`src/tools/`** — All 48 tool handler modules
- **`src/resources.rs`** — 24 resource URI handlers across 8 schemes
- **`src/prompts.rs`** — 8 prompt templates
- **`src/arch_ops_server/`** — Original Python reference (still present)

## Key conventions

- **Platform gating:** Arch-only tools check `is_arch_linux()` (reads `/etc/arch-release`). Non-Arch hosts get descriptive error messages, not crashes.
- **Unified action pattern:** Many MCP tools use an `action` parameter to multiplex operations (e.g. `manage_orphans(action='list'|'remove')`, `query_file_ownership(mode='file_to_package'|'package_to_files'|'filename_search')`).
- **Rust target:** edition 2021, tokio async runtime, serde_json for JSON-RPC.
- **CI:** GitHub Actions run only on `v*.*.*` tag pushes — publish to GitHub Releases and GHCR. No PR/test CI.

## Testing

- Pytest with `pytest-asyncio` in `auto` mode. Use `await` freely in test functions.
- Tests mock external deps (httpx, subprocess, file I/O). No real Arch system needed.
- Arch-gated tests use `@pytest.mark.skipif(not IS_ARCH, ...)`.
- Standalone HTTP integration test: `python test_http_server.py` (expects server on localhost:8080).
