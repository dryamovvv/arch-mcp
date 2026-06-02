# Rust Port: `arch-ops-server` → `arch-opsd`

**Date:** 2026-06-02
**Status:** Design

## Motivation

- **A:** Single static binary (~5 MB), zero runtime dependencies (no Python interpreter)
- **B:** Lower memory footprint and faster startup on resource-constrained RPi5
- All 41 tools + 8 prompts + 38 resources — full functional equivalent of Python v3.5.0

## Platform

- Build target: `aarch64-unknown-linux-musl` (primary) and `x86_64-unknown-linux-musl`
- Cross-compilation via `cross` CLI tool
- Runtime: Linux only, Arch Linux detection via `/etc/arch-release`

## Cargo Dependencies

```toml
[package]
name = "arch-opsd"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1", features = ["full"] }
tokio-stream = "0.1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
clap = { version = "4", features = ["derive"] }
reqwest = { version = "0.12", features = ["json"] }
scraper = "0.20"
chrono = { version = "0.4", features = ["serde"] }
regex = "1"
once_cell = "1"
axum = { version = "0.8", optional = true }
tower = { version = "0.5", optional = true }

[features]
default = ["http"]
http = ["dep:axum", "dep:tower"]

[profile.release]
opt-level = "z"       # minimal binary size
lto = true
codegen-units = 1
strip = "symbols"
```

## Project Structure

```
arch-opsd/
├── Cargo.toml
├── cross.toml                  # cross-compilation for aarch64
├── src/
│   ├── main.rs                 # clap entry: stdio | http modes
│   ├── protocol.rs             # MCP JSON-RPC types + serialize
│   ├── server.rs               # ToolRegistry, dispatcher, transport loop
│   ├── platform.rs             # is_arch_linux(), cpu_arch()
│   ├── command.rs              # tokio::process::Command wrapper
│   ├── client.rs               # reqwest HTTP client wrapper
│   ├── error.rs                # ToolError, ErrorKind
│   ├── tools/
│   │   ├── mod.rs              # ToolRegistry, ToolHandler trait
│   │   ├── wiki.rs             # search_archwiki
│   │   ├── aur.rs              # search_aur, audit_package_security, install_package_secure
│   │   ├── pacman.rs           # get_official_package_info, check_updates_dry_run
│   │   ├── packages.rs         # manage_orphans, manage_install_reason, verify_package_integrity, check_database_freshness
│   │   ├── files.rs            # query_file_ownership, manage_groups
│   │   ├── system.rs           # get_system_info, analyze_storage, diagnose_system
│   │   ├── health.rs           # run_system_health_check
│   │   ├── news.rs             # fetch_news
│   │   ├── logs.rs             # query_package_history
│   │   ├── journal.rs          # manage_logs, manage_journal_gateway
│   │   ├── mirrors.rs          # optimize_mirrors
│   │   ├── config.rs           # analyze_pacman_conf, analyze_makepkg_conf
│   │   ├── btrfs.rs            # analyze_btrfs, manage_btrfs_snapshots, manage_btrfs_scrub
│   │   ├── boot.rs             # manage_boot
│   │   ├── report.rs           # generate_report
│   │   ├── build_test.rs       # verify_boot_artifacts, verify_service_health, verify_homectl_user, compare_fstab, compare_packages, check_security_posture, check_rpi_hardware, benchmark_quick
│   │   ├── luks.rs
│   │   ├── firewall.rs
│   │   ├── hardware.rs
│   │   ├── boot_config.rs
│   │   ├── backup.rs
│   │   ├── recovery.rs
│   │   └── telegram.rs
│   ├── resources.rs            # resource URI handlers (archwiki://, aur://, archrepo://, etc.)
│   └── prompts.rs              # 8 prompt generators
```

## Architectural Decisions

### Protocol Layer (`protocol.rs`)
Pure JSON-RPC 2.0 with serde. No MCP SDK. Types:
- `Request { id, method, params }`
- `Response { id, result }` / `Error { id, error: { code, message, data } }`
- `Tool { name, description, input_schema }`
- `Resource { uri, name, mime_type }`
- `Prompt { name, description, arguments }`

### Tool Dispatch (`tools/mod.rs`)
- `ToolHandler` trait: `async fn call(&self, args: HashMap<String, Value>) -> Result<Value, ToolError>`
- `ToolRegistry` stores `HashMap<&'static str, ToolEntry>` where `ToolEntry { info: Tool, handler: Box<dyn ToolHandler> }`
- 41 + 7 = 48 tools registered at startup

### Platform Gating (`platform.rs`)
```rust
pub fn is_arch_linux() -> bool { Path::new("/etc/arch-release").exists() }
```
Cached via `OnceLock<bool>`. All tools that require Arch check this and return `ToolError::PlatformNotArch`.

### HTTP Client (`client.rs`)
Thin wrapper around `reqwest::Client`:
- `get_json(url)` — for AUR RPC, Wiki API, archlinux.org API, mirror status API
- `get_html(url)` — for Wiki scraping fallback
- `head(url)` — for mirror speed testing
- Rate limiting: 1 req/100ms for AUR (as in Python original)

### Command Execution (`command.rs`)
Wraps `tokio::process::Command`:
- `run(cmd, args) -> Result<(exit_code, stdout, stderr)>`
- Timeout via `tokio::time::timeout`
- Sudo detection: if password needed, return structured error

### Error Handling (`error.rs`)
```rust
enum ErrorKind {
    PlatformNotArch,
    PacmanFailed(String),       // cmd + exit code
    NetworkError(String),       // reqwest errors
    ParseError(String),         // regex / serde / INI parse failures
    InvalidArgument(String),    // bad action, unknown package, etc
    Timeout,
    PermissionDenied,
    Internal(String),           // unexpected
}

struct ToolError {
    kind: ErrorKind,
    message: String,
    wiki_suggestions: Vec<String>,
}
```

### MCP Transports (`server.rs`)
**STDIO transport (always enabled):**
- Read `\n`-delimited JSON-RPC from stdin
- Write `\n`-delimited responses to stdout
- Session lifecycle: initialize → list_tools → call_tool → … → shutdown

**HTTP/SSE transport (feature `http`, enabled by default):**
- `axum::Router` with routes:
  - `GET /sse` — SSE endpoint, sends `endpoint: /messages?session_id=...`
  - `POST /messages` — receives JSON-RPC messages
  - `POST /mcp` — Smithery-compatible direct HTTP mode
- CORS wildcard
- Optional `ARCH_OPSD_API_KEY` bearer token auth

### Resources (`resources.rs`)
URI dispatcher pattern matching on URI prefix:
- `archwiki://{page}` → fetch wiki page
- `aur://{pkg}/pkgbuild` → fetch PKGBUILD
- `aur://{pkg}/info` → fetch AUR info JSON
- `archrepo://{pkg}` → fetch official repo info
- `pacman://installed`, `orphans`, `explicit`, ... → arch-only commands
- `system://info`, `disk`, `services/failed`, `logs/boot`, `health`
- `archnews://latest`, `critical`, `since-update`
- `mirrors://active`, `health`
- `config://pacman`, `makepkg`

### Prompts (`prompts.rs`)
8 prompt templates, same as Python. Each prompt handler:
1. Takes named arguments
2. Fetches live data via tool modules
3. Generates structured messages with `role: "user"` and `role: "assistant"` turns

## Module Details

### `pacman.rs` — 1559 Python → ~800 Rust
- `get_official_package_info()` — run `pacman -Si`, parse into struct. Fallback: HTTP GET archlinux.org API → serde deserialize
- `check_updates_dry_run()` — run `checkupdates`, parse output
- `remove_packages()` — run `pacman -R[s][dd]`, parse output
- Sub-command parsing via regex and line-by-line iteration

### `aur.rs` — 1259 Python → ~700 Rust
- `search_aur()` — HTTP GET `aur.archlinux.org/rpc/v5/search/...`, serde deserialize, smart ranking
- `audit_package_security()` — two sub-actions:
  - `analyze_pkgbuild_safety()` — regex-based scanner for 50+ red flag patterns
  - `analyze_package_metadata_risk()` — trust scoring from votes/popularity/maintainer
- Rate limiting via `tokio::sync::Semaphore` or simple `sleep(Duration::from_millis(100))`

### `wiki.rs` — 245 Python → ~150 Rust
- Try MediaWiki API JSON first (`action=parse`)
- Fallback: `scraper::Html` parse + manual Markdown extraction (no `markdownify` equivalent needed)
- Search via `action=opensearch`

### `config.rs` — 402 Python → ~200 Rust
- `analyze_pacman_conf()` — line-by-line parser for INI-style `/etc/pacman.conf`
- `analyze_makepkg_conf()` — simple shell variable extraction via regex
- No external INI parser needed — format is simple enough

### `btrfs.rs` — 945 Python → ~500 Rust
- All btrfs commands via `btrfs` CLI tool: `btrfs filesystem show`, `btrfs subvolume list`, `btrfs device stats`, etc
- Scrub management: `btrfs scrub status|cancel`, `systemctl start btrfs-scrub@...`
- Snapshot management via `snapper` CLI
- Output parsers using regex on structured command output

### `health.rs` — 190 Python → ~100 Rust
- Aggregate calls to system, packages, mirrors, news, btrfs modules
- Collect issues, suggestions, summary into structured report

### `report.rs` — 406 Python → ~250 Rust
- Markdown table generation with Russian locale headers
- Same structure as Python: "Программа | Что значит | Результат"

### `build_test.rs` — 620 Python → ~350 Rust
- RPi5 OS build validation: check files on `/boot`, run systemctl commands, parse package lists
- Security checks: grep sshd config, fail2ban status, sudoers parsing

### v0.10 Modules (7 files, ~1300 combined Python → ~700 Rust)
Each is a subprocess wrapper around CLI tools:
- `luks.rs` — `cryptsetup` commands
- `firewall.rs` — `nft` list rules / add rule
- `hardware.rs` — `vcgencmd` parsing
- `boot_config.rs` — read config.txt / cmdline.txt
- `backup.rs` — `btrbk` wrapper
- `recovery.rs` — systemctl, fsck, fstab checks
- `telegram.rs` — reqwest POST to Telegram Bot API

## Cross-Compilation Strategy

```toml
# cross.toml
[target.aarch64-unknown-linux-musl]
image = "rust:latest"  # cross includes musl toolchains
```

```bash
cross build --release --target aarch64-unknown-linux-musl
# Output: target/aarch64-unknown-linux-musl/release/arch-opsd
```

Static musl binary — no libc dependency, runs on any Linux kernel.

## Implementation Order (Phased)

1. **Phase 0 — Skeleton:** Cargo.toml, main.rs, protocol.rs, server.rs (STDIO only), platform.rs, error.rs, command.rs, client.rs. Hello-world MCP server.
2. **Phase 1 — Core tools:** wiki, aur, pacman, packages, files, system, news, mirrors, config (the data-fetching tools)
3. **Phase 2 — Monitoring:** health, logs, journal, report
4. **Phase 3 — Specialist:** btrfs, boot, build_test
5. **Phase 4 — v0.10:** luks, firewall, hardware, boot_config, backup, recovery, telegram
6. **Phase 5 — Resources + Prompts:** 38 resource handlers, 8 prompt generators
7. **Phase 6 — HTTP transport:** axum SSE, /mcp endpoint, Smithery compat
8. **Phase 7 — Polish:** error messages, wiki suggestions, user-facing quality

## Testing Strategy

- Unit tests for each module via `#[cfg(test)] mod tests {}`
- JSON-RPC protocol parsing/serialization tests
- Tool handler tests with mocked subprocess output
- Integration test binary that starts server, sends JSON-RPC, validates responses
- No CI in this repo (same as Python — tag-only releases)

## Security

- Same security audit as Python AUR module (50+ regex patterns)
- Static binary reduces supply-chain attack surface
- No `eval`, no dynamic imports, no shell injection via typed `Command::arg()`
