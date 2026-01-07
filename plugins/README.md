# Plugins

This directory contains the built-in plugins for Hamr launcher.

## Documentation

The plugin development documentation has been reorganized for easier navigation:

| Document | Description |
|----------|-------------|
| **[Getting Started](../docs/PLUGINS.md)** | Build your first plugin in 5 minutes |
| [Response Types](../docs/plugins/response-types.md) | All response types (`results`, `execute`, `card`, `form`, etc.) |
| [Visual Elements](../docs/plugins/visual-elements.md) | Sliders, switches, badges, gauges, progress bars |
| [Advanced Features](../docs/plugins/advanced-features.md) | Daemon mode, polling, indexing, status badges |
| [Testing](../docs/plugins/testing.md) | Test harness usage and mock data patterns |
| [Cheat Sheet](../docs/plugins/CHEATSHEET.md) | Quick reference for common patterns |
| [Raycast Conversion](../docs/plugins/raycast-conversion.md) | Porting Raycast extensions to Hamr |

## Quick Links

- **Start here:** [docs/PLUGINS.md](../docs/PLUGINS.md)
- **Test your plugin:** `HAMR_TEST_MODE=1 ./test-harness ./my-plugin/handler.py initial`
- **View logs:** `journalctl --user -f`

## Built-in Plugins

| Plugin | Trigger | Description |
|--------|---------|-------------|
| [`apps/`](apps/) | `/apps` | Application launcher |
| [`bitwarden/`](bitwarden/) | `/bitwarden` | Password manager |
| [`calculate/`](calculate/) | `=` | Calculator |
| [`clipboard/`](clipboard/) | `;` | Clipboard history |
| [`dict/`](dict/) | `/dict` | Dictionary lookup |
| [`emoji/`](emoji/) | `/emoji` | Emoji picker |
| [`files/`](files/) | `~` | File search |
| [`notes/`](notes/) | `/notes` | Quick notes |
| [`player/`](player/) | `/player` | Media controls |
| [`quicklinks/`](quicklinks/) | `/quicklinks` | Web search shortcuts |
| [`screenshot/`](screenshot/) | `/screenshot` | Screenshot browser |
| [`shell/`](shell/) | `!` | Shell history |
| [`sound/`](sound/) | `/sound` | Volume controls |
| [`timer/`](timer/) | `/timer` | Countdown timers |
| [`todo/`](todo/) | `/todo` | Todo list |
| [`wallpaper/`](wallpaper/) | `/wallpaper` | Wallpaper selector |

## User Plugins

Create your plugins in `~/.config/hamr/plugins/`:

```bash
mkdir -p ~/.config/hamr/plugins/my-plugin
# Create manifest.json and handler.py
# See docs/PLUGINS.md for a complete tutorial
```
