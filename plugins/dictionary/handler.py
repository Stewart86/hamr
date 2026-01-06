#!/usr/bin/env python3
"""
Dictionary workflow handler - looks up word definitions using Free Dictionary API
"""

import json
import os
import sys
import urllib.request
import urllib.error

# Test mode - return mock data instead of calling real API
TEST_MODE = os.environ.get("HAMR_TEST_MODE") == "1"

# Mock definitions for common test words
MOCK_DEFINITIONS = {
    "hello": {
        "word": "hello",
        "phonetic": "/həˈloʊ/",
        "meanings": [
            {
                "partOfSpeech": "exclamation",
                "definitions": [
                    {
                        "definition": "Used as a greeting or to begin a phone conversation.",
                        "example": "Hello, how are you?",
                    }
                ],
                "synonyms": ["hi", "hey", "greetings"],
                "antonyms": ["goodbye", "farewell"],
            },
            {
                "partOfSpeech": "noun",
                "definitions": [
                    {
                        "definition": "An utterance of 'hello'; a greeting.",
                        "example": "She gave a friendly hello.",
                    }
                ],
                "synonyms": [],
                "antonyms": [],
            },
        ],
    },
    "happy": {
        "word": "happy",
        "phonetic": "/ˈhæpi/",
        "meanings": [
            {
                "partOfSpeech": "adjective",
                "definitions": [
                    {
                        "definition": "Feeling or showing pleasure or contentment.",
                        "example": "She was happy to see him.",
                    }
                ],
                "synonyms": ["cheerful", "content", "delighted", "joyful", "glad"],
                "antonyms": ["sad", "unhappy", "miserable", "depressed"],
            }
        ],
    },
    "cat": {
        "word": "cat",
        "phonetic": "/kæt/",
        "meanings": [
            {
                "partOfSpeech": "noun",
                "definitions": [
                    {
                        "definition": "A small domesticated carnivorous mammal with soft fur.",
                        "example": "The cat sat on the mat.",
                    }
                ],
                "synonyms": ["feline", "kitty"],
                "antonyms": [],
            }
        ],
    },
    "dog": {
        "word": "dog",
        "phonetic": "/dɔɡ/",
        "meanings": [
            {
                "partOfSpeech": "noun",
                "definitions": [
                    {
                        "definition": "A domesticated carnivorous mammal that typically has a long snout and an acute sense of smell.",
                        "example": "The dog wagged its tail.",
                    }
                ],
                "synonyms": ["canine", "hound", "pooch"],
                "antonyms": [],
            }
        ],
    },
    "run": {
        "word": "run",
        "phonetic": "/rʌn/",
        "meanings": [
            {
                "partOfSpeech": "verb",
                "definitions": [
                    {
                        "definition": "Move at a speed faster than a walk.",
                        "example": "She ran to catch the bus.",
                    }
                ],
                "synonyms": ["sprint", "dash", "race", "jog"],
                "antonyms": ["walk", "stroll"],
            }
        ],
    },
    "walk": {
        "word": "walk",
        "phonetic": "/wɔːk/",
        "meanings": [
            {
                "partOfSpeech": "verb",
                "definitions": [
                    {
                        "definition": "Move at a regular pace by lifting and setting down each foot in turn.",
                        "example": "We walked along the beach.",
                    }
                ],
                "synonyms": ["stroll", "amble", "saunter"],
                "antonyms": ["run", "sprint"],
            }
        ],
    },
    "think": {
        "word": "think",
        "phonetic": "/θɪŋk/",
        "meanings": [
            {
                "partOfSpeech": "verb",
                "definitions": [
                    {
                        "definition": "Have a particular belief or idea.",
                        "example": "I think it's going to rain.",
                    }
                ],
                "synonyms": ["believe", "consider", "ponder", "reflect"],
                "antonyms": [],
            }
        ],
    },
    "a": {
        "word": "a",
        "phonetic": "/ə/",
        "meanings": [
            {
                "partOfSpeech": "article",
                "definitions": [
                    {
                        "definition": "Used when referring to someone or something for the first time.",
                        "example": "A man walked into the room.",
                    }
                ],
                "synonyms": [],
                "antonyms": [],
            }
        ],
    },
}


def get_definition(word: str) -> dict | None:
    """Fetch word definition from Free Dictionary API (or mock in test mode)"""
    if TEST_MODE:
        # Return mock data in test mode
        return MOCK_DEFINITIONS.get(word.lower())

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
                    # Copy to clipboard using wl-copy (skip in test mode)
                    if not TEST_MODE:
                        import subprocess

                        subprocess.run(["wl-copy", content], check=False)

                    print(
                        json.dumps(
                            {
                                "type": "execute",
                                "notify": f"Definition of '{word}' copied",
                                "close": True,
                            }
                        )
                    )
            return


if __name__ == "__main__":
    main()
