# CLI Reference

Hamr provides a unified CLI (`hamr`) for controlling the launcher.

## Quick Reference

| Command | Description |
|---------|-------------|
| `hamr` | Start GTK launcher (auto-starts daemon) |
| `hamr toggle` | Toggle launcher visibility |
| `hamr plugin <id>` | Open a specific plugin |
| `hamr status` | Check daemon status |
| `hamr install` | Optional: set up systemd services and user directories |

## Starting Hamr

```bash
# Start GTK launcher (recommended - auto-starts daemon if needed)
hamr

# Or start components separately
hamr daemon    # Run daemon in foreground
hamr gtk       # Run GTK UI in foreground
hamr tui       # Run TUI in terminal
```

## Controlling the Launcher

```bash
hamr toggle              # Toggle visibility (bind to hotkey)
hamr show                # Show the launcher
hamr hide                # Hide the launcher
hamr plugin clipboard    # Open specific plugin
hamr plugin apps         # Open apps plugin
```

## Daemon Management

```bash
hamr status          # Check if daemon is running
hamr shutdown        # Stop the daemon
hamr reload-plugins  # Reload plugins without restart
```

## Plugin Management

```bash
hamr plugins list    # List installed plugins
hamr plugins audit   # Verify plugin checksums
```

## Installation Commands

Hamr works without systemd. Use `hamr install` only if you want systemd user services and the default user directories.

```bash
hamr install --check  # Preview what will be set up
hamr install          # Set up systemd services and directories
hamr uninstall        # Remove systemd services (keeps config)
```

## Systemd Integration

Systemd is opt-in. After running `hamr install`, systemd services are created:

```bash
# Start services
systemctl --user start hamr-gtk    # Starts both GTK and daemon

# Check status
systemctl --user status hamr-daemon
systemctl --user status hamr-gtk

# View logs
journalctl --user -u hamr-daemon -f
journalctl --user -u hamr-gtk -f

# Stop services
systemctl --user stop hamr-gtk
systemctl --user stop hamr-daemon
```

## Keybinding Examples

### Hyprland

```conf
exec-once = hamr
bind = $mainMod, SPACE, exec, hamr toggle
bind = $mainMod, V, exec, hamr plugin clipboard
```

### Niri

```kdl
spawn-at-startup "hamr"

binds {
    Mod+Space { spawn "hamr" "toggle"; }
    Mod+V { spawn "hamr" "plugin" "clipboard"; }
}
```

## Environment Variables

| Variable | Description |
|----------|-------------|
| `RUST_LOG` | Log level (`debug`, `info`, `warn`, `error`) |
| `HAMR_PLUGIN_DEBUG` | Enable plugin debug output |

See [Logging](logging.md) for more details.
