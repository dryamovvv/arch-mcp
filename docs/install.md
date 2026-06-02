# Installation

## Quick Install (pre-built binary)

```bash
# Download latest release (aarch64 or x86_64)
curl -fsSL https://github.com/dryamovvv/arch-mcp/releases/latest/download/arch-opsd-linux-aarch64 -o /usr/local/bin/arch-opsd
chmod +x /usr/local/bin/arch-opsd
```

## Build from Source

```bash
git clone https://github.com/dryamovvv/arch-mcp.git
cd arch-mcp
cargo build --release
# Binary: target/release/arch-opsd
```

## Cross-compile for RPi5 (aarch64)

```bash
# On x86_64 machine:
cargo install cross
cross build --release --target aarch64-unknown-linux-musl
# Binary: target/aarch64-unknown-linux-musl/release/arch-opsd
# Copy to RPi5 via scp
```

## systemd Service (optional)

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

## Usage

```bash
# STDIO mode (default for MCP clients)
arch-opsd stdio

# HTTP mode (with SSE-based MCP transport)
arch-opsd http          # listens on :8080
```

### Claude / Cursor / MCP clients

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

### SSH to remote machine

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

## Verification

```bash
arch-opsd stdio --help
# Or:
echo '{"id":1,"method":"initialize","params":{}}' | arch-opsd stdio
# Should respond with JSON-RPC capabilities
```
