#!/usr/bin/env python3
"""
What's That Word? - Find words from descriptions or fix misspellings

Uses AI to suggest words based on:
- Descriptions of the word's meaning
- Misspelled words that need correction

Returns a list of word suggestions with copy actions.
"""

import json
import shutil
import subprocess
import sys

OPENCODE_AVAILABLE = shutil.which("opencode") is not None

SYSTEM_PROMPT = """You are a word-finding assistant. The user will either:
1. Describe a word they're trying to remember (e.g., "the fear of heights")
2. Provide a misspelled word they need corrected (e.g., "definately")

Your task is to respond with a JSON array of the most likely words, ordered by relevance.

IMPORTANT: Respond ONLY with a valid JSON array of strings. No explanations, no markdown, no other text.

Examples:
- Input: "fear of heights" -> ["acrophobia", "vertigo", "altophobia"]
- Input: "definately" -> ["definitely", "defiantly", "definite"]
- Input: "word for when you postpone things" -> ["procrastinate", "defer", "delay", "postpone"]
- Input: "feeling of already experienced something" -> ["deja vu", "familiarity", "recognition"]

Return 3-8 words maximum, most likely first."""


def query_ai(user_input: str) -> list[str]:
    """Query OpenCode for word suggestions"""
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

        start_idx = output.find("[")
        end_idx = output.rfind("]")

        if start_idx != -1 and end_idx != -1:
            json_str = output[start_idx : end_idx + 1]
            words = json.loads(json_str)
            if isinstance(words, list):
                return [str(w) for w in words if w]

        return []

    except (subprocess.TimeoutExpired, subprocess.SubprocessError):
        return []


def build_word_results(words: list[str], query: str) -> dict:
    """Build results response for word list"""
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

    return {
        "type": "results",
        "results": results,
        "placeholder": "Or try a different description...",
        "inputMode": "submit",
        "context": query,
    }


def main():
    input_data = json.load(sys.stdin)
    step = input_data.get("step", "initial")
    query = input_data.get("query", "").strip()
    selected = input_data.get("selected", {})
    action = input_data.get("action", "")
    context = input_data.get("context", "")

    if step == "initial":
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

        print(
            json.dumps(
                {
                    "type": "results",
                    "results": [
                        {
                            "id": "__help__",
                            "name": "Describe a word or type a misspelling",
                            "description": "Press Enter to search",
                            "icon": "info",
                        }
                    ],
                    "placeholder": "e.g., 'fear of heights' or 'definately'",
                    "inputMode": "submit",
                }
            )
        )
        return

    if step == "search":
        if not OPENCODE_AVAILABLE:
            print(json.dumps({"type": "results", "results": []}))
            return

        if not query:
            print(
                json.dumps(
                    {
                        "type": "results",
                        "results": [],
                        "placeholder": "Describe the word or type misspelling...",
                        "inputMode": "submit",
                    }
                )
            )
            return

        words = query_ai(query)

        if not words:
            print(
                json.dumps(
                    {
                        "type": "results",
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
                        "inputMode": "submit",
                        "context": query,
                    }
                )
            )
            return

        print(json.dumps(build_word_results(words, query)))
        return

    if step == "action":
        item_id = selected.get("id", "")

        if item_id in ("__help__", "__not_found__", "__error__"):
            print(json.dumps({}))
            return

        if item_id == "__retry__":
            retry_query = context if context else query
            if retry_query:
                words = query_ai(retry_query)
                if words:
                    print(json.dumps(build_word_results(words, retry_query)))
                    return

            print(
                json.dumps(
                    {
                        "type": "results",
                        "results": [],
                        "clearInput": True,
                        "placeholder": "Describe the word or type misspelling...",
                        "inputMode": "submit",
                    }
                )
            )
            return

        if item_id.startswith("word:"):
            word = item_id[5:]
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

        print(json.dumps({}))


if __name__ == "__main__":
    main()
