#!/usr/bin/env python3
"""
URL plugin - detect and open URLs in browser.
Triggered via match patterns when user types something that looks like a URL.
"""

import json
import sys


def normalize_url(url: str) -> str:
    """Add https:// if no protocol specified."""
    url = url.strip()
    if not url.startswith(("http://", "https://", "ftp://")):
        return "https://" + url
    return url


def main():
    input_data = json.load(sys.stdin)
    step = input_data.get("step", "initial")
    query = input_data.get("query", "").strip()
    selected = input_data.get("selected", {})

    if step == "match":
        if not query:
            print(json.dumps({"type": "match", "result": None}))
            return

        url = normalize_url(query)

        print(
            json.dumps(
                {
                    "type": "match",
                    "result": {
                        "id": "open_url",
                        "name": url,
                        "description": "Open in browser",
                        "icon": "open_in_browser",
                        "verb": "Open",
                        "openUrl": url,
                        "close": True,
                        "actions": [
                            {
                                "id": "copy",
                                "name": "Copy URL",
                                "icon": "content_copy",
                            }
                        ],
                        "priority": 90,
                    },
                }
            )
        )
        return

    # Handle initial and search - show URL result
    if step in ("initial", "search"):
        if not query:
            print(json.dumps({"type": "results", "results": []}))
            return

        url = normalize_url(query)
        print(
            json.dumps(
                {
                    "type": "results",
                    "results": [
                        {
                            # Store URL in id so it's available in action step
                            "id": url,
                            "name": url,
                            "description": "Open in browser",
                            "icon": "open_in_browser",
                            "verb": "Open",
                            "openUrl": url,
                            "close": True,
                            "actions": [
                                {
                                    "id": "copy",
                                    "name": "Copy URL",
                                    "icon": "content_copy",
                                }
                            ],
                        }
                    ],
                    "inputMode": "realtime",
                }
            )
        )
        return

    if step == "action":
        action = input_data.get("action", "")
        # URL is stored in selected.id from the search results
        url = selected.get("id", "")

        if not url:
            print(json.dumps({"type": "error", "message": "No URL provided"}))
            return

        if action == "copy":
            print(
                json.dumps(
                    {
                        "type": "execute",
                        "copy": url,
                        "notify": f"Copied: {url}",
                        "close": True,
                    }
                )
            )
            return

        # Default action: open URL
        print(
            json.dumps(
                {
                    "type": "execute",
                    "openUrl": url,
                    "close": True,
                }
            )
        )
        return

    print(json.dumps({"type": "results", "results": []}))


if __name__ == "__main__":
    main()
