---
name: add-arch-tool
description: 'Adds a new feature to the arch-mcp server — from design to merge.  Use when the user wants to extend the server with a new MCP tool:  monitoring, diagnostics, config, packages, systemd.'
---

# You are adding a new MCP tool to `arch-ops-server`. Follow the full cycle below.

## Stage 1: Design

1. **Understand what the command does.** E.g., `systemd-analyze` returns boot time.
   Check docs: `man systemd-analyze` or `systemd-analyze --help`.

2. **Pick the right module:**
   - System info/diagnostics → `system.py`
   - Packages/pacman → `pacman.py`
   - AUR → `aur.py`
   - Logs/systemd → `journal.py`
   - Configs → `config.py`
   - Groups → `groups.py`

3. **Verify on real hardware (RPi5) that the command works.** Use `arch-linux` MCP:
   ```
   arch-linux_manage_logs: unit='ssh', lines=3      # check server is alive
   ```
   Then `bash` with `ssh rpi5 "systemd-analyze"` or directly via MCP tools.

## Stage 2: Backend (`src/arch_ops_server/<module>.py`)

Create or extend a function following existing patterns:

```python
async def get_boot_time() -> Dict[str, Any]:
    """
    Get system boot time from systemd-analyze.

    Returns:
        Dict with boot time info
    """
    logger.info("Getting boot time via systemd-analyze")

    try:
        if not check_command_exists("systemd-analyze"):
            return create_error_response(
                "NotSupported",
                "systemd-analyze not available"
            )

        exit_code, stdout, stderr = await run_command(
            ["systemd-analyze"],
            timeout=10,
            check=False
        )

        if exit_code != 0:
            return create_error_response(
                "CommandError",
                f"systemd-analyze failed: {stderr}"
            )

        result = {
            "raw_output": stdout.strip(),
            "execution_ok": True
        }

        logger.info(f"Boot time: {stdout.strip()}")
        return result

    except Exception as e:
        logger.error(f"Failed to get boot time: {e}")
        return create_error_response(
            "Error",
            f"Failed to get boot time: {str(e)}"
        )
```

**Key conventions:**
- All functions are `async` — use `await run_command(...)`
- `run_command(...)` returns a tuple `(exit_code, stdout, stderr)`
- Always `check=False` — handle errors yourself via `exit_code`
- Always specify a `timeout` (5-15 sec depending on the command)
- Use `create_error_response("Type", "message")` for structured errors
- Use `check_command_exists(...)` to verify the binary is available
- Functions return `Dict[str, Any]`
- Log via `logger.info(...)` / `logger.error(...)`
- Imports from `.utils`: `IS_ARCH`, `run_command`, `create_error_response`, `check_command_exists`

## Stage 3: Server registration (`server.py`)

Three edit points:

### 3a. Import the function (top of file, ~line 50)
```python
from .system import (  # or other module
    get_system_info,
    analyze_storage,
    diagnose_system,
    get_boot_time,        # <-- add
)
```

### 3b. Tool definition (in the tools list, ~line 890+)
```python
Tool(
    name="get_boot_time",
    description="[MONITORING] Get system boot time via systemd-analyze. Shows kernel, initrd, and userspace startup durations. Works on systemd-based systems.",
    inputSchema={
        "type": "object",
        "properties": {}
    },
    annotations=ToolAnnotations(readOnlyHint=True)
),
```

Category prefixes for descriptions: `[DISCOVERY]`, `[LIFECYCLE]`, `[MAINTENANCE]`, `[ORGANIZATION]`, `[SECURITY]`, `[MONITORING]`, `[HISTORY]`, `[MIRRORS]`, `[CONFIG]`.

### 3c. Call handler (in `call_tool`, ~line 1250+)
```python
elif name == "get_boot_time":
    result = await get_boot_time()
```

## Stage 4: Metadata (`tool_metadata.py`)

Add an entry to `TOOL_METADATA`:
```python
"get_boot_time": ToolMetadata(
    name="get_boot_time",
    category="monitoring",
    platform="systemd",
    permission="read",
    workflow="diagnose",
    related_tools=["get_system_info", "diagnose_system"],
    prerequisite_tools=[]
),
```

Categories: `discovery`, `lifecycle`, `maintenance`, `organization`, `security`, `monitoring`, `history`, `mirrors`, `config`.
Platforms: `any`, `arch`, `systemd`.
Permissions: `read`, `write`.

## Stage 5: Tests (`tests/test_system.py`)

Add tests mocking the subprocess. Pattern:

```python
@pytest.mark.asyncio
async def test_get_boot_time(mock_subprocess_success):
    """Test getting boot time from systemd-analyze."""
    mock_subprocess_success.return_value = (0, "Startup finished in 2.3s", "")
    result = await get_boot_time()
    assert result["execution_ok"] is True
    assert "2.3s" in result["raw_output"]


@pytest.mark.asyncio
async def test_get_boot_time_not_available(monkeypatch):
    """Test when systemd-analyze is not installed."""
    monkeypatch.setattr(
        "arch_ops_server.system.check_command_exists",
        lambda x: False
    )
    result = await get_boot_time()
    assert result.get("error") is not None
```

Fixtures are defined in `conftest.py` — `mock_subprocess_success`, `mock_subprocess_failure`, etc.

## Stage 6: Real hardware verification

1. **Install the package in dev mode with new code:**
   ```bash
   uv pip install -e ".[dev]"
   ```

2. **Run tests locally:** `pytest tests/ -k "boot_time"`
   Make sure tests pass.

3. **Deploy code to RPi5 and test via MCP.**
   Use `arch-linux_*` tools to call the new function remotely.
   Verify real data is returned without errors.

4. **Check edge cases:**
   - Command not available (non-systemd system)
   - Command timeout
   - Unexpected output format

## Stage 7: Documentation

- Update the tool list in `README.md` if there is a tools table.
- Update `AGENTS.md` if the new feature changes architectural conventions.
- Do NOT over-document — only add what actually helps.

## Stage 8: Commit

```bash
git add src/ tests/ README.md
git commit -m "feat: add get_boot_time tool (systemd-analyze)"
```

Format: `type: brief description`.
Types: `feat`, `fix`, `test`, `docs`, `refactor`, `chore`.

## After completion

Tell the user:
- What was added (tool name, where it lives)
- Test results (passed/failed)
- RPi5 verification result (works/failed)
- Commit link
