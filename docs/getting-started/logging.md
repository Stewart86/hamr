# Logging

This guide explains where Hamr logs are stored, how to adjust verbosity, and privacy considerations.

## Log File Locations

Hamr components write logs to `/tmp/` with timestamped filenames:

| Component | Log File Pattern | Symlink (Latest) |
|-----------|------------------|------------------|
| Daemon | `/tmp/hamr-daemon-YYYYMMDD_HHMMSS.log` | `/tmp/hamr-daemon.log` |
| TUI | `/tmp/hamr-tui-YYYYMMDD_HHMMSS.log` | `/tmp/hamr-tui.log` |

The symlinks always point to the most recent log file for easy access.

### Debug vs Release Builds

| Build Type | Default Level | Output Location |
|------------|---------------|-----------------|
| Debug | `debug` (verbose) | Timestamped files in `/tmp/` |
| Release | `info` | stderr only |

Debug builds include file/line numbers in log output for easier debugging.

## Environment Variables

### RUST_LOG

Controls Rust component logging verbosity using the standard tracing crate:

```bash
# Most verbose - all trace messages
RUST_LOG=trace hamr daemon

# Debug level - detailed diagnostic info
RUST_LOG=debug hamr daemon

# Info level (default for release)
RUST_LOG=info hamr daemon

# Warnings and errors only
RUST_LOG=warn hamr daemon

# Errors only
RUST_LOG=error hamr daemon
```

Filter to specific crates:

```bash
# Debug only for hamr crates (reduces noise from dependencies)
RUST_LOG=hamr=debug hamr daemon

# Combine filters
RUST_LOG=hamr=debug,tokio=warn hamr daemon
```

### HAMR_PLUGIN_DEBUG

Enables verbose debug output in the Python plugin SDK:

```bash
# Enable plugin SDK debug logging
HAMR_PLUGIN_DEBUG=1 hamr daemon
```

When enabled, plugins using the SDK will output additional diagnostic information including:
- Incoming JSON-RPC requests
- Outgoing responses
- Internal state transitions

Accepted values: `1`, `true`, `yes` (case-insensitive)

## Reading Logs

### Follow logs in real-time

```bash
# Daemon logs
tail -f /tmp/hamr-daemon.log

# TUI logs (in separate terminal)
tail -f /tmp/hamr-tui.log

# Both simultaneously
tail -f /tmp/hamr-daemon.log /tmp/hamr-tui.log
```

### Search for specific patterns

```bash
# Find errors and warnings
grep -i "error\|warn" /tmp/hamr-daemon.log

# Plugin-related messages
grep "plugin" /tmp/hamr-daemon.log | tail -30

# Action handling
grep "handle_action\|Forwarding" /tmp/hamr-daemon.log

# Form/settings changes
grep -i "form\|slider\|switch" /tmp/hamr-daemon.log
```

### View historical logs

```bash
# List recent daemon logs (newest first)
ls -lt /tmp/hamr-daemon-*.log | head -5

# List recent TUI logs
ls -lt /tmp/hamr-tui-*.log | head -5
```

## Common Debugging Scenarios

### Plugin not responding

```bash
# Check if plugin is connected
grep "plugin" /tmp/hamr-daemon.log | tail -20

# Look for action forwarding
grep "handle_item_selected\|Forwarding action" /tmp/hamr-daemon.log
```

### JSON deserialization errors

```bash
# TUI logs show parsing failures
grep -i "deserialization\|error" /tmp/hamr-tui.log
```

### Configuration issues

```bash
# Search for config-related messages
grep -i "config" /tmp/hamr-daemon.log
```

## Privacy Statement

**No telemetry**: Hamr does not send any data to external servers. All logging is local to your machine.

### What logs contain

Logs may include:
- Search queries you type
- Plugin IDs and names
- File paths on your system
- Application names from your index
- Error messages and stack traces

### Before sharing logs

When reporting issues or sharing logs publicly:

1. Review log content for sensitive information
2. Redact personal file paths (e.g., `/home/username/` -> `/home/<user>/`)
3. Remove any queries that reveal private information
4. Consider using only relevant excerpts rather than full logs

Example redaction:

```bash
# Create redacted log excerpt
tail -200 /tmp/hamr-daemon.log | \
  sed "s|/home/$USER|/home/<user>|g" > hamr-debug-redacted.log
```

## Log Retention

### tmpfs behavior

Logs in `/tmp/` are typically stored on a tmpfs (RAM disk) on most Linux distributions:

- Logs are cleared on reboot
- Fast write performance
- No disk wear

### Archiving logs

To preserve logs across reboots:

```bash
# Archive current session
cp /tmp/hamr-daemon.log ~/hamr-logs/daemon-$(date +%Y%m%d).log

# Archive with compression
gzip -c /tmp/hamr-daemon.log > ~/hamr-logs/daemon-$(date +%Y%m%d).log.gz
```

### Cleaning old logs

Timestamped logs accumulate during long sessions:

```bash
# Remove logs older than 7 days
find /tmp -name "hamr-*.log" -mtime +7 -delete

# Keep only the 5 most recent of each type
ls -t /tmp/hamr-daemon-*.log | tail -n +6 | xargs rm -f
ls -t /tmp/hamr-tui-*.log | tail -n +6 | xargs rm -f
```

## See Also

- [Troubleshooting](troubleshooting.md) - Common issues and solutions
- [Configuration](configuration.md) - Hamr settings reference
- [Plugin SDK](../plugins/python-sdk.md) - Plugin development with debug options
