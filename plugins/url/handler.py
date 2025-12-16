#!/usr/bin/env python3
"""
URL plugin - detect and open URLs in browser.
Triggered via match patterns when user types something that looks like a URL.
"""

import json
import os
import sys

TEST_MODE = os.environ.get("HAMR_TEST_MODE") == "1"


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

    # ===== MATCH: Called when query matches URL pattern =====
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
                        "execute": {
                            "command": ["xdg-open", url],
                        },
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

    # ===== ACTION: Handle action buttons =====
    if step == "action":
        action = input_data.get("action", "")

        if action == "copy":
            url = normalize_url(query)
            print(
                json.dumps(
                    {
                        "type": "execute",
                        "execute": {
                            "command": ["wl-copy", url],
                            "notify": f"Copied: {url}",
                            "close": True,
                        },
                    }
                )
            )
            return

        # Default action: open URL
        url = normalize_url(query)
        print(
            json.dumps(
                {
                    "type": "execute",
                    "execute": {
                        "command": ["xdg-open", url],
                        "close": True,
                    },
                }
            )
        )
        return

    # ===== INITIAL/SEARCH: Not typically used for match-only plugin =====
    print(json.dumps({"type": "results", "results": []}))


if __name__ == "__main__":
    main()
