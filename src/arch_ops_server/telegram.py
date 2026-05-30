# SPDX-License-Identifier: GPL-3.0-only OR MIT
"""
Telegram LUKS unlock management module.
Manages Telegram-bot-based LUKS unlock in initramfs.
"""

import logging
from typing import Dict, Any, Optional

from .utils import (
    IS_ARCH,
    run_command,
    create_error_response,
    check_command_exists,
)

logger = logging.getLogger(__name__)


async def manage_telegram_unlock(
    action: str,
    token: Optional[str] = None,
    chat_id: Optional[str] = None,
    password: Optional[str] = None,
) -> Dict[str, Any]:
    """
    Manage Telegram-based LUKS unlock for initramfs.

    Actions:
      status      — check if telegram-unlock hook is in initramfs
      test_bot    — test Telegram Bot API connectivity
      send_unlock — send LUKS password through Telegram bot
    """
    if not IS_ARCH:
        return create_error_response(
            "NotSupported", "This feature is only available on Arch Linux."
        )

    if action == "status":
        logger.info("Checking telegram-unlock status")

        result = {"hook_in_initramfs": False, "hook_files": []}

        import os

        hook_dir = "/etc/initcpio/hooks/"
        if os.path.isdir(hook_dir):
            for f in os.listdir(hook_dir):
                if "telegram" in f.lower() or "unlock" in f.lower():
                    result["hook_files"].append(os.path.join(hook_dir, f))

        install_dir = "/etc/initcpio/install/"
        if os.path.isdir(install_dir):
            for f in os.listdir(install_dir):
                if "telegram" in f.lower() or "unlock" in f.lower():
                    result["hook_files"].append(os.path.join(install_dir, f))

        if result["hook_files"]:
            result["hook_in_initramfs"] = True

        # Check initramfs for the hook
        exit_code, out, err = await run_command(
            ["lsinitcpio", "/boot/initramfs-linux.img"],
            timeout=10,
            check=False,
        )
        if exit_code == 0:
            for line in out.strip().splitlines():
                if "telegram" in line.lower() or "unlock" in line.lower():
                    if "hooks" not in result:
                        result["hooks"] = []
                    result["hooks"].append(line.strip())

        return result

    elif action == "test_bot":
        if not token or not chat_id:
            return create_error_response(
                "MissingArgument", "token and chat_id are required for test_bot"
            )
        logger.info("Testing Telegram bot connectivity")

        exit_code, out, err = await run_command(
            ["curl", "-s", "-m", "10", f"https://api.telegram.org/bot{token}/getMe"],
            timeout=15,
            check=False,
        )
        if exit_code != 0 or "ok" not in out.lower():
            return create_error_response(
                "NetworkError", f"Failed to connect to Telegram Bot API: {err or out}"
            )

        exit_code2, out2, err2 = await run_command(
            [
                "curl",
                "-s",
                "-m",
                "10",
                f"https://api.telegram.org/bot{token}/sendMessage",
                "-d",
                f"chat_id={chat_id}",
                "-d",
                "text=arch-mcp connectivity test",
            ],
            timeout=15,
            check=False,
        )

        bot_ok = exit_code == 0 and "ok" in out.lower() and '"ok":true' in out
        msg_ok = exit_code2 == 0 and "ok" in out2.lower() and '"ok":true' in out2

        return {
            "bot_reachable": bot_ok,
            "message_sent": msg_ok,
            "bot_info": out.strip(),
            "message_result": out2.strip(),
        }

    elif action == "send_unlock":
        if not token or not chat_id or not password:
            return create_error_response(
                "MissingArgument",
                "token, chat_id, and password are required for send_unlock",
            )
        logger.info("Sending unlock password via Telegram bot")

        exit_code, out, err = await run_command(
            [
                "curl",
                "-s",
                "-m",
                "10",
                f"https://api.telegram.org/bot{token}/sendMessage",
                "-d",
                f"chat_id={chat_id}",
                "-d",
                f"text={password}",
            ],
            timeout=15,
            check=False,
        )

        success = exit_code == 0 and "ok" in out.lower() and '"ok":true' in out
        return {
            "ok": success,
            "result": out.strip(),
            "message": "Unlock password sent via Telegram"
            if success
            else f"Failed to send: {out.strip()}",
        }

    else:
        return create_error_response(
            "InvalidAction",
            f"Unknown action: '{action}'. Valid actions: status, test_bot, send_unlock.",
        )
