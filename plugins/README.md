# Plugins

This directory contains the built-in plugins for Hamr launcher.

## Documentation

Full documentation is available at: **https://stewart86.github.io/hamr**

| Document | Description |
|----------|-------------|
| **[Getting Started](../docs/plugins/index.md)** | Build your first plugin in 5 minutes |
| [Response Types](../docs/plugins/response-types.md) | All response types (`results`, `execute`, `card`, `form`, etc.) |
| [Visual Elements](../docs/plugins/visual-elements.md) | Sliders, switches, badges, gauges, progress bars |
| [Advanced Features](../docs/plugins/advanced-features.md) | Daemon mode, indexing, search ranking |
| [Testing](../docs/plugins/testing.md) | Manual testing and debugging |
| [Cheat Sheet](../docs/plugins/cheatsheet.md) | Quick reference for common patterns |
| [Raycast Conversion](../docs/plugins/raycast-conversion.md) | Porting Raycast extensions to Hamr |

## Quick Links

- **Start here:** [docs/plugins/index.md](../docs/plugins/index.md)
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
# See docs/plugins/index.md for a complete tutorial
```
