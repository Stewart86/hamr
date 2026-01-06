#!/usr/bin/env python3
"""
What's That Word? - Find words from descriptions or fix misspellings

Uses AI to suggest words based on:
- Descriptions of the word's meaning
- Misspelled words that need correction

Returns a list of word suggestions with copy actions.
"""

import json
import os
import shutil
import subprocess
import sys

# Test mode - return mock data instead of calling real AI
TEST_MODE = os.environ.get("HAMR_TEST_MODE") == "1"

OPENCODE_AVAILABLE = shutil.which("opencode") is not None or TEST_MODE

SYSTEM_PROMPT = """You are a word-finding assistant. The user will either:
1. Describe a word they're trying to remember (e.g., "the fear of heights")
2. Provide a misspelled word they need corrected (e.g., "definately")

Your task is to respond with a JSON array of the most likely words, ordered by relevance.

IMPORTANT: Respond ONLY with a valid JSON array of strings. No explanations, no markdown, no other text.

Examples:
- Input: "fear of heights" → ["acrophobia", "vertigo", "altophobia"]
- Input: "definately" → ["definitely", "defiantly", "definite"]
- Input: "word for when you postpone things" → ["procrastinate", "defer", "delay", "postpone"]
- Input: "feeling of already experienced something" → ["déjà vu", "familiarity", "recognition"]

Return 3-8 words maximum, most likely first."""


def query_ai(user_input: str) -> list[str]:
    """Query OpenCode for word suggestions (or mock in test mode)"""
    # In test mode, return mock suggestions based on input patterns
    if TEST_MODE:
        input_lower = user_input.lower()
        # Common test patterns with mock responses
        if "fear" in input_lower and "height" in input_lower:
            return ["acrophobia", "vertigo", "altophobia"]
        elif "definat" in input_lower or "definit" in input_lower:
            return ["definitely", "defiantly", "definite"]
        elif "postpone" in input_lower or "delay" in input_lower:
            return ["procrastinate", "defer", "delay", "postpone"]
        elif "deja" in input_lower or "already" in input_lower:
            return ["déjà vu", "familiarity", "recognition"]
        elif input_lower.strip():
            # Return generic mock suggestions for any non-empty query
            return ["suggestion1", "suggestion2", "suggestion3"]
        return []

    try:
        prompt = f"{SYSTEM_PROMPT}\n\nUser input: {user_input}"
        result = subprocess.run(
            ["opencode", "--model", "google/gemini-2.5-flash", "run", prompt],
            capture_output=True,
            text=True,
            timeout=30,
        )

        if result.returncode != 0:
            return []

        output = result.stdout.strip()

        # Try to extract JSON array from output
        start_idx = output.find("[")
        end_idx = output.rfind("]")

        if start_idx != -1 and end_idx != -1:
            json_str = output[start_idx : end_idx + 1]
            words = json.loads(json_str)
            if isinstance(words, list):
                return [str(w) for w in words if w]

        return []

    except (
        subprocess.TimeoutExpired,
        subprocess.SubprocessError,
        json.JSONDecodeError,
    ):
        return []


def main():
    input_data = json.load(sys.stdin)
    step = input_data.get("step", "initial")
    query = input_data.get("query", "").strip()
    selected = input_data.get("selected", {})
    action = input_data.get("action", "")
    context = input_data.get("context", "")

    # Check opencode availability
    if not OPENCODE_AVAILABLE:
        print(
            json.dumps(
                {
                    "type": "results",
                    "results": [
                        {
                            "id": "__error__",
                            "name": "OpenCode CLI required",
                            "description": "Install from https://opencode.ai",
                            "icon": "error",
                        }
                    ],
                }
            )
        )
        return

    if step == "initial":
        print(
            json.dumps(
                {
                    "type": "results",
                    "inputMode": "submit",
                    "results": [
                        {
                            "id": "__help__",
                            "name": "Describe a word or type a misspelling",
                            "description": "Press Enter to search",
                            "icon": "info",
                        }
                    ],
                    "placeholder": "e.g., 'fear of heights' or 'definately'",
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
                        "inputMode": "submit",
                        "results": [],
                        "placeholder": "Describe the word or type misspelling...",
                    }
                )
            )
            return

        # Query AI for word suggestions
        words = query_ai(query)

        if not words:
            print(
                json.dumps(
                    {
                        "type": "results",
                        "inputMode": "submit",
                        "context": query,
                        "results": [
                            {
                                "id": "__not_found__",
                                "name": "No words found",
                                "description": "Try a different description",
                                "icon": "search_off",
                            },
                            {
                                "id": "__retry__",
                                "name": "Try again",
                                "description": "Search with same query",
                                "icon": "refresh",
                            },
                        ],
                        "placeholder": "Try a different description...",
                    }
                )
            )
            return

        # Build results list with copy action
        results = [
            {
                "id": "__retry__",
                "name": "Try again",
                "description": "Get different suggestions",
                "icon": "refresh",
            }
        ]
        for i, word in enumerate(words):
            results.append(
                {
                    "id": f"word:{word}",
                    "name": word,
                    "description": "Best match" if i == 0 else "",
                    "icon": "star" if i == 0 else "label",
                    "verb": "Copy",
                    "actions": [
                        {"id": "copy", "name": "Copy", "icon": "content_copy"},
                    ],
                }
            )

        print(
            json.dumps(
                {
                    "type": "results",
                    "inputMode": "submit",
                    "context": query,
                    "results": results,
                    "placeholder": "Or try a different description...",
                }
            )
        )
        return

    if step == "action":
        selected_id = selected.get("id", "")

        # Help item - not actionable
        if selected_id == "__help__":
            return

        # Not found - not actionable
        if selected_id == "__not_found__":
            return

        # Retry with same query
        if selected_id == "__retry__":
            if context:
                words = query_ai(context)
                if words:
                    results = [
                        {
                            "id": "__retry__",
                            "name": "Try again",
                            "description": "Get different suggestions",
                            "icon": "refresh",
                        }
                    ]
                    for i, word in enumerate(words):
                        results.append(
                            {
                                "id": f"word:{word}",
                                "name": word,
                                "description": "Best match" if i == 0 else "",
                                "icon": "star" if i == 0 else "label",
                                "verb": "Copy",
                                "actions": [
                                    {
                                        "id": "copy",
                                        "name": "Copy",
                                        "icon": "content_copy",
                                    },
                                ],
                            }
                        )
                    print(
                        json.dumps(
                            {
                                "type": "results",
                                "inputMode": "submit",
                                "context": context,
                                "results": results,
                                "placeholder": "Or try a different description...",
                            }
                        )
                    )
                    return

            # No context or failed - show empty
            print(
                json.dumps(
                    {
                        "type": "results",
                        "inputMode": "submit",
                        "results": [],
                        "clearInput": True,
                        "placeholder": "Describe the word or type misspelling...",
                    }
                )
            )
            return

        # Word selection - copy to clipboard
        if selected_id.startswith("word:"):
            word = selected_id[5:]  # Remove "word:" prefix

            # Copy action or default selection (skip in test mode)
            if not TEST_MODE:
                subprocess.run(["wl-copy", word], check=False)
            print(
                json.dumps(
                    {
                        "type": "execute",
                        "notify": f"Copied: {word}",
                        "close": True,
                    }
                )
            )
            return


if __name__ == "__main__":
    main()
