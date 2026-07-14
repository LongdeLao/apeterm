#!/usr/bin/env python3
"""Stable ApeTerm bridge around pytr's CLI.

Credentials and session cookies stay in pytr's own ~/.pytr directory. ApeTerm
only receives a normalized, read-only portfolio snapshot.
"""

from __future__ import annotations

import argparse
import asyncio
import json
import os
import subprocess
import sys
from datetime import datetime, timezone
from decimal import Decimal, ROUND_HALF_UP
from pathlib import Path

import pytr.utils as pytr_utils
from pytr.api import BASE_DIR, CREDENTIALS_FILE, TradeRepublicApi, TradeRepublicError


pytr_utils.log_level = "critical"


def ensure_venv_path() -> None:
    """pytr shells out to `playwright`; expose the private venv scripts dir."""
    scripts_dir = Path(sys.executable).parent
    current_path = os.environ.get("PATH", "")
    os.environ["PATH"] = f"{scripts_dir}{os.pathsep}{current_path}" if current_path else str(scripts_dir)


def pytr(*args: str, capture: bool = False) -> subprocess.CompletedProcess[str]:
    ensure_venv_path()
    return subprocess.run(
        [sys.executable, "-m", "pytr", *args],
        check=False,
        text=True,
        capture_output=capture,
    )


def connect() -> int:
    return pytr("login", "--store_credentials").returncode


def write_credentials(phone: str, pin: str) -> None:
    BASE_DIR.mkdir(parents=True, exist_ok=True)
    CREDENTIALS_FILE.write_text(f"{phone}\n{pin}\n", encoding="utf-8")


def login_start() -> int:
    payload = json.load(sys.stdin)
    phone = payload["phone"].strip()
    pin = payload["pin"].strip()
    tr = TradeRepublicApi(phone_no=phone, pin=pin, save_cookies=True, waf_token="playwright")
    if tr.resume_websession():
        write_credentials(phone, pin)
        print(json.dumps({"status": "connected"}))
        return 0
    countdown = tr.initiate_weblogin()
    tr._websession.cookies.save(ignore_discard=True)
    print(
        json.dumps(
            {
                "status": "code_required",
                "process_id": tr._process_id,
                "countdown": countdown,
            }
        )
    )
    return 0


def login_complete() -> int:
    payload = json.load(sys.stdin)
    phone = payload["phone"].strip()
    pin = payload["pin"].strip()
    process_id = payload["process_id"].strip()
    code = payload["code"].strip()
    tr = TradeRepublicApi(phone_no=phone, pin=pin, save_cookies=True, waf_token="playwright")
    tr._websession.cookies.load(ignore_discard=True)
    tr._process_id = process_id
    tr.complete_weblogin(code)
    write_credentials(phone, pin)
    print(json.dumps({"status": "connected"}))
    return 0


def parse_number(value: str) -> float:
    value = value.strip().replace("\u00a0", "")
    return float(value) if value else 0.0


def decimal_value(value: object) -> Decimal:
    if value is None:
        return Decimal("0")
    return Decimal(str(value))


async def receive_subscription(tr: TradeRepublicApi, subscription_id: str) -> object:
    while True:
        response_id, subscription, response = await tr.recv()
        if response_id == subscription_id:
            await tr.unsubscribe(response_id)
            return response
        await tr.unsubscribe(response_id)


def securities_account_number(tr: TradeRepublicApi) -> str:
    value = tr.settings().get("securitiesAccountNumber")
    if not value:
        raise RuntimeError("Trade Republic securities account number was not available.")
    return str(value)


async def portfolio_positions(tr: TradeRepublicApi) -> list[dict]:
    """Read portfolio positions using Trade Republic's current web topic."""
    sec_acc_no = securities_account_number(tr)
    subscription_attempts = [
        {"type": "compactPortfolioByType", "secAccNo": sec_acc_no},
        {"type": "compactPortfolio", "secAccNo": sec_acc_no},
        {"type": "compactPortfolio"},
    ]
    last_error: Exception | None = None
    portfolio_response = None
    for payload in subscription_attempts:
        try:
            subscription_id = await tr.subscribe(payload)
            portfolio_response = await receive_subscription(tr, subscription_id)
            break
        except TradeRepublicError as error:
            last_error = error
            if "BAD_SUBSCRIPTION_TYPE" not in str(error):
                raise
    if portfolio_response is None:
        raise RuntimeError(f"Trade Republic portfolio sync failed: {last_error}")

    if isinstance(portfolio_response, dict) and isinstance(portfolio_response.get("categories"), list):
        return [
            {**position, "categoryType": category.get("categoryType")}
            for category in portfolio_response["categories"]
            for position in category.get("positions", [])
            if isinstance(position, dict)
        ]
    if isinstance(portfolio_response, dict) and isinstance(portfolio_response.get("positions"), list):
        return [position for position in portfolio_response["positions"] if isinstance(position, dict)]
    if isinstance(portfolio_response, list):
        return [position for position in portfolio_response if isinstance(position, dict)]
    return []


async def fetch_portfolio_snapshot() -> dict:
    tr = TradeRepublicApi(save_cookies=True)
    if not tr.resume_websession():
        raise RuntimeError("Trade Republic session expired. Open Portfolio and press c to connect.")

    raw_positions = await portfolio_positions(tr)
    cash_id = await tr.cash()
    cash_response = await receive_subscription(tr, cash_id)

    positions = []
    for raw in raw_positions or []:
        isin = raw.get("instrumentId") or raw.get("isin") or raw.get("ISIN") or ""
        quantity = decimal_value(raw.get("netSize") or raw.get("quantity"))
        average_cost = decimal_value(raw.get("averageBuyIn") or raw.get("avgCost"))
        price = decimal_value(raw.get("price"))
        name = raw.get("name") or raw.get("shortName") or isin or "Unknown"
        exchange_ids = raw.get("exchangeIds") or []

        if isin and (not name or name == isin or not exchange_ids):
            try:
                details_id = await tr.instrument_details(isin)
                details = await receive_subscription(tr, details_id)
                name = details.get("shortName") or details.get("name") or name
                exchange_ids = details.get("exchangeIds") or exchange_ids
            except Exception:
                pass

        if isin and price == 0 and exchange_ids:
            try:
                ticker_id = await tr.ticker(isin, exchange=exchange_ids[0])
                ticker = await receive_subscription(tr, ticker_id)
                price = decimal_value((ticker.get("last") or {}).get("price"))
            except Exception:
                pass

        net_value = decimal_value(raw.get("netValue"))
        if net_value == 0:
            net_value = (price * quantity).quantize(Decimal("0.01"), rounding=ROUND_HALF_UP)

        positions.append(
            {
                "name": name,
                "isin": isin,
                "symbol": None,
                "quantity": float(quantity),
                "price": float(price),
                "average_cost": float(average_cost),
                "net_value": float(net_value),
                "cost_value": float((quantity * average_cost).quantize(Decimal("0.01"), rounding=ROUND_HALF_UP)),
            }
        )

    await tr.close()

    cash_item = cash_response[0] if isinstance(cash_response, list) and cash_response else {}
    currency = cash_item.get("currencyId", "EUR") if isinstance(cash_item, dict) else "EUR"
    cash = float(decimal_value(cash_item.get("amount") if isinstance(cash_item, dict) else 0))
    return {
        "broker": "trade_republic",
        "currency": currency,
        "cash": cash,
        "synced_at": datetime.now(timezone.utc).isoformat(),
        "positions": positions,
    }


def sync(output: Path) -> int:
    snapshot = asyncio.run(fetch_portfolio_snapshot())
    output.parent.mkdir(parents=True, exist_ok=True)
    temporary = output.with_suffix(output.suffix + ".tmp")
    temporary.write_text(json.dumps(snapshot, indent=2), encoding="utf-8")
    temporary.replace(output)
    print(json.dumps({"positions": len(snapshot["positions"]), "output": str(output)}))
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)
    subparsers.add_parser("connect")
    subparsers.add_parser("login-start")
    subparsers.add_parser("login-complete")
    sync_parser = subparsers.add_parser("sync")
    sync_parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.command == "connect":
        return connect()
    if args.command == "login-start":
        return login_start()
    if args.command == "login-complete":
        return login_complete()
    return sync(args.output)


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as exc:
        sys.stderr.write(f"{exc}\n")
        raise SystemExit(1)
