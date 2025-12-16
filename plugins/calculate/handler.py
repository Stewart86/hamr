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

TEST_MODE = os.environ.get("HAMR_TEST_MODE") == "1"


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
    if TEST_MODE:
        # Mock responses for testing
        mock_results = {
            "2+2": "4",
            "sqrt(16)": "4",
            "10 celsius to fahrenheit": "50 °F",
            "50 USD": "50 USD",
            "20% * 32": "6.4",
        }
        return mock_results.get(expr, f"= {expr}")

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
                            "execute": {
                                "command": ["wl-copy", result],
                                "notify": f"Copied: {result}",
                            },
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
                                "execute": {
                                    "command": ["wl-copy", result],
                                    "notify": f"Copied: {result}",
                                    "name": f"Calculate: {query} = {result}",
                                },
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

        if item_id == "calc_result":
            # Re-calculate to get current result
            expr = preprocess_expression(query)
            result = calculate(expr)

            if result:
                print(
                    json.dumps(
                        {
                            "type": "execute",
                            "execute": {
                                "command": ["wl-copy", result],
                                "notify": f"Copied: {result}",
                                "name": f"Calculate: {query} = {result}",
                                "icon": "calculate",
                                "close": True,
                            },
                        }
                    )
                )
            else:
                print(json.dumps({"type": "error", "message": "Could not calculate"}))
            return

        print(json.dumps({"type": "error", "message": f"Unknown action: {item_id}"}))


if __name__ == "__main__":
    main()
