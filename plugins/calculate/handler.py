#!/usr/bin/env python3
"""
Calculate plugin - math expressions, currency, units, and temperature conversion.
Uses qalc (qalculate) for evaluation.

Supports:
  - Basic math: "2+2", "sqrt(16)", "sin(pi/2)"
  - Temperature: "10c", "34f", "10 celsius to fahrenheit"
  - Currency: "$50", "S$100", "100 USD to EUR", "50 in JPY"
  - Units: "10ft to m", "5 miles to km", "100kg to lb"
  - Percentages: "20% of 32", "15% off 100"
  - Time: "10:30 + 2h"
"""

import json
import os
import re
import subprocess
import sys
from pathlib import Path

# Calculation history tracking
CACHE_DIR = Path(os.environ.get("XDG_CACHE_HOME", Path.home() / ".cache")) / "hamr"
CALC_HISTORY_FILE = CACHE_DIR / "calc-history.json"
MAX_HISTORY_ITEMS = 10


def load_calc_history() -> list[dict]:
    """Load calculation history from cache.

    Returns list of {"query": "2+2", "result": "4"} dicts, most recent first.
    """
    if not CALC_HISTORY_FILE.exists():
        return []
    try:
        return json.loads(CALC_HISTORY_FILE.read_text())
    except (json.JSONDecodeError, OSError):
        return []


def save_calc_history(query: str, result: str) -> None:
    """Save calculation to history (most recent first)."""
    history = load_calc_history()
    # Remove duplicates of same query
    history = [h for h in history if h.get("query") != query]
    history.insert(0, {"query": query, "result": result})
    history = history[:MAX_HISTORY_ITEMS]
    try:
        CACHE_DIR.mkdir(parents=True, exist_ok=True)
        CALC_HISTORY_FILE.write_text(json.dumps(history))
    except OSError:
        pass


def get_plugin_actions() -> list[dict]:
    """Get plugin-level actions for the action bar"""
    return [
        {
            "id": "clear_history",
            "name": "Clear History",
            "icon": "delete_sweep",
            "shortcut": "Ctrl+1",
        },
    ]


def clear_calc_history() -> None:
    """Clear all calculation history"""
    if CALC_HISTORY_FILE.exists():
        try:
            CALC_HISTORY_FILE.unlink()
        except OSError:
            pass


CURRENCY_SYMBOL_MAP = {
    "$": "USD",
    "€": "EUR",
    "£": "GBP",
    "¥": "JPY",
    "₹": "INR",
    "₽": "RUB",
    "₩": "KRW",
    "₪": "ILS",
    "฿": "THB",
    "₫": "VND",
    "₴": "UAH",
    "₸": "KZT",
    "₺": "TRY",
    "₼": "AZN",
    "₾": "GEL",
}

CURRENCY_PREFIX_MAP = {
    "S$": "SGD",
    "HK$": "HKD",
    "A$": "AUD",
    "C$": "CAD",
    "NZ$": "NZD",
    "NT$": "TWD",
    "R$": "BRL",
    "MX$": "MXN",
}

ALL_CURRENCY_CODES = [
    "USD",
    "EUR",
    "GBP",
    "JPY",
    "CNY",
    "SGD",
    "AUD",
    "CAD",
    "CHF",
    "HKD",
    "NZD",
    "SEK",
    "NOK",
    "DKK",
    "KRW",
    "INR",
    "RUB",
    "BRL",
    "MXN",
    "ZAR",
    "TRY",
    "THB",
    "MYR",
    "IDR",
    "PHP",
    "VND",
    "PLN",
    "CZK",
    "HUF",
    "ILS",
    "AED",
    "SAR",
    "TWD",
    "BTC",
    "ETH",
]


def preprocess_thousand_separators(expr: str) -> str:
    """Remove thousand separators: "1,000,000" -> "1000000" """
    return re.sub(r"(\d),(?=\d{3}(?:,\d{3})*(?:\.\d+)?(?:\s|$|[a-zA-Z]))", r"\1", expr)


def preprocess_temperature(expr: str) -> str:
    """
    Temperature shorthand:
    "10c" -> "10 celsius to fahrenheit"
    "34f" -> "34 fahrenheit to celsius"
    """
    result = expr

    # "10c" or "10°c" at end or before space -> "10 celsius"
    result = re.sub(
        r"^(-?\d+\.?\d*)\s*°?c(\s+to\s+|\s+in\s+|\s*$)",
        r"\1 celsius\2",
        result,
        flags=re.IGNORECASE,
    )
    result = re.sub(
        r"^(-?\d+\.?\d*)\s*°?f(\s+to\s+|\s+in\s+|\s*$)",
        r"\1 fahrenheit\2",
        result,
        flags=re.IGNORECASE,
    )

    # Auto-add conversion target for standalone temperature
    if re.match(r"^-?\d+\.?\d*\s+celsius\s*$", result, re.IGNORECASE):
        result += " to fahrenheit"
    elif re.match(r"^-?\d+\.?\d*\s+fahrenheit\s*$", result, re.IGNORECASE):
        result += " to celsius"

    return result


def preprocess_currency(expr: str) -> str:
    """
    Currency symbols to codes:
    "$50" -> "50 USD"
    "S$100" -> "100 SGD"
    "sgd100" -> "100 SGD"
    """
    result = expr

    for prefix, code in CURRENCY_PREFIX_MAP.items():
        escaped = prefix.replace("$", r"\$")
        result = re.sub(
            escaped + r"\s*([\d,]+\.?\d*)", rf"\1 {code}", result, flags=re.IGNORECASE
        )

    for symbol, code in CURRENCY_SYMBOL_MAP.items():
        escaped = re.escape(symbol)
        result = re.sub(escaped + r"\s*([\d,]+\.?\d*)", rf"\1 {code}", result)

    codes_pattern = "|".join(ALL_CURRENCY_CODES)
    match = re.match(rf"^({codes_pattern})\s*([\d,]+\.?\d*)", result, re.IGNORECASE)
    if match:
        result = re.sub(
            rf"^({codes_pattern})\s*([\d,]+\.?\d*)",
            rf"\2 {match.group(1).upper()}",
            result,
            count=1,
            flags=re.IGNORECASE,
        )

    return result


def preprocess_percentage(expr: str) -> str:
    """
    Percentage operations:
    "20% of 32" -> "20% * 32"
    "15% off 100" -> "100 - 15%"
    """
    result = expr
    result = re.sub(r"(\d+\.?\d*\s*%)\s+of\s+", r"\1 * ", result, flags=re.IGNORECASE)
    result = re.sub(
        r"(\d+\.?\d*)\s*%\s+off\s+(\d+\.?\d*)", r"\2 - \1%", result, flags=re.IGNORECASE
    )
    return result


def preprocess_conversion(expr: str) -> str:
    """Normalize "in" to "to" for conversions: "100 USD in EUR" -> "100 USD to EUR" """
    if (
        re.search(r"\d.*\s+in\s+\w+$", expr, re.IGNORECASE)
        and " to " not in expr.lower()
    ):
        return re.sub(r"\s+in\s+(\w+)$", r" to \1", expr, flags=re.IGNORECASE)
    return expr


def convert_hex_binary(expr: str) -> str | None:
    """
    Handle hex/binary/decimal conversions:
    - "0xFF to decimal" or "0xff to dec" -> "255"
    - "255 to hex" -> "0xFF"
    - "255 to binary" or "255 to bin" -> "0b11111111"
    - "0b1111 to decimal" -> "15"
    - "0b1111 to hex" -> "0xF"
    """
    expr_lower = expr.lower().strip()

    # Hex to decimal: "0xff to decimal" or "0xff to dec"
    match = re.match(r"^(0x[0-9a-f]+)\s+to\s+(decimal|dec)$", expr_lower)
    if match:
        try:
            return str(int(match.group(1), 16))
        except ValueError:
            return None

    # Hex to binary: "0xff to binary" or "0xff to bin"
    match = re.match(r"^(0x[0-9a-f]+)\s+to\s+(binary|bin)$", expr_lower)
    if match:
        try:
            return bin(int(match.group(1), 16))
        except ValueError:
            return None

    # Binary to decimal: "0b1111 to decimal"
    match = re.match(r"^(0b[01]+)\s+to\s+(decimal|dec)$", expr_lower)
    if match:
        try:
            return str(int(match.group(1), 2))
        except ValueError:
            return None

    # Binary to hex: "0b1111 to hex"
    match = re.match(r"^(0b[01]+)\s+to\s+hex$", expr_lower)
    if match:
        try:
            return hex(int(match.group(1), 2))
        except ValueError:
            return None

    # Decimal to hex: "255 to hex"
    match = re.match(r"^(\d+)\s+to\s+hex$", expr_lower)
    if match:
        try:
            return hex(int(match.group(1)))
        except ValueError:
            return None

    # Decimal to binary: "255 to binary" or "255 to bin"
    match = re.match(r"^(\d+)\s+to\s+(binary|bin)$", expr_lower)
    if match:
        try:
            return bin(int(match.group(1)))
        except ValueError:
            return None

    return None


def preprocess_expression(query: str, math_prefix: str = "=") -> str:
    """Preprocess query into qalc-friendly syntax."""
    expr = query.strip()

    # Strip math prefix if present
    if math_prefix and expr.startswith(math_prefix):
        expr = expr[len(math_prefix) :].strip()

    expr = preprocess_thousand_separators(expr)
    expr = preprocess_temperature(expr)
    expr = preprocess_currency(expr)
    expr = preprocess_percentage(expr)
    expr = preprocess_conversion(expr)

    return expr


def calculate(expr: str) -> str | None:
    """Run qalc and return result, or None on error."""
    # Try hex/binary conversion first (doesn't need qalc)
    hex_bin_result = convert_hex_binary(expr)
    if hex_bin_result:
        return hex_bin_result

    try:
        result = subprocess.run(
            ["qalc", "-t", expr],
            capture_output=True,
            text=True,
            timeout=5,
        )
        output = result.stdout.strip()

        # Validate result
        if not output:
            return None
        if output == expr:
            return None
        if output.startswith("error:"):
            return None
        if "was not found" in output:
            return None

        return output
    except (subprocess.TimeoutExpired, FileNotFoundError):
        return None


def main():
    input_data = json.load(sys.stdin)
    step = input_data.get("step", "initial")
    query = input_data.get("query", "").strip()
    selected = input_data.get("selected", {})
    action = input_data.get("action", "")

    if step == "match":
        if not query:
            print(json.dumps({"type": "error", "message": "No expression provided"}))
            return

        # Preprocess and calculate
        expr = preprocess_expression(query)
        result = calculate(expr)

        if result:
            print(
                json.dumps(
                    {
                        "type": "match",
                        "result": {
                            "id": "calc_result",
                            "name": result,
                            "description": query,
                            "icon": "calculate",
                            "verb": "Copy",
                            "copy": result,
                            "notify": f"Copied: {result}",
                            "priority": 100,
                        },
                    }
                )
            )
        else:
            # No valid result - return empty (core will hide)
            print(json.dumps({"type": "match", "result": None}))
        return

    if step == "initial":
        # Show history if available
        history = load_calc_history()
        if history:
            results = []
            for h in history:
                results.append(
                    {
                        "id": f"history:{h['query']}",
                        "name": h["result"],
                        "description": h["query"],
                        "icon": "history",
                        "verb": "Copy",
                    }
                )
            print(
                json.dumps(
                    {
                        "type": "results",
                        "results": results,
                        "inputMode": "realtime",
                        "placeholder": "Enter expression (e.g., 2+2, $50 to EUR, 10c)...",
                        "pluginActions": get_plugin_actions(),
                    }
                )
            )
        else:
            print(
                json.dumps(
                    {
                        "type": "prompt",
                        "prompt": {
                            "text": "Enter expression (e.g., 2+2, $50 to EUR, 10c)..."
                        },
                    }
                )
            )
        return

    if step == "search":
        if not query:
            print(
                json.dumps(
                    {
                        "type": "results",
                        "results": [],
                        "placeholder": "Enter expression...",
                    }
                )
            )
            return

        expr = preprocess_expression(query)
        result = calculate(expr)

        if result:
            print(
                json.dumps(
                    {
                        "type": "results",
                        "results": [
                            {
                                "id": "calc_result",
                                "name": result,
                                "description": f"= {query}",
                                "icon": "calculate",
                                "verb": "Copy",
                            }
                        ],
                        "inputMode": "realtime",
                    }
                )
            )
        else:
            print(
                json.dumps(
                    {
                        "type": "results",
                        "results": [
                            {
                                "id": "error",
                                "name": "Invalid expression",
                                "description": query,
                                "icon": "error",
                            }
                        ],
                        "inputMode": "realtime",
                    }
                )
            )
        return

    if step == "action":
        item_id = selected.get("id", "")

        # Plugin action: clear history
        if item_id == "__plugin__" and action == "clear_history":
            clear_calc_history()
            print(
                json.dumps(
                    {
                        "type": "prompt",
                        "prompt": {"text": "History cleared. Enter expression..."},
                    }
                )
            )
            return

        # History item selected - copy the result
        if item_id.startswith("history:"):
            original_query = item_id[8:]  # Remove "history:" prefix
            expr = preprocess_expression(original_query)
            result = calculate(expr)
            if result:
                print(
                    json.dumps(
                        {
                            "type": "execute",
                            "copy": result,
                            "notify": f"Copied: {result}",
                            "close": True,
                        }
                    )
                )
            else:
                print(json.dumps({"type": "error", "message": "Could not calculate"}))
            return

        if item_id == "calc_result":
            # Re-calculate to get current result
            expr = preprocess_expression(query)
            result = calculate(expr)

            if result:
                save_calc_history(query, result)
                print(
                    json.dumps(
                        {
                            "type": "execute",
                            "copy": result,
                            "notify": f"Copied: {result}",
                            "close": True,
                        }
                    )
                )
            else:
                print(json.dumps({"type": "error", "message": "Could not calculate"}))
            return

        print(json.dumps({"type": "error", "message": f"Unknown action: {item_id}"}))


if __name__ == "__main__":
    main()
