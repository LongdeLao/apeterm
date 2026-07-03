import json
import sys
import threading
import time

import yfinance as yf


DEFAULT_SYMBOLS = ["AMZN", "AAPL", "META", "MSFT", "NVDA"]
SYMBOLS = [symbol.upper() for symbol in sys.argv[1:] if symbol.strip()] or DEFAULT_SYMBOLS
HEARTBEAT_SECONDS = 1.0

quotes = {}
quotes_lock = threading.Lock()
stdout_lock = threading.Lock()
market_state = "regular"


def number(value, default=0.0):
    try:
        return float(value)
    except (TypeError, ValueError):
        return default


def normalize_market_state(value):
    value = str(value or "").upper()
    if value in {"PRE", "PREPRE", "PRE_MARKET"}:
        return "pre_market"
    if value in {"REGULAR", "REGULAR_MARKET"}:
        return "regular"
    if value in {"POST", "POSTPOST", "AFTER_HOURS"}:
        return "after_hours"
    return "after_hours"


def symbol_series(frame, field, symbol):
    try:
        data = frame[field]
    except Exception:
        return None

    try:
        return data[symbol]
    except Exception:
        pass

    if symbol == SYMBOLS[0]:
        return data

    return None


def refresh_market_state():
    global market_state

    try:
        info = yf.Ticker(SYMBOLS[0]).info
        market_state = normalize_market_state(info.get("marketState"))
    except Exception:
        market_state = "after_hours"


def integer(value):
    try:
        if value is None:
            return None
        return int(float(value))
    except (TypeError, ValueError):
        return None


def emit_quote_line(symbol, price, change_percent, day_volume=None, avg_volume=None):
    if not symbol or price <= 0:
        return

    with stdout_lock:
        print(
            json.dumps(
                {
                    "symbol": symbol,
                    "price": price,
                    "price_change_percent": change_percent,
                    "day_volume": day_volume,
                    "avg_volume": avg_volume,
                    "market_state": market_state,
                },
                separators=(",", ":"),
            ),
            flush=True,
        )


def update_quote(symbol, price, change_percent, day_volume=None, avg_volume=None):
    if not symbol or price <= 0:
        return

    with quotes_lock:
        previous = quotes.get(symbol, {})
        quotes[symbol] = {
            "price": price,
            "price_change_percent": change_percent,
            "day_volume": day_volume if day_volume is not None else previous.get("day_volume"),
            "avg_volume": avg_volume if avg_volume is not None else previous.get("avg_volume"),
        }


def emit_cached_quotes():
    with quotes_lock:
        current_quotes = [(symbol, quotes.get(symbol)) for symbol in SYMBOLS]

    for symbol, quote in current_quotes:
        if quote:
            emit_quote_line(
                symbol,
                quote["price"],
                quote["price_change_percent"],
                quote.get("day_volume"),
                quote.get("avg_volume"),
            )


def start_heartbeat():
    def run():
        ticks = 0
        while True:
            time.sleep(HEARTBEAT_SECONDS)
            ticks += 1
            if ticks % 60 == 0:
                refresh_market_state()
            emit_cached_quotes()

    threading.Thread(target=run, daemon=True).start()


def emit_initial_snapshot():
    daily = yf.download(
        SYMBOLS,
        period="3mo",
        interval="1d",
        auto_adjust=False,
        progress=False,
        threads=True,
    )

    if daily.empty:
        return

    daily_closes = daily["Close"].dropna(how="all")
    if daily_closes.empty:
        return

    latest_daily = daily_closes.iloc[-1]
    previous_daily = daily_closes.iloc[-2] if len(daily_closes) > 1 else latest_daily

    for symbol in SYMBOLS:
        latest_close_series = symbol_series(daily, "Close", symbol)
        if latest_close_series is not None:
            cleaned_closes = latest_close_series.dropna()
            if cleaned_closes.empty:
                continue
            price = number(cleaned_closes.iloc[-1])
            previous_price = number(
                cleaned_closes.iloc[-2] if len(cleaned_closes) > 1 else cleaned_closes.iloc[-1]
            )
        else:
            price = number(latest_daily.get(symbol))
            previous_price = number(previous_daily.get(symbol))
        change_percent = 0.0
        if previous_price > 0:
            change_percent = ((price - previous_price) / previous_price) * 100
        day_volume = None
        avg_volume = None
        symbol_volumes = symbol_series(daily, "Volume", symbol)
        if symbol_volumes is not None:
            cleaned = symbol_volumes.dropna()
            if not cleaned.empty:
                day_volume = integer(cleaned.iloc[-1])
                avg_volume = integer(cleaned.tail(30).mean())

        update_quote(symbol, price, change_percent, day_volume, avg_volume)

    emit_cached_quotes()


def message_handler(message):
    symbol = message.get("id") or message.get("symbol")
    price = number(message.get("price") or message.get("regularMarketPrice"))
    change_percent = number(
        message.get("change_percent")
        or message.get("changePercent")
        or message.get("regularMarketChangePercent")
    )
    day_volume = integer(
        message.get("dayVolume")
        or message.get("regularMarketVolume")
        or message.get("volume")
    )

    update_quote(symbol, price, change_percent, day_volume)


def main():
    refresh_market_state()
    emit_initial_snapshot()
    start_heartbeat()

    with yf.WebSocket(verbose=False) as websocket:
        websocket.subscribe(SYMBOLS)
        websocket.listen(message_handler)


if __name__ == "__main__":
    main()
