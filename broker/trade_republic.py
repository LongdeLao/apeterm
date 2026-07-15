#!/usr/bin/env python3
"""Stable ApeTerm bridge around pytr's CLI.

Credentials and session cookies stay in pytr's own ~/.pytr directory. ApeTerm
only receives a normalized, read-only portfolio snapshot.
"""

from __future__ import annotations

import argparse
import asyncio
import base64
import hashlib
import json
import os
import platform
import subprocess
import sys
import time
from datetime import datetime, timezone
from decimal import Decimal, ROUND_HALF_UP
from pathlib import Path

import pytr.utils as pytr_utils
from pytr.api import BASE_DIR, CREDENTIALS_FILE, TradeRepublicApi, TradeRepublicError


pytr_utils.log_level = "critical"
WEB_APP_VERSION = "15.101.0"


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


def normalize_phone(phone: str) -> str:
    phone = "".join(phone.strip().split())
    if phone.startswith("00"):
        phone = f"+{phone[2:]}"
    if phone.startswith("0"):
        phone = f"+49{phone.lstrip('0')}"
    if not phone.startswith("+") or not phone[1:].isdigit():
        raise ValueError("Phone number must use international format, e.g. +4917612345678.")
    return phone


def trade_republic_error(response) -> RuntimeError:
    try:
        payload = response.json()
        error = (payload.get("errors") or [{}])[0]
        code = error.get("errorCode") or response.reason
        message = error.get("errorMessage") or code
        retry = (error.get("meta") or {}).get("nextAttemptInSeconds")
        suffix = f" Retry in {retry}s." if retry else ""
        detail = f"{message} ({code})" if message != code else code
    except Exception:
        body = response.text.strip()
        detail = body[:240] if body else response.reason
    suffix = suffix if "suffix" in locals() else ""
    return RuntimeError(f"Trade Republic login failed: HTTP {response.status_code}: {detail}.{suffix}")


def web_device_info(phone: str) -> str:
    seed = hashlib.sha256(f"apeterm:{phone}:{platform.node()}".encode()).hexdigest()
    payload = {
        "stableDeviceId": seed,
        "model": platform.machine() or "Desktop",
        "browser": "Chrome",
        "browserVersion": "146.0.0.0",
        "os": platform.system() or "Desktop",
        "osVersion": platform.release() or "",
        "timezone": time.tzname[0] if time.tzname else "UTC",
        "timezoneOffset": int(-time.timezone / 60),
        "screen": "1920x1080x24",
        "preferredLanguages": ["en-US", "en"],
        "numberOfCores": os.cpu_count() or 4,
        "deviceMemory": 8,
    }
    return base64.b64encode(json.dumps(payload, separators=(",", ":")).encode()).decode()


def prepare_web_login_session(tr: TradeRepublicApi, phone: str) -> dict[str, str]:
    if tr._waf_token == "awswaf":
        tr._waf_token = tr._fetch_waf_token_awswaf()
    elif tr._waf_token == "playwright":
        tr._waf_token = tr._fetch_waf_token_playwright()
    if tr._waf_token:
        tr._set_waf_cookie(tr._waf_token)
    headers = {
        "X-TR-Device-Info": web_device_info(phone),
        "X-TR-App-Version": WEB_APP_VERSION,
    }
    tr._websession.headers.update(headers)
    return headers


def post_v2_login(tr: TradeRepublicApi, phone: str, pin: str) -> dict:
    headers = prepare_web_login_session(tr, phone)
    response = tr._websession.post(
        f"{tr._host}/api/v2/auth/web/login",
        json={"phoneNumber": phone, "pin": pin},
        headers=headers,
    )
    if not response.ok:
        raise trade_republic_error(response)
    return response.json()


def get_v2_process(tr: TradeRepublicApi, process_id: str) -> dict:
    response = tr._websession.get(f"{tr._host}/api/v2/auth/web/login/processes/{process_id}")
    if not response.ok:
        raise trade_republic_error(response)
    return response.json()


def wait_for_v2_process(tr: TradeRepublicApi, process_id: str, countdown: int) -> bool:
    deadline = time.time() + max(15, min(countdown + 10, 180))
    while time.time() < deadline:
        process = get_v2_process(tr, process_id)
        status = process.get("status")
        if status in {"CONFIRMED", "COMPLETED"}:
            tr.save_websession()
            return True
        if process.get("requiredAction") == "AUTHENTICATOR_VERIFICATION":
            return False
        if status not in {None, "PENDING"}:
            raise RuntimeError(f"Trade Republic login was rejected ({status}).")
        time.sleep(2)
    raise RuntimeError("Trade Republic login confirmation timed out.")


def complete_v2_authenticator(tr: TradeRepublicApi, process_id: str, code: str) -> None:
    response = tr._websession.post(
        f"{tr._host}/api/v2/auth/web/login/processes/{process_id}/authenticator-verification",
        json={"code": code},
    )
    if not response.ok:
        raise trade_republic_error(response)
    if not wait_for_v2_process(tr, process_id, 120):
        raise RuntimeError("Trade Republic still requires app confirmation after the code.")


def login_start() -> int:
    payload = json.load(sys.stdin)
    phone = normalize_phone(payload["phone"])
    pin = payload["pin"].strip()
    tr = TradeRepublicApi(phone_no=phone, pin=pin, save_cookies=True, waf_token="playwright")
    if tr.resume_websession():
        write_credentials(phone, pin)
        print(json.dumps({"status": "connected"}))
        return 0
    try:
        result = post_v2_login(tr, phone, pin)
        process_id = result["processId"]
        countdown = int(result.get("countdownInSeconds") or 120)
        process = get_v2_process(tr, process_id)
        if process.get("requiredAction") == "AUTHENTICATOR_VERIFICATION":
            tr._websession.cookies.save(ignore_discard=True)
            print(
                json.dumps(
                    {
                        "status": "code_required",
                        "process_id": f"v2:{process_id}",
                        "countdown": countdown,
                    }
                )
            )
            return 0
        if wait_for_v2_process(tr, process_id, countdown):
            write_credentials(phone, pin)
            print(json.dumps({"status": "connected"}))
            return 0
    except RuntimeError:
        raise

    countdown = tr.initiate_weblogin()
    tr._websession.cookies.save(ignore_discard=True)
    print(
        json.dumps(
            {
                "status": "code_required",
                "process_id": f"v1:{tr._process_id}",
                "countdown": countdown,
            }
        )
    )
    return 0


def login_complete() -> int:
    payload = json.load(sys.stdin)
    phone = normalize_phone(payload["phone"])
    pin = payload["pin"].strip()
    process_id = payload["process_id"].strip()
    code = payload["code"].strip()
    tr = TradeRepublicApi(phone_no=phone, pin=pin, save_cookies=True, waf_token="playwright")
    tr._websession.cookies.load(ignore_discard=True)
    prepare_web_login_session(tr, phone)
    if process_id.startswith("v2:"):
        complete_v2_authenticator(tr, process_id.removeprefix("v2:"), code)
    else:
        tr._process_id = process_id.removeprefix("v1:")
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
