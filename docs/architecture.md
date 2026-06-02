# Architecture

## Overview

MCP server bridging AI assistants with the Arch Linux ecosystem (Wiki, AUR, official repos, system management). Written in Rust, dual-licensed: GPL-3.0-only OR MIT.

## Source Layout

| File | Purpose |
|------|---------|
| `src/server.rs` | STDIO transport + JSON-RPC dispatch (48 tools, 8 prompts, 24 resources) plus `handle_mcp_request()` for HTTP dispatch |
| `src/http_server.rs` | HTTP/SSE transport (axum, feature-gated `--features http`) |
| `src/tools/` | All 48 tool handler modules |
| `src/resources.rs` | 24 resource URI handlers across 8 schemes |
| `src/prompts.rs` | 8 prompt templates |
| `src/protocol.rs` | MCP protocol types (JSON-RPC) |
| `src/platform.rs` | Platform detection (`is_arch_linux()`) |
| `src/command.rs` | Subprocess execution helpers |
| `src/client.rs` | HTTP client helpers (reqwest) |
| `src/error.rs` | Error types |

## Key Conventions

- **Platform gating:** Arch-only tools check `is_arch_linux()` (reads `/etc/arch-release`). Non-Arch hosts get descriptive error messages, not crashes.
- **Unified action pattern:** Many tools use an `action` parameter to multiplex operations (e.g. `manage_orphans(action='list'|'remove')`, `query_file_ownership(mode='file_to_package'|'package_to_files'|'filename_search')`).
- **Rust target:** edition 2021, tokio async runtime, serde_json for JSON-RPC.
- **CI:** GitHub Actions run only on `v*.*.*` tag pushes — publish to GitHub Releases and GHCR. No PR/test CI.

## Build Commands

| Command | Description |
|---------|-------------|
| `cargo build --release` | Release build (STDIO) |
| `cargo build --release --features http` | Release build with HTTP/SSE transport |
| `cargo check` | Quick compilation check |
| `cross build --release --target aarch64-unknown-linux-musl` | Cross-compile for aarch64 |
| `cargo run -- stdio` | Run in STDIO mode |
| `cargo run --features http -- http` | Run in HTTP mode |

## Transport

- **STDIO** — Always available, uses stdin/stdout for JSON-RPC messages
- **HTTP/SSE** — Optional (`--features http`), axum-based with Server-Sent Events for server→client messages and direct POST for client→server
