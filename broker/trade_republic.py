#!/usr/bin/env python3
"""Stable ApeTerm bridge around pytr's CLI.

Credentials and session cookies stay in pytr's own ~/.pytr directory. ApeTerm
only receives a normalized, read-only portfolio snapshot.
"""

from __future__ import annotations

import argparse
import csv
import json
import os
import re
import subprocess
import sys
import tempfile
from datetime import datetime, timezone
from pathlib import Path


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


def parse_number(value: str) -> float:
    value = value.strip().replace("\u00a0", "")
    return float(value) if value else 0.0


def sync(output: Path) -> int:
    with tempfile.TemporaryDirectory(prefix="apeterm-pytr-") as directory:
        csv_path = Path(directory) / "portfolio.csv"
        result = pytr(
            "portfolio",
            "--output",
            str(csv_path),
            "--no-decimal-localization",
            capture=True,
        )
        if result.returncode != 0:
            sys.stderr.write(result.stderr or result.stdout)
            return result.returncode

        positions = []
        with csv_path.open(newline="", encoding="utf-8") as handle:
            for row in csv.DictReader(handle, delimiter=";"):
                quantity = parse_number(row.get("quantity", "0"))
                price = parse_number(row.get("price", "0"))
                average_cost = parse_number(row.get("avgCost", "0"))
                net_value = parse_number(row.get("netValue", "0"))
                positions.append(
                    {
                        "name": row.get("Name", "Unknown"),
                        "isin": row.get("ISIN", ""),
                        "symbol": None,
                        "quantity": quantity,
                        "price": price,
                        "average_cost": average_cost,
                        "net_value": net_value,
                        "cost_value": quantity * average_cost,
                    }
                )

        cash_match = re.search(r"^Cash\s+(\S+)\s+(-?[0-9.]+)\s*$", result.stdout, re.MULTILINE)
        currency = cash_match.group(1) if cash_match else "EUR"
        cash = float(cash_match.group(2)) if cash_match else 0.0
        snapshot = {
            "broker": "trade_republic",
            "currency": currency,
            "cash": cash,
            "synced_at": datetime.now(timezone.utc).isoformat(),
            "positions": positions,
        }
        output.parent.mkdir(parents=True, exist_ok=True)
        temporary = output.with_suffix(output.suffix + ".tmp")
        temporary.write_text(json.dumps(snapshot, indent=2), encoding="utf-8")
        temporary.replace(output)
        print(json.dumps({"positions": len(positions), "output": str(output)}))
        return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)
    subparsers.add_parser("connect")
    sync_parser = subparsers.add_parser("sync")
    sync_parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    return connect() if args.command == "connect" else sync(args.output)


if __name__ == "__main__":
    raise SystemExit(main())
