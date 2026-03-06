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

OPENCODE_BIN_ENV = "HAMR_WHATS_THAT_WORD_OPENCODE_BIN"
OPENCODE_MODEL_ENV = "HAMR_WHATS_THAT_WORD_MODEL"
COMMON_OPENCODE_PATHS = (
    "~/.local/bin/opencode",
    "~/.bun/bin/opencode",
    "~/bin/opencode",
)

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


def resolve_opencode_bin() -> str | None:
    """Resolve the OpenCode CLI path in launcher/daemon environments."""
    candidates = []

    bin_override = os.environ.get(OPENCODE_BIN_ENV, "").strip()
    if bin_override:
        candidates.append(bin_override)

    resolved = shutil.which("opencode")
    if resolved:
        candidates.append(resolved)

    candidates.extend(os.path.expanduser(path) for path in COMMON_OPENCODE_PATHS)

    for candidate in candidates:
        if candidate and os.path.isfile(candidate) and os.access(candidate, os.X_OK):
            return candidate

    for shell_name in ("bash", "zsh"):
        shell_path = shutil.which(shell_name)
        if not shell_path:
            continue

        try:
            result = subprocess.run(
                [shell_path, "-lc", "command -v opencode"],
                capture_output=True,
                text=True,
                timeout=2,
                check=False,
            )
        except (subprocess.TimeoutExpired, subprocess.SubprocessError):
            continue

        candidate = result.stdout.strip()
        if (
            result.returncode == 0
            and candidate
            and os.path.isfile(candidate)
            and os.access(candidate, os.X_OK)
        ):
            return candidate

    return None


def get_opencode_command() -> list[str] | None:
    """Build the OpenCode command with optional model override."""
    opencode_bin = resolve_opencode_bin()
    if not opencode_bin:
        return None

    command = [opencode_bin, "run"]
    model = os.environ.get(OPENCODE_MODEL_ENV, "").strip()
    if model:
        command.extend(["--model", model])

    return command


def build_error_results(
    name: str, description: str, context: str = ""
) -> dict[str, object]:
    """Build a visible submit-mode error response instead of silently doing nothing."""
    response: dict[str, object] = {
        "type": "results",
        "results": [
            {
                "id": "__error__",
                "name": name,
                "description": description,
                "icon": "error",
            }
        ],
        "placeholder": "Describe the word or type misspelling...",
        "inputMode": "submit",
    }

    if context:
        response["context"] = context

    return response


def query_ai(user_input: str) -> tuple[list[str], str | None]:
    """Query OpenCode for word suggestions"""
    command = get_opencode_command()
    if not command:
        return [], (
            "OpenCode CLI was not found. Install `opencode` or set "
            f"`{OPENCODE_BIN_ENV}` to the executable path."
        )

    try:
        prompt = f"{SYSTEM_PROMPT}\n\nUser input: {user_input}"
        result = subprocess.run(
            [*command, prompt],
            capture_output=True,
            text=True,
            timeout=30,
            check=False,
        )

        if result.returncode != 0:
            error_text = result.stderr.strip() or result.stdout.strip()
            if error_text:
                error_text = error_text.splitlines()[-1]
            else:
                error_text = "OpenCode returned a non-zero exit status."
            return [], error_text

        output = result.stdout.strip()

        start_idx = output.find("[")
        end_idx = output.rfind("]")

        if start_idx != -1 and end_idx != -1:
            json_str = output[start_idx : end_idx + 1]
            words = json.loads(json_str)
            if isinstance(words, list):
                return [str(w) for w in words if w], None

        return [], "OpenCode returned an unexpected response format."

    except json.JSONDecodeError:
        return [], "OpenCode returned invalid JSON."
    except (subprocess.TimeoutExpired, subprocess.SubprocessError):
        return [], "OpenCode request failed before returning suggestions."


def build_word_results(words: list[str], query: str) -> dict:
    """Build results response for word list"""
    results: list[dict[str, object]] = [
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
        if not get_opencode_command():
            print(
                json.dumps(
                    build_error_results(
                        "OpenCode CLI required",
                        "Install `opencode` or set HAMR_WHATS_THAT_WORD_OPENCODE_BIN.",
                    )
                )
            )
            return

        print(
            json.dumps(
                {
                    "type": "results",
                    "results": [],
                    "placeholder": "e.g., 'fear of heights' or 'definately'",
                    "inputMode": "submit",
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
                        "placeholder": "Describe the word or type misspelling...",
                        "inputMode": "submit",
                    }
                )
            )
            return

        words, error = query_ai(query)

        if error:
            print(
                json.dumps(
                    build_error_results(
                        "Unable to search words",
                        error,
                        context=query,
                    )
                )
            )
            return

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

        if item_id in ("__not_found__", "__error__"):
            print(json.dumps({"type": "noop"}))
            return

        if item_id == "__retry__":
            retry_query = context if context else query
            if retry_query:
                words, error = query_ai(retry_query)
                if error:
                    print(
                        json.dumps(
                            build_error_results(
                                "Unable to search words",
                                error,
                                context=retry_query,
                            )
                        )
                    )
                    return

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

        print(json.dumps({"type": "noop"}))


if __name__ == "__main__":
    main()
