#!/usr/bin/env python3
"""
Create Plugin - AI helper to create new workflow plugins for Hamr launcher

IMPORTANT: This plugin requires OpenCode CLI to be installed.
Install opencode: https://opencode.ai

This plugin provides a conversational interface where the AI will:
1. Ask clarifying questions about your plugin idea
2. Discuss the approach before creating
3. Only create the plugin when you confirm

If opencode is not available, this plugin will display an error message
with instructions for manual plugin creation.
"""

import json
import os
import shutil
import subprocess
import sys
import time
from datetime import datetime
from pathlib import Path

# Test mode - mock external dependencies
TEST_MODE = os.environ.get("HAMR_TEST_MODE") == "1"

# Check if opencode is available (always available in test mode)
OPENCODE_AVAILABLE = shutil.which("opencode") is not None or TEST_MODE

# Session storage file
SESSION_FILE = Path.home() / ".cache" / "hamr" / "create-plugin-session.json"


def get_actions_dir() -> Path:
    """Get the actions directory path"""
    config_home = os.environ.get("XDG_CONFIG_HOME", os.path.expanduser("~/.config"))
    return Path(config_home) / "hamr" / "actions"


def load_session() -> dict:
    """Load conversation session from cache"""
    if SESSION_FILE.exists():
        try:
            with open(SESSION_FILE) as f:
                return json.load(f)
        except (json.JSONDecodeError, IOError):
            pass
    return {"messages": [], "state": "initial"}


def save_session(session: dict):
    """Save conversation session to cache"""
    SESSION_FILE.parent.mkdir(parents=True, exist_ok=True)
    with open(SESSION_FILE, "w") as f:
        json.dump(session, f)


def clear_session():
    """Clear the conversation session"""
    if SESSION_FILE.exists():
        SESSION_FILE.unlink()


def get_system_prompt() -> str:
    """Get the system prompt for the AI"""
    actions_dir = get_actions_dir()
    return f"""You are a helpful assistant that helps users create plugins for the Hamr launcher.

Your role is to:
1. Understand what kind of plugin the user wants to create
2. Ask clarifying questions if the request is unclear
3. Discuss the approach and features before creating anything
4. Only create the plugin when the user confirms they're ready

IMPORTANT RULES:
- Do NOT create files or run commands until the user explicitly confirms
- Ask clarifying questions first (e.g., "What should happen when...", "Do you want it to...")
- When you understand the requirements, summarize what you'll create and ask "Should I create this plugin now?"
- Only when user says yes/confirm/create it/go ahead, then create the files

When creating a plugin:
- Create it in: {actions_dir}
- Use a descriptive folder name (lowercase, hyphens)
- Create manifest.json with name, description, and icon (Material Symbols icon name)
- Create handler.py following this protocol:

Input JSON (stdin): {{"step": "initial|search|action", "query": "...", "selected": {{"id": "..."}}}}

Output JSON (stdout) - one of:
- {{"type": "results", "results": [{{"id": "...", "name": "...", "description": "...", "icon": "..."}}]}}
- {{"type": "card", "card": {{"title": "...", "content": "markdown content", "markdown": true}}}}
- {{"type": "execute", "execute": {{"command": ["cmd", "args"], "close": true}}}}
- {{"type": "imageBrowser", "imageBrowser": {{"directory": "~/Pictures", "title": "...", "actions": [...]}}}}
- {{"type": "prompt", "prompt": {{"text": "Enter something..."}}}}

Make handler.py executable (chmod +x).

Look at existing plugins in {actions_dir} for examples.
Read {actions_dir}/AGENTS.md for comprehensive documentation on the plugin protocol.

## Converting Raycast Extensions

If the user provides a Raycast extension URL or asks to replicate Raycast functionality:

1. **Fetch and analyze** the Raycast extension source code
2. **Identify the core functionality** - what data does it fetch/display?
3. **Map components** to Hamr equivalents:
   - Raycast `List` → Hamr `results` response
   - Raycast `List.Item` → Hamr result object with id, name, description, icon, actions
   - Raycast `Detail` → Hamr `card` response with markdown
   - Raycast `Grid` → Hamr `imageBrowser` or results with thumbnails
   - Raycast `Form` → Multi-step workflow with `inputMode: "submit"`
   - Raycast `ActionPanel` → Hamr `actions` array on result items

4. **Translate platform APIs** (macOS → Linux):
   - `Clipboard.copy()` → `["wl-copy", "text"]`
   - `Action.OpenInBrowser` → `["xdg-open", "url"]`
   - `showToast()` → `"notify": "message"` in execute response
   - `showHUD()` → `notify-send` command

5. **Update file paths** for Linux:
   - `~/Library/Application Support/Google/Chrome` → `~/.config/google-chrome`
   - `~/Library/Application Support/BraveSoftware/Brave-Browser` → `~/.config/BraveSoftware/Brave-Browser`
   - `~/Library/Application Support/Microsoft Edge` → `~/.config/microsoft-edge`
   - `~/Library/Caches` → `~/.cache`
   - `~/Library/Preferences` → `~/.config`

6. **Handle hooks** - Raycast uses React hooks, Hamr is stateless:
   - `usePromise`/`useCachedPromise` → Fetch data directly in handler
   - `useCachedState` → Use file-based JSON cache or `context` field
   - `useState` → Use `context` field to persist state across search calls

When converting, focus on the CORE FUNCTIONALITY. Don't try to replicate every feature - 
prioritize what works well on Linux. Safari support doesn't make sense on Linux, for example."""


def chat_with_opencode(user_message: str, session: dict) -> tuple[bool, dict]:
    """Send a message to OpenCode and return a structured payload."""

    # Mock response in test mode
    if TEST_MODE:
        messages = session.get("messages", [])
        now = int(time.time())
        mock_response = f"Mock AI response to: {user_message}"
        messages.append({"role": "user", "content": user_message, "ts": now})
        messages.append(
            {
                "role": "assistant",
                "content": mock_response,
                "ts": now,
                "thinking": "",
                "toolCalls": "",
                "raw": "",
            }
        )
        session["messages"] = messages
        save_session(session)
        return True, {"text": mock_response}

    try:
        messages = session.get("messages", [])

        # First message includes system prompt
        if not messages:
            full_prompt = f"{get_system_prompt()}\n\nUser: {user_message}"
        else:
            full_prompt = user_message

        cmd = ["opencode", "run", "--format", "json"]
        if messages:
            cmd.append("--continue")
        cmd.append(full_prompt)

        result = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            timeout=120,
            cwd=str(get_actions_dir()),
        )

        if result.returncode != 0:
            return False, {"error": result.stderr or "OpenCode command failed"}

        payload = extract_opencode_payload(result.stdout)

        now = int(time.time())
        messages.append({"role": "user", "content": user_message, "ts": now})
        messages.append(
            {
                "role": "assistant",
                "content": payload.get("text", ""),
                "ts": int(time.time()),
                "thinking": payload.get("thinking", ""),
                "toolCalls": payload.get("toolCalls", ""),
                "raw": payload.get("raw", ""),
            }
        )
        session["messages"] = messages
        save_session(session)

        return True, payload

    except subprocess.TimeoutExpired:
        return False, {"error": "Request timed out. Please try again."}
    except subprocess.SubprocessError as e:
        return False, {"error": f"Error: {e}"}


def _dedupe_consecutive(items: list[str]) -> list[str]:
    out: list[str] = []
    for item in items:
        if item and (not out or out[-1] != item):
            out.append(item)
    return out


def extract_opencode_payload(stdout: str) -> dict:
    """Extract readable text + raw events from OpenCode JSON stream."""

    lines = stdout.strip().split("\n")
    text_parts: list[str] = []
    events: list[dict] = []
    thinking_parts: list[str] = []

    for line in lines:
        if not line.strip():
            continue

        try:
            event = json.loads(line)
            events.append(event)

            event_type = (event.get("type") or "").strip()

            if event_type == "text":
                part = event.get("part")
                if isinstance(part, dict):
                    text = part.get("text")
                    if isinstance(text, str) and text:
                        text_parts.append(text)

                    for key in ("thinking", "thought", "reasoning"):
                        maybe = part.get(key)
                        if isinstance(maybe, str) and maybe:
                            thinking_parts.append(maybe)

            elif event_type == "message.completed":
                content = event.get("message", {}).get("content", "")
                if isinstance(content, str) and content:
                    text_parts.append(content)

        except json.JSONDecodeError:
            # Not JSON, might be plain text output
            if line.strip() and not line.startswith("{"):
                text_parts.append(line)

    text = "\n".join(_dedupe_consecutive(text_parts)).strip()
    thinking = "\n".join(_dedupe_consecutive(thinking_parts)).strip()

    # Heuristic: treat any event with "tool" in its type (or an explicit tool field)
    # as a tool-call artifact.
    tool_events = [
        e
        for e in events
        if "tool" in (e.get("type", "").lower()) or e.get("tool") is not None
    ]

    def pretty(obj) -> str:
        try:
            return json.dumps(obj, indent=2, ensure_ascii=False)
        except TypeError:
            return str(obj)

    return {
        "text": text,
        "thinking": thinking,
        "toolCalls": pretty(tool_events) if tool_events else "",
        "raw": pretty(events) if events else "",
    }


def format_time(ts: int) -> str:
    return datetime.fromtimestamp(ts).strftime("%H:%M")


def format_date(ts: int) -> str:
    return datetime.fromtimestamp(ts).strftime("%b %d, %Y")


def build_conversation_card(session: dict, *, title: str) -> dict:
    messages = session.get("messages", [])

    blocks: list[dict] = []

    last_date = ""
    for msg in messages:
        role = msg.get("role", "assistant")
        content = msg.get("content", "")
        ts = msg.get("ts")
        try:
            ts_int = int(ts) if ts is not None else None
        except (TypeError, ValueError):
            ts_int = None

        if ts_int is not None:
            current_date = format_date(ts_int)
            if current_date != last_date:
                blocks.append({"type": "pill", "text": current_date})
                last_date = current_date

        details = {
            "thinking": msg.get("thinking", ""),
            "toolCalls": msg.get("toolCalls", ""),
            "artifacts": msg.get("artifacts", []),
            "raw": msg.get("raw", ""),
        }

        blocks.append(
            {
                "type": "message",
                "role": role,
                "content": content,
                "markdown": role != "user",
                "timestamp": format_time(ts_int) if ts_int is not None else "",
                "details": details,
            }
        )

    transcript = "\n\n".join(
        f"{m.get('role', 'assistant')}: {m.get('content', '').strip()}".strip()
        for m in messages
        if (m.get("content") or "").strip()
    ).strip()

    return {
        "kind": "blocks",
        "title": title,
        "blocks": blocks,
        "maxHeight": 820,
        "showDetails": False,
        "allowToggleDetails": True,
        "transcript": transcript,
    }


def main():
    input_data = json.load(sys.stdin)
    step = input_data.get("step", "initial")
    query = input_data.get("query", "").strip()
    selected = input_data.get("selected", {})

    # Check opencode availability
    if not OPENCODE_AVAILABLE:
        print(
            json.dumps(
                {
                    "type": "card",
                    "card": {
                        "title": "OpenCode Required",
                        "content": """**OpenCode CLI is required to use this plugin.**

OpenCode is an AI coding agent for the terminal.

## Installation

Install from the official website: https://opencode.ai

## Manual Plugin Creation

You can manually create plugins by:
1. Creating a folder in `~/.config/hamr/actions/`
2. Adding a `manifest.json` with name, description, and icon
3. Adding a `handler.py` script that reads JSON from stdin and outputs JSON to stdout

See existing plugins for examples.""",
                        "markdown": True,
                    },
                }
            )
        )
        return

    session = load_session()

    if step == "initial":
        # Check if we have an ongoing conversation
        if session.get("messages"):
            # Show option to continue or start fresh
            print(
                json.dumps(
                    {
                        "type": "results",
                        "inputMode": "submit",
                        "results": [
                            {
                                "id": "continue",
                                "name": "Continue previous conversation",
                                "description": "Resume where you left off",
                                "icon": "history",
                            },
                            {
                                "id": "new",
                                "name": "Start new conversation",
                                "description": "Clear history and start fresh",
                                "icon": "add_circle",
                            },
                            {
                                "id": "help",
                                "name": "How plugins work",
                                "description": "Learn about the plugin protocol",
                                "icon": "help",
                            },
                        ],
                        "placeholder": "Type your message and press Enter...",
                    }
                )
            )
        else:
            print(
                json.dumps(
                    {
                        "type": "results",
                        "inputMode": "submit",
                        "results": [
                            {
                                "id": "help",
                                "name": "How plugins work",
                                "description": "Learn about the plugin protocol",
                                "icon": "help",
                            },
                        ],
                        "placeholder": "Describe the plugin you want to create... (Enter to send)",
                    }
                )
            )
        return

    if step == "search":
        # With submit mode, this is only called when user presses Enter
        # So we process the query directly as a message to the AI
        if not query:
            # Empty submit - just show results
            print(
                json.dumps(
                    {
                        "type": "results",
                        "inputMode": "submit",
                        "results": [],
                        "placeholder": "Describe your plugin idea... (Enter to send)",
                    }
                )
            )
            return

        # Process the query as a message to the AI
        success, payload = chat_with_opencode(query, session)

        if not success:
            # Add a system message so the timeline reflects the failure
            session_messages = session.get("messages", [])
            session_messages.append(
                {
                    "role": "system",
                    "content": f"Failed to get response: {payload.get('error', 'Unknown error')}",
                    "ts": int(time.time()),
                }
            )
            session["messages"] = session_messages
            save_session(session)

        card_payload = build_conversation_card(session, title="Create Plugin")

        print(
            json.dumps(
                {
                    "type": "card",
                    "card": card_payload,
                    "inputMode": "submit",
                    "placeholder": "Type your reply... (Enter to send)",
                    "clearInput": True,
                }
            )
        )
        return

    if step == "action":
        item_id = selected.get("id", "")

        if item_id == "help":
            print(
                json.dumps(
                    {
                        "type": "card",
                        "card": {
                            "title": "Hamr Plugin Protocol",
                            "content": """## How Plugins Work

Plugins are folders in `~/.config/hamr/actions/` containing:

### manifest.json
```json
{
  "name": "My Plugin",
  "description": "What it does",
  "icon": "material_icon_name"
}
```

### handler.py (executable)
Communicates via JSON on stdin/stdout:

**Input:** `{"step": "initial|search|action", "query": "...", "selected": {"id": "..."}}`

**Output options:**
- `{"type": "results", "results": [...]}`
- `{"type": "card", "card": {"title": "...", "content": "...", "markdown": true}}`
- `{"type": "execute", "execute": {"command": [...], "close": true}}`

## Using This Plugin

Just describe what you want to create! The AI will:
1. Ask clarifying questions
2. Discuss the approach
3. Create the plugin when you confirm""",
                            "markdown": True,
                        },
                        "inputMode": "submit",
                        "placeholder": "Type your plugin idea... (Enter to send)",
                    }
                )
            )
            return

        if item_id == "new":
            clear_session()
            session = {"messages": [], "state": "initial"}
            print(
                json.dumps(
                    {
                        "type": "results",
                        "inputMode": "submit",
                        "results": [],
                        "placeholder": "Describe the plugin you want to create... (Enter to send)",
                        "clearInput": True,
                    }
                )
            )
            return

        if item_id == "continue":
            if session.get("messages"):
                print(
                    json.dumps(
                        {
                            "type": "card",
                            "card": build_conversation_card(
                                session, title="Create Plugin"
                            ),
                            "inputMode": "submit",
                            "placeholder": "Type your reply... (Enter to send)",
                            "clearInput": True,
                        }
                    )
                )
            else:
                print(
                    json.dumps(
                        {
                            "type": "results",
                            "inputMode": "submit",
                            "results": [],
                            "placeholder": "Continue describing your plugin... (Enter to send)",
                        }
                    )
                )
            return


if __name__ == "__main__":
    main()
