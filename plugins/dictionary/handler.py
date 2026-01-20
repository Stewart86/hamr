#!/usr/bin/env python3
"""
Dictionary plugin - look up word definitions using Free Dictionary API
"""

import json
import sys
import urllib.request
import urllib.error


def get_definition(word: str) -> dict | None:
    """Fetch word definition from Free Dictionary API"""
    url = f"https://api.dictionaryapi.dev/api/v2/entries/en/{word}"
    try:
        with urllib.request.urlopen(url, timeout=5) as response:
            data = json.loads(response.read().decode())
            if data and len(data) > 0:
                return data[0]
    except (urllib.error.URLError, urllib.error.HTTPError, json.JSONDecodeError):
        pass
    return None


def format_definition(data: dict) -> str:
    """Format dictionary data into readable markdown"""
    word = data.get("word", "")
    phonetic = data.get("phonetic", "")

    lines = []
    if phonetic:
        lines.append(f"**{word}** {phonetic}")
    else:
        lines.append(f"**{word}**")
    lines.append("")

    all_synonyms = []
    all_antonyms = []

    for meaning in data.get("meanings", [])[:3]:  # Limit to 3 meanings
        part_of_speech = meaning.get("partOfSpeech", "")
        lines.append(f"*{part_of_speech}*")

        for i, definition in enumerate(
            meaning.get("definitions", [])[:2], 1
        ):  # Limit to 2 definitions
            defn = definition.get("definition", "")
            lines.append(f"{i}. {defn}")

            example = definition.get("example")
            if example:
                lines.append(f'   > "{example}"')

        # Collect synonyms/antonyms from meaning level
        all_synonyms.extend(meaning.get("synonyms", []))
        all_antonyms.extend(meaning.get("antonyms", []))

        lines.append("")

    # Add synonyms section if any exist
    if all_synonyms:
        unique_synonyms = list(dict.fromkeys(all_synonyms))[:8]  # Limit to 8
        lines.append(f"**Synonyms:** {', '.join(unique_synonyms)}")
        lines.append("")

    # Add antonyms section if any exist
    if all_antonyms:
        unique_antonyms = list(dict.fromkeys(all_antonyms))[:8]  # Limit to 8
        lines.append(f"**Antonyms:** {', '.join(unique_antonyms)}")
        lines.append("")

    return "\n".join(lines)


def main():
    input_data = json.load(sys.stdin)
    step = input_data.get("step", "initial")
    query = input_data.get("query", "").strip()
    selected = input_data.get("selected", {})
    action = input_data.get("action", "")

    if step == "initial":
        # Just started - prompt for input
        print(
            json.dumps(
                {"type": "prompt", "prompt": {"text": "Enter word to define..."}}
            )
        )
        return

    if step == "search":
        if not query:
            print(
                json.dumps({"type": "results", "results": [], "inputMode": "realtime"})
            )
            return

        # Look up the word
        data = get_definition(query)

        if data:
            # Found definition - show as markdown card
            content = format_definition(data)
            word = data.get("word", query)

            print(
                json.dumps(
                    {
                        "type": "card",
                        "card": {
                            "content": content,
                            "markdown": True,
                            "actions": [
                                {
                                    "id": "copy",
                                    "name": "Copy",
                                    "icon": "content_copy",
                                },
                            ],
                        },
                        "inputMode": "realtime",
                        "context": word,  # Store word for copy action
                    }
                )
            )
        else:
            # No definition found
            print(
                json.dumps(
                    {
                        "type": "results",
                        "results": [
                            {
                                "id": "__not_found__",
                                "name": f"No definition found for '{query}'",
                                "icon": "search_off",
                            }
                        ],
                        "inputMode": "realtime",
                    }
                )
            )
        return

    if step == "action":
        item_id = selected.get("id", "")
        context = input_data.get("context", "")

        if item_id == "__not_found__":
            return

        # Copy action from card
        if action == "copy" or item_id == "copy":
            word = context if context else query
            if word:
                # Re-fetch definition for copy
                data = get_definition(word)
                if data:
                    content = format_definition(data)
                    import subprocess

                    subprocess.run(["wl-copy"], input=content.encode(), check=False)

                    print(
                        json.dumps(
                            {
                                "type": "execute",
                                "action": {
                                    "type": "notify",
                                    "message": f"Definition of '{word}' copied",
                                },
                                "close": True,
                            }
                        )
                    )
            return


if __name__ == "__main__":
    main()
