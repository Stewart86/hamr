#!/usr/bin/env python3
"""
Hamr Socket Plugin SDK

Provides helpers for connecting to the hamr daemon and communicating via JSON-RPC 2.0.

Example usage:

    from hamr_sdk import HamrPlugin

    plugin = HamrPlugin(
        id="my-plugin",
        name="My Plugin",
        description="A socket-based plugin",
        icon="extension"
    )

    @plugin.on_search
    def handle_search(query: str, context: str | None) -> list[dict]:
        return [{"id": "1", "name": "Result", "icon": "star"}]

    @plugin.on_action
    def handle_action(item_id: str, action: str | None, context: str | None, source: str | None) -> dict | None:
        # source is "ambient" for ambient bar actions, None for regular actions
        return {"type": "execute", "copy": "Hello", "close": True}

    plugin.run()
"""

import asyncio
import inspect
import json
import os
import signal
import struct
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Callable, Optional


def get_socket_path() -> str:
    """Get the hamr daemon socket path.

    Tries dev socket first (hamr-dev.sock) if it exists, otherwise falls back
    to production socket (hamr.sock). This allows plugins to work with both
    dev and production daemons.
    """
    runtime_dir = os.environ.get("XDG_RUNTIME_DIR", "/tmp")
    dev_socket = os.path.join(runtime_dir, "hamr-dev.sock")
    prod_socket = os.path.join(runtime_dir, "hamr.sock")

    # Prefer dev socket if it exists
    if os.path.exists(dev_socket):
        return dev_socket
    return prod_socket


def _is_debug_enabled() -> bool:
    """Check if debug logging is enabled via environment variable."""
    return os.environ.get("HAMR_PLUGIN_DEBUG", "").lower() in ("1", "true", "yes")


@dataclass
class PluginManifest:
    """Plugin manifest for registration."""

    id: str
    name: str
    description: Optional[str] = None
    icon: Optional[str] = None
    prefix: Optional[str] = None
    priority: int = 0

    def to_dict(self) -> dict:
        result = {
            "id": self.id,
            "name": self.name,
            "priority": self.priority,
        }
        if self.description:
            result["description"] = self.description
        if self.icon:
            result["icon"] = self.icon
        if self.prefix:
            result["prefix"] = self.prefix
        return result


class HamrPlugin:
    """
    Base class for socket-based hamr plugins.

    Handles connection, registration, and message routing.
    """

    def __init__(
        self,
        id: str,
        name: str,
        description: Optional[str] = None,
        icon: Optional[str] = None,
        prefix: Optional[str] = None,
        priority: int = 0,
        socket_path: Optional[str] = None,
        debug: Optional[bool] = None,
    ):
        self.manifest = PluginManifest(
            id=id,
            name=name,
            description=description,
            icon=icon,
            prefix=prefix,
            priority=priority,
        )
        self.socket_path = socket_path or get_socket_path()
        self.debug = debug if debug is not None else _is_debug_enabled()

        self._reader: Optional[asyncio.StreamReader] = None
        self._writer: Optional[asyncio.StreamWriter] = None
        self._request_id = 0
        self._pending_responses: dict[int, asyncio.Future] = {}

        # Handlers
        self._on_initial: Optional[Callable] = None
        self._on_search: Optional[Callable] = None
        self._on_action: Optional[Callable] = None
        self._on_form_submitted: Optional[Callable] = None
        self._on_slider_changed: Optional[Callable] = None
        self._on_switch_toggled: Optional[Callable] = None

        # Background tasks
        self._background_tasks: list[Callable] = []

        # Shutdown flag
        self._shutdown = False

    def _log(self, message: str) -> None:
        """Log a debug message if debug mode is enabled."""
        if self.debug:
            print(f"[{self.manifest.id}] {message}", file=sys.stderr)

    def on_initial(self, handler: Callable):
        """Decorator for initial request handler."""
        self._on_initial = handler
        return handler

    def on_search(self, handler: Callable):
        """Decorator for search request handler."""
        self._on_search = handler
        return handler

    def on_action(self, handler: Callable):
        """Decorator for action request handler."""
        self._on_action = handler
        return handler

    def on_form_submitted(self, handler: Callable):
        """Decorator for form submission handler."""
        self._on_form_submitted = handler
        return handler

    def on_slider_changed(self, handler: Callable):
        """Decorator for slider change handler."""
        self._on_slider_changed = handler
        return handler

    def on_switch_toggled(self, handler: Callable):
        """Decorator for switch toggle handler."""
        self._on_switch_toggled = handler
        return handler

    def add_background_task(self, task: Callable):
        """Add a background coroutine to run alongside message handling."""
        self._background_tasks.append(task)
        return task

    async def connect(self) -> None:
        """Connect to the hamr daemon socket."""
        self._log(f"Connecting to {self.socket_path}")
        self._reader, self._writer = await asyncio.open_unix_connection(
            self.socket_path
        )
        self._log("Connected")

    async def register(self) -> dict:
        """Register this plugin with the daemon."""
        response = await self._send_request(
            "register",
            {
                "role": {
                    "type": "plugin",
                    "id": self.manifest.id,
                    "manifest": self.manifest.to_dict(),
                }
            },
        )
        self._log(f"Registered: {response}")
        return response

    async def send_results(self, results: list[dict], **kwargs) -> None:
        """Send search results to the daemon."""
        params = {"results": results, **kwargs}
        await self._send_notification("plugin_results", params)

    async def send_status(self, status: dict) -> None:
        """Send status update (badges, chips, ambient items)."""
        await self._send_notification("plugin_status", {"status": status})

    async def send_index(self, items: list[dict]) -> None:
        """Send index items for search indexing."""
        await self._send_notification("plugin_index", {"items": items})

    async def send_execute(self, action: dict) -> None:
        """Request action execution (copy, launch, etc.)."""
        await self._send_notification("plugin_execute", {"action": action})

    async def send_update(self, patches: list[dict]) -> None:
        """Send partial result updates."""
        await self._send_notification("plugin_update", {"patches": patches})

    # ========== Response Builders ==========
    # These methods build properly typed response dicts for handler returns.

    @staticmethod
    def results(
        items: list[dict],
        *,
        input_mode: Optional[str] = None,
        status: Optional[dict] = None,
        context: Optional[str] = None,
        placeholder: Optional[str] = None,
        clear_input: bool = False,
        navigate_forward: Optional[bool] = None,
        plugin_actions: Optional[list[dict]] = None,
        navigation_depth: Optional[int] = None,
        display_hint: Optional[str] = None,
    ) -> dict:
        """Build a results response.

        Args:
            items: List of result items
            display_hint: Optional hint for display mode: "auto", "list", "grid", "large_grid"
        """
        response: dict[str, Any] = {"type": "results", "results": items}
        if input_mode:
            response["inputMode"] = input_mode
        if status:
            response["status"] = status
        if context:
            response["context"] = context
        if placeholder:
            response["placeholder"] = placeholder
        if clear_input:
            response["clearInput"] = clear_input
        if navigate_forward is not None:
            response["navigateForward"] = navigate_forward
        if plugin_actions:
            response["pluginActions"] = plugin_actions
        if navigation_depth is not None:
            response["navigationDepth"] = navigation_depth
        if display_hint:
            response["displayHint"] = display_hint
        return response

    @staticmethod
    def form(
        form: dict,
        *,
        context: Optional[str] = None,
    ) -> dict:
        """Build a form response."""
        response: dict[str, Any] = {"type": "form", "form": form}
        if context:
            response["context"] = context
        return response

    @staticmethod
    def card(
        title: str,
        *,
        content: Optional[str] = None,
        markdown: Optional[str] = None,
        actions: Optional[list[dict]] = None,
        status: Optional[dict] = None,
        context: Optional[str] = None,
        kind: Optional[str] = None,
        blocks: Optional[list[dict]] = None,
        max_height: Optional[int] = None,
        show_details: Optional[bool] = None,
        allow_toggle_details: Optional[bool] = None,
    ) -> dict:
        """Build a card response.

        Card data is nested under a 'card' key to match Rust's expected structure:
        {"type": "card", "card": {...}, "status": ..., "context": ...}
        """
        card_data: dict[str, Any] = {"title": title}
        if content:
            card_data["content"] = content
        if markdown:
            card_data["markdown"] = markdown
        if actions:
            card_data["actions"] = actions
        if kind:
            card_data["kind"] = kind
        if blocks:
            card_data["blocks"] = blocks
        if max_height:
            card_data["maxHeight"] = max_height
        if show_details is not None:
            card_data["showDetails"] = show_details
        if allow_toggle_details is not None:
            card_data["allowToggleDetails"] = allow_toggle_details

        response: dict[str, Any] = {"type": "card", "card": card_data}
        if status:
            response["status"] = status
        if context:
            response["context"] = context
        return response

    @staticmethod
    def execute(
        *,
        launch: Optional[str] = None,
        copy: Optional[str] = None,
        url: Optional[str] = None,
        close: bool = False,
        hide: bool = False,
        type_text: Optional[str] = None,
        play_sound: Optional[str] = None,
    ) -> dict:
        """Build an execute response.

        Fields are placed directly in the response (not nested under 'execute')
        to match Rust's serde tagged enum deserialization.
        """
        response: dict[str, Any] = {"type": "execute"}
        if launch:
            response["launch"] = launch
        if copy:
            response["copy"] = copy
        if url:
            response["openUrl"] = url
        if close:
            response["close"] = close
        if hide:
            response["hide"] = hide
        if type_text:
            response["typeText"] = type_text
        if play_sound:
            response["sound"] = play_sound
        return response

    @staticmethod
    def noop() -> dict:
        """Build a no-op response (execute with no actions)."""
        return {"type": "execute"}

    @staticmethod
    def close() -> dict:
        """Build a close response (convenience for execute with close=True)."""
        return {"type": "execute", "close": True}

    @staticmethod
    def copy_and_close(text: str) -> dict:
        """Build a copy-and-close response."""
        return {"type": "execute", "copy": text, "close": True}

    @staticmethod
    def launch_and_close(desktop_file: str) -> dict:
        """Build a launch-and-close response."""
        return {"type": "execute", "launch": desktop_file, "close": True}

    @staticmethod
    def open_url(url: str, *, close: bool = True) -> dict:
        """Build an open-url response."""
        return {"type": "execute", "openUrl": url, "close": close}

    @staticmethod
    def error(message: str, *, details: str | None = None) -> dict:
        """Build an error response."""
        result = {"type": "error", "message": message}
        if details:
            result["details"] = details
        return result

    def _next_id(self) -> int:
        self._request_id += 1
        return self._request_id

    async def _send_request(self, method: str, params: dict) -> dict:
        """Send a request and wait for response."""
        request_id = self._next_id()
        message = {
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": request_id,
        }

        await self._write_message(message)

        # Read response directly (message_loop hasn't started yet during registration)
        response = await self._read_message()
        if response is None:
            raise RuntimeError(f"No response received for request {request_id}")

        # Check for error response
        if "error" in response:
            raise RuntimeError(response["error"].get("message", "Unknown error"))

        # Return the result
        return response.get("result", {})

    async def _send_notification(self, method: str, params: dict) -> None:
        """Send a notification (no response expected)."""
        message = {
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        }
        await self._write_message(message)

    async def _write_message(self, message: dict) -> None:
        """Write a length-prefixed JSON message."""
        self._log(f"Writing: {message}")
        if not self._writer:
            raise RuntimeError("Not connected")

        data = json.dumps(message).encode("utf-8")
        length = struct.pack(">I", len(data))
        self._writer.write(length + data)
        await self._writer.drain()

    async def _read_message(self) -> Optional[dict]:
        """Read a length-prefixed JSON message."""
        if not self._reader:
            return None

        try:
            length_bytes = await self._reader.readexactly(4)
            length = struct.unpack(">I", length_bytes)[0]
            data = await self._reader.readexactly(length)
            return json.loads(data.decode("utf-8"))
        except asyncio.IncompleteReadError:
            return None
        except Exception as e:
            self._log(f"Read error: {e}")
            return None

    async def _handle_message(self, message: dict) -> None:
        """Handle an incoming message."""
        self._log(f"Received message: {message}")
        # Check if this is a response to a pending request
        if "id" in message and "result" in message:
            request_id = message["id"]
            if request_id in self._pending_responses:
                self._pending_responses[request_id].set_result(
                    message.get("result", {})
                )
                return

        if "id" in message and "error" in message:
            request_id = message["id"]
            if request_id in self._pending_responses:
                self._pending_responses[request_id].set_exception(
                    RuntimeError(message["error"].get("message", "Unknown error"))
                )
                return

        # Handle incoming requests
        method = message.get("method", "")
        params = message.get("params") or {}
        request_id = message.get("id")

        result = None

        if method == "initial":
            if self._on_initial:
                try:
                    result = await self._call_handler(self._on_initial, params)
                except Exception as e:
                    self._log(f"ERROR in initial handler: {e}")
                    import traceback

                    self._log(traceback.format_exc())
                    raise

        elif method == "search":
            if self._on_search:
                query = params.get("query", "")
                context = params.get("context")
                result = await self._call_handler(self._on_search, query, context)

        elif method == "action":
            if self._on_action:
                item_id = params.get("item_id", "")
                action = params.get("action")
                context = params.get("context")
                source = params.get("source")
                result = await self._call_handler(
                    self._on_action, item_id, action, context, source
                )

        elif method == "form_submitted":
            if self._on_form_submitted:
                form_data = params.get("form_data", {})
                context = params.get("context")
                result = await self._call_handler(
                    self._on_form_submitted, form_data, context
                )

        elif method == "slider_changed":
            if self._on_slider_changed:
                slider_id = params.get("id", "")
                value = params.get("value", 0)
                result = await self._call_handler(
                    self._on_slider_changed, slider_id, value
                )

        elif method == "switch_toggled":
            if self._on_switch_toggled:
                switch_id = params.get("id", "")
                value = params.get("value", False)
                result = await self._call_handler(
                    self._on_switch_toggled, switch_id, value
                )

        self._log(f"Handler result for {method}: {type(result)}")

        # Send response if this was a request
        if request_id is not None:
            response = {
                "jsonrpc": "2.0",
                "result": result or {},
                "id": request_id,
            }
            self._log(f"Sending response for request_id={request_id}")
            await self._write_message(response)

    async def _call_handler(self, handler: Callable, *args) -> Any:
        """Call a handler, supporting both sync and async handlers.

        Automatically adapts the number of arguments to match the handler's signature,
        allowing backward compatibility when new parameters are added.
        """
        # Get the handler's signature to determine how many args it accepts
        try:
            sig = inspect.signature(handler)
            max_params = len(sig.parameters)
            # Only pass as many args as the handler can accept
            args = args[:max_params]
        except (ValueError, TypeError):
            # If we can't inspect the signature, just pass all args
            pass

        result = handler(*args)
        if asyncio.iscoroutine(result):
            result = await result
        return result

    async def _message_loop(self) -> None:
        """Main message handling loop."""
        while not self._shutdown:
            message = await self._read_message()
            if message is None:
                self._log("Connection closed")
                break

            try:
                await self._handle_message(message)
            except Exception as e:
                self._log(f"Handler error: {e}")

    async def _run_async(self) -> None:
        """Run the plugin (async version)."""
        # Setup signal handlers
        loop = asyncio.get_event_loop()

        def shutdown():
            self._shutdown = True

        for sig in (signal.SIGTERM, signal.SIGINT):
            loop.add_signal_handler(sig, shutdown)

        try:
            await self.connect()
            await self.register()

            # Start background tasks (fire-and-forget, they can complete without affecting the main loop)
            background_tasks = []
            for bg_task in self._background_tasks:
                background_tasks.append(asyncio.create_task(bg_task(self)))

            # Run message loop as the main controlling task
            # Background tasks completing won't cause the plugin to exit
            await self._message_loop()

            # Cancel any remaining background tasks
            for task in background_tasks:
                if not task.done():
                    task.cancel()

        except Exception as e:
            self._log(f"Error: {e}")

        finally:
            if self._writer:
                self._writer.close()
                await self._writer.wait_closed()

    def run(self) -> None:
        """Run the plugin (blocking)."""
        asyncio.run(self._run_async())


# Convenience type exports
Result = dict
Results = list[dict]
Status = dict
Action = dict
