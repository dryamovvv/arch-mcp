---
description: Comprehensive Arch Linux system audit using all arch-linux MCP tools
agent: general
subtask: true
---

Load the `arch-system` skill and perform a comprehensive system audit of the remote RPi5 using the `arch-linux` MCP server.

## Workflow

1. Start with `generate_report(action='full')` for a quick bare-table overview.
2. Run additional tools from `arch-system` Batch 1 (skip those already covered by `generate_report`).
3. If issues are found, run `arch-system` Batch 2 tools for deeper diagnostics.
4. Compile results into a structured report in `arch-audit-results.md`.
5. Provide a summary of critical issues found and prioritize fixes.
