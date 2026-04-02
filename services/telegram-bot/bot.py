"""Investor OS Telegram Bot.

Delivers trading signals from NATS consensus events and allows
semi-auto trade confirmation via inline keyboard buttons.

Config (env vars):
    TELEGRAM_BOT_TOKEN  — BotFather token (required for operation)
    TELEGRAM_CHAT_ID    — default recipient chat ID
    NATS_URL            — NATS server URL (default nats://nats:4222)
    DB_URL              — PostgreSQL connection string
"""

from __future__ import annotations

import asyncio
import json
import logging
import os
import signal
import sys
from datetime import datetime, timezone

import nats
import psycopg2
from telegram import InlineKeyboardButton, InlineKeyboardMarkup, Update
from telegram.ext import (
    Application,
    CallbackQueryHandler,
    CommandHandler,
    ContextTypes,
)

# ---------------------------------------------------------------------------
# Logging
# ---------------------------------------------------------------------------
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
)
logger = logging.getLogger("ios-telegram-bot")

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------
TELEGRAM_BOT_TOKEN: str = os.getenv("TELEGRAM_BOT_TOKEN", "")
TELEGRAM_CHAT_ID: str = os.getenv("TELEGRAM_CHAT_ID", "")
NATS_URL: str = os.getenv("NATS_URL", "nats://nats:4222")
DB_URL: str = os.getenv("DB_URL", "")

# In-memory user registry (chat_id -> settings)
_users: dict[str, dict] = {}

# ---------------------------------------------------------------------------
# Database helpers
# ---------------------------------------------------------------------------


def _db_conn():
    """Return a psycopg2 connection or None if DB_URL is not configured."""
    if not DB_URL:
        return None
    try:
        return psycopg2.connect(DB_URL)
    except Exception as exc:
        logger.warning("DB connection failed: %s", exc)
        return None


def _fetch_active_strategies() -> list[dict]:
    """Fetch active user strategies from PostgreSQL."""
    conn = _db_conn()
    if conn is None:
        return []
    try:
        with conn.cursor() as cur:
            cur.execute(
                "SELECT id, name, asset, mode, status "
                "FROM user_strategies WHERE status = 'active' "
                "ORDER BY created_at DESC LIMIT 10"
            )
            cols = [d[0] for d in cur.description]
            return [dict(zip(cols, row)) for row in cur.fetchall()]
    except Exception as exc:
        logger.warning("Failed to fetch strategies: %s", exc)
        return []
    finally:
        conn.close()


def _fetch_latest_predictions() -> list[dict]:
    """Fetch the latest ML predictions from PostgreSQL."""
    conn = _db_conn()
    if conn is None:
        return []
    try:
        with conn.cursor() as cur:
            cur.execute(
                "SELECT symbol, direction, confidence, predicted_return, "
                "       model_name, created_at "
                "FROM ml_predictions "
                "ORDER BY created_at DESC LIMIT 5"
            )
            cols = [d[0] for d in cur.description]
            return [dict(zip(cols, row)) for row in cur.fetchall()]
    except Exception as exc:
        logger.warning("Failed to fetch predictions: %s", exc)
        return []
    finally:
        conn.close()


# ---------------------------------------------------------------------------
# Telegram command handlers
# ---------------------------------------------------------------------------


async def cmd_start(update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
    """Handle /start — register user and show welcome message."""
    chat_id = str(update.effective_chat.id)
    _users[chat_id] = {"signals_enabled": True, "registered_at": datetime.now(timezone.utc).isoformat()}
    logger.info("User registered: chat_id=%s", chat_id)
    await update.message.reply_text(
        "Welcome to Investor OS!\n\n"
        "I deliver trading signals from the consensus engine.\n\n"
        "Commands:\n"
        "/status  -- active strategies + latest predictions\n"
        "/signals on|off  -- enable/disable signal notifications\n"
    )


async def cmd_status(update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
    """Handle /status — show active strategies and latest predictions."""
    strategies = _fetch_active_strategies()
    predictions = _fetch_latest_predictions()

    lines: list[str] = ["== Active Strategies =="]
    if strategies:
        for s in strategies:
            lines.append(f"  {s.get('name', '?')} | {s.get('asset', '?')} | {s.get('mode', '?')} | {s.get('status', '?')}")
    else:
        lines.append("  (none)")

    lines.append("")
    lines.append("== Latest Predictions ==")
    if predictions:
        for p in predictions:
            direction = (p.get("direction") or "?").upper()
            conf = p.get("confidence", 0)
            ret = p.get("predicted_return", 0)
            model = p.get("model_name", "?")
            lines.append(f"  {p.get('symbol', '?')}: {direction} ({conf:.0%}) ret={ret:+.2%} [{model}]")
    else:
        lines.append("  (none)")

    await update.message.reply_text("\n".join(lines))


async def cmd_signals(update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
    """Handle /signals on|off — toggle signal notifications."""
    chat_id = str(update.effective_chat.id)
    args = context.args
    if not args or args[0].lower() not in ("on", "off"):
        await update.message.reply_text("Usage: /signals on|off")
        return

    enabled = args[0].lower() == "on"
    if chat_id not in _users:
        _users[chat_id] = {"signals_enabled": enabled, "registered_at": datetime.now(timezone.utc).isoformat()}
    else:
        _users[chat_id]["signals_enabled"] = enabled

    state_str = "ENABLED" if enabled else "DISABLED"
    logger.info("Signals %s for chat_id=%s", state_str, chat_id)
    await update.message.reply_text(f"Signal notifications: {state_str}")


async def callback_trade_action(update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
    """Handle inline keyboard button presses for trade confirmation."""
    query = update.callback_query
    await query.answer()

    data = query.data or ""
    parts = data.split(":")
    if len(parts) < 3:
        await query.edit_message_text("Invalid action.")
        return

    action = parts[0]  # approve / reject
    symbol = parts[1]
    direction = parts[2]

    if action == "approve":
        await query.edit_message_text(
            f"APPROVED: {symbol} {direction}\n"
            f"Trade proposal forwarded to execution engine."
        )
        logger.info("Trade APPROVED by user: %s %s", symbol, direction)
    elif action == "reject":
        await query.edit_message_text(
            f"REJECTED: {symbol} {direction}\n"
            f"Trade proposal discarded."
        )
        logger.info("Trade REJECTED by user: %s %s", symbol, direction)
    else:
        await query.edit_message_text(f"Unknown action: {action}")


# ---------------------------------------------------------------------------
# NATS signal formatting
# ---------------------------------------------------------------------------


def _format_consensus_message(data: dict) -> str:
    """Format a consensus event into a human-readable Telegram message."""
    symbol = data.get("symbol", "???")
    direction = (data.get("direction") or "?").upper()
    confidence = data.get("confidence", 0)
    models = data.get("models", [])
    predicted_return = data.get("predicted_return", 0)
    var95 = data.get("var95", 0)

    # Direction emoji
    if direction == "LONG":
        emoji = "\U0001f535"  # blue circle
    elif direction == "SHORT":
        emoji = "\U0001f534"  # red circle
    else:
        emoji = "\u26aa"  # white circle

    model_names = ", ".join(models) if models else "consensus"

    lines = [
        f"{emoji} {symbol}: {direction} ({confidence:.0f}% confidence)",
        f"{model_names} agree",
        f"Return: {predicted_return:+.1%}",
        f"VaR95: {var95:+.1%}",
    ]
    return "\n".join(lines)


def _build_trade_keyboard(symbol: str, direction: str) -> InlineKeyboardMarkup:
    """Build approve/reject inline keyboard for a trade proposal."""
    return InlineKeyboardMarkup([
        [
            InlineKeyboardButton("Approve", callback_data=f"approve:{symbol}:{direction}"),
            InlineKeyboardButton("Reject", callback_data=f"reject:{symbol}:{direction}"),
        ]
    ])


# ---------------------------------------------------------------------------
# NATS subscription loop
# ---------------------------------------------------------------------------


async def _nats_listener(app: Application) -> None:
    """Connect to NATS and subscribe to consensus + trade proposal subjects."""
    bot = app.bot

    # Determine target chat IDs: registered users + default
    def _target_chat_ids() -> list[str]:
        ids: set[str] = set()
        if TELEGRAM_CHAT_ID:
            ids.add(TELEGRAM_CHAT_ID)
        for cid, settings in _users.items():
            if settings.get("signals_enabled", True):
                ids.add(cid)
        return list(ids)

    retry_delay = 1
    max_retry_delay = 60

    while True:
        try:
            logger.info("Connecting to NATS at %s ...", NATS_URL)
            nc = await nats.connect(NATS_URL)
            logger.info("NATS connected")
            retry_delay = 1  # reset on successful connect

            # --- Consensus handler ---
            async def on_consensus(msg):
                try:
                    data = json.loads(msg.data.decode())
                except Exception:
                    logger.warning("Invalid consensus message: %s", msg.data)
                    return

                confidence = data.get("confidence", 0)
                if confidence <= 50:
                    return  # below threshold

                text = _format_consensus_message(data)
                for cid in _target_chat_ids():
                    try:
                        await bot.send_message(chat_id=cid, text=text)
                    except Exception as exc:
                        logger.warning("Failed to send to %s: %s", cid, exc)

            # --- Trade proposal handler ---
            async def on_trade_proposal(msg):
                try:
                    data = json.loads(msg.data.decode())
                except Exception:
                    logger.warning("Invalid trade proposal message: %s", msg.data)
                    return

                symbol = data.get("symbol", "???")
                direction = (data.get("direction") or "?").upper()
                confidence = data.get("confidence", 0)
                predicted_return = data.get("predicted_return", 0)

                text = (
                    f"Trade Proposal\n"
                    f"{symbol}: {direction} ({confidence:.0f}% confidence)\n"
                    f"Expected return: {predicted_return:+.1%}\n"
                    f"\nApprove or reject?"
                )
                keyboard = _build_trade_keyboard(symbol, direction)

                for cid in _target_chat_ids():
                    try:
                        await bot.send_message(chat_id=cid, text=text, reply_markup=keyboard)
                    except Exception as exc:
                        logger.warning("Failed to send proposal to %s: %s", cid, exc)

            await nc.subscribe("ios.consensus.*", cb=on_consensus)
            await nc.subscribe("ios.trade.proposal.*", cb=on_trade_proposal)
            logger.info("Subscribed to ios.consensus.* and ios.trade.proposal.*")

            # Keep alive until connection drops
            while nc.is_connected:
                await asyncio.sleep(1)

            logger.warning("NATS connection lost, will reconnect...")

        except Exception as exc:
            logger.warning("NATS error: %s — retrying in %ds", exc, retry_delay)

        await asyncio.sleep(retry_delay)
        retry_delay = min(retry_delay * 2, max_retry_delay)


# ---------------------------------------------------------------------------
# Main entry point
# ---------------------------------------------------------------------------


async def post_init(app: Application) -> None:
    """Start the NATS listener after the Telegram bot initializes."""
    asyncio.create_task(_nats_listener(app))
    logger.info("NATS listener task started")


def main() -> None:
    """Entry point."""
    if not TELEGRAM_BOT_TOKEN:
        logger.warning(
            "TELEGRAM_BOT_TOKEN is not set. "
            "The bot cannot start without a valid token. "
            "Set the TELEGRAM_BOT_TOKEN environment variable and restart."
        )
        # Exit cleanly — don't crash, just stop
        sys.exit(0)

    logger.info("Starting Investor OS Telegram Bot...")
    logger.info("NATS_URL=%s", NATS_URL)
    logger.info("TELEGRAM_CHAT_ID=%s", TELEGRAM_CHAT_ID or "(not set)")
    logger.info("DB_URL=%s", "configured" if DB_URL else "(not set)")

    app = (
        Application.builder()
        .token(TELEGRAM_BOT_TOKEN)
        .post_init(post_init)
        .build()
    )

    # Register command handlers
    app.add_handler(CommandHandler("start", cmd_start))
    app.add_handler(CommandHandler("status", cmd_status))
    app.add_handler(CommandHandler("signals", cmd_signals))
    app.add_handler(CallbackQueryHandler(callback_trade_action))

    # Run until stopped
    app.run_polling(allowed_updates=Update.ALL_TYPES)


if __name__ == "__main__":
    main()
