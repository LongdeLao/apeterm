import json
import logging
import re
import sys
import time
import urllib.parse
import urllib.request

import yfinance as yf


logging.getLogger("yfinance").setLevel(logging.CRITICAL)


def number(value):
    try:
        if value is None:
            return None
        return float(value)
    except (TypeError, ValueError):
        return None


def pick(mapping, names):
    for name in names:
        try:
            value = mapping.get(name)
        except AttributeError:
            value = None
        if value is not None:
            return value
    return None


def two_sentence_summary(text):
    if not text:
        return None
    sentences = re.split(r"(?<=[.!?])\s+", text.strip())
    return " ".join(sentences[:2])


def translate_to_german(text):
    if not text:
        return None

    try:
        from deep_translator import GoogleTranslator

        translated = GoogleTranslator(source="auto", target="de").translate(text)
        return translated or None
    except Exception:
        pass

    try:
        query = urllib.parse.urlencode(
            {
                "client": "gtx",
                "sl": "auto",
                "tl": "de",
                "dt": "t",
                "q": text,
            }
        )
        request = urllib.request.Request(
            f"https://translate.googleapis.com/translate_a/single?{query}",
            headers={"User-Agent": "Mozilla/5.0"},
        )
        with urllib.request.urlopen(request, timeout=4) as response:
            payload = json.loads(response.read().decode("utf-8"))
        translated = "".join(part[0] for part in payload[0] if part and part[0])
        return translated or None
    except Exception:
        return None


def days_until(timestamp):
    value = number(timestamp)
    if value is None:
        return None
    days = int(round((value - time.time()) / 86400))
    return days if days >= 0 else None


def add_history_points(points, history):
    for ts, row in history.iterrows():
        close = number(row.get("Close"))
        if close is None:
            continue
        volume = number(row.get("Volume"))
        try:
            unix_ts = int(ts.timestamp())
        except Exception:
            continue
        points[unix_ts] = {
            "ts": unix_ts,
            "close": round(close, 4),
            "volume": volume,
        }


def main():
    symbol = sys.stdin.read().strip().upper()
    result = {
        "price": None,
        "previous_close": None,
        "day_volume": None,
        "open": None,
        "day_high": None,
        "day_low": None,
        "market_cap": None,
        "avg_volume": None,
        "extended_price": None,
        "extended_change_percent": None,
        "week_52_high": None,
        "week_52_low": None,
        "trailing_pe": None,
        "forward_pe": None,
        "dividend_yield": None,
        "beta": None,
        "next_earnings_days": None,
        "summary": None,
        "summary_de": None,
        "city": None,
        "state": None,
        "country": None,
        "website": None,
        "full_time_employees": None,
        "history": [],
    }

    if not symbol:
        print(json.dumps(result), flush=True)
        return

    try:
        ticker = yf.Ticker(symbol)
        fast = ticker.fast_info
        result["price"] = number(pick(fast, ["last_price", "lastPrice"]))
        result["previous_close"] = number(
            pick(fast, ["previous_close", "previousClose", "regularMarketPreviousClose"])
        )
        result["market_cap"] = number(pick(fast, ["market_cap", "marketCap"]))
        result["avg_volume"] = number(
            pick(fast, ["three_month_average_volume", "threeMonthAverageVolume"])
        )
        result["day_volume"] = number(pick(fast, ["last_volume", "lastVolume", "regularMarketVolume"]))
        result["week_52_high"] = number(pick(fast, ["year_high", "yearHigh"]))
        result["week_52_low"] = number(pick(fast, ["year_low", "yearLow"]))
        result["open"] = number(pick(fast, ["open", "regularMarketOpen"]))
        result["day_high"] = number(pick(fast, ["day_high", "dayHigh"]))
        result["day_low"] = number(pick(fast, ["day_low", "dayLow"]))

        try:
            points = {}
            daily = ticker.history(period="5y", interval="1d")[["Close", "Volume"]].dropna(
                subset=["Close"]
            )
            add_history_points(points, daily)
            intraday = ticker.history(period="1d", interval="1m")[["Close", "Volume"]].dropna(
                subset=["Close"]
            )
            add_history_points(points, intraday)
            result["history"] = [points[key] for key in sorted(points)]
        except Exception:
            result["history"] = []

        info = ticker.info
        result["market_cap"] = result["market_cap"] or number(info.get("marketCap"))
        result["avg_volume"] = result["avg_volume"] or number(info.get("averageVolume"))
        result["day_volume"] = result["day_volume"] or number(
            info.get("volume") or info.get("regularMarketVolume")
        )
        result["extended_price"] = number(
            info.get("postMarketPrice")
            or info.get("preMarketPrice")
            or info.get("postMarketPrice")
        )
        result["week_52_high"] = result["week_52_high"] or number(info.get("fiftyTwoWeekHigh"))
        result["week_52_low"] = result["week_52_low"] or number(info.get("fiftyTwoWeekLow"))
        result["open"] = result["open"] or number(info.get("open") or info.get("regularMarketOpen"))
        result["day_high"] = result["day_high"] or number(info.get("dayHigh"))
        result["day_low"] = result["day_low"] or number(info.get("dayLow"))
        result["trailing_pe"] = number(info.get("trailingPE"))
        result["forward_pe"] = number(info.get("forwardPE"))
        result["dividend_yield"] = number(info.get("dividendYield"))
        result["beta"] = number(info.get("beta"))
        result["next_earnings_days"] = days_until(
            info.get("earningsTimestamp")
            or info.get("earningsTimestampStart")
        )
        result["city"] = info.get("city")
        result["state"] = info.get("state")
        result["country"] = info.get("country")
        result["website"] = info.get("website")
        result["full_time_employees"] = number(info.get("fullTimeEmployees"))
        result["summary"] = two_sentence_summary(info.get("longBusinessSummary", ""))
        result["summary_de"] = translate_to_german(result["summary"])
        if result["extended_price"] is not None and result["price"] not in (None, 0):
            result["extended_change_percent"] = (
                (result["extended_price"] - result["price"]) / result["price"]
            ) * 100
    except Exception:
        pass

    print(json.dumps(result, separators=(",", ":")), flush=True)


if __name__ == "__main__":
    main()
