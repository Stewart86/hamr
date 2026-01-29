# Troubleshooting

This guide covers common issues and their solutions.

## Quick Diagnosis

| Symptom | Cause | Solution |
|---------|-------|----------|
| Launcher doesn't appear | Daemon not running | Run `hamr` (auto-starts daemon) or check `systemctl --user status hamr-daemon` |
| "Connection refused" error | Socket not created | Ensure daemon is running: `pgrep hamr-daemon` or restart with `hamr` |
| Window appears but immediately closes | Compositor lacks layer-shell support | Verify compositor supports layer-shell (see [Compositor Support](installation.md#compositor-support-matrix)) |
| No search results | Index not built | Run `hamr index` to rebuild the application index |
| Plugin not responding | Plugin crashed or timeout | Check logs: `grep "plugin" /tmp/hamr-daemon.log \| tail -20` |
| Slow startup | Large plugin index | Review `~/.local/share/hamr/index/` size; consider disabling unused plugins |
| Keybinding doesn't work | Compositor config issue | Verify binding syntax in compositor config; test with `hamr toggle` manually |
| GTK theme not applied | GTK4 theme missing | Install a GTK4-compatible theme; Hamr uses GTK4, not GTK3 |

## Daemon Issues

### Daemon won't start

1. Check if another instance is running:
   ```bash
   pgrep -a hamr-daemon
   ```

2. Check for socket conflicts:
   ```bash
   ls -la /tmp/hamr-*.sock
   # Remove stale socket if daemon isn't running
   rm /tmp/hamr-*.sock
   ```

3. Check logs for errors:
   ```bash
   tail -50 /tmp/hamr-daemon.log
   ```

### Daemon crashes on startup

Common causes:

- **Missing GTK4**: Install GTK4 4.20+ and gtk4-layer-shell
- **Missing plugins directory**: Run `hamr install` to set up directories
- **Corrupt config**: Move `~/.config/hamr/config.json` and restart

## Plugin Issues

### Plugin not found

1. Verify plugin exists:
   ```bash
   ls ~/.local/share/hamr/plugins/
   ```

2. Check plugin manifest is valid JSON:
   ```bash
   cat ~/.local/share/hamr/plugins/<plugin>/manifest.json | python -m json.tool
   ```

3. Ensure plugin has execute permission:
   ```bash
   chmod +x ~/.local/share/hamr/plugins/<plugin>/main.py
   ```

### Plugin returns no results

1. Test plugin directly:
   ```bash
   hamr test <plugin> "your query"
   ```

2. Check plugin logs:
   ```bash
   grep "<plugin>" /tmp/hamr-daemon.log | tail -30
   ```

3. Enable plugin debug mode:
   ```bash
   HAMR_PLUGIN_DEBUG=1 hamr daemon
   ```

### Plugin timeout

Plugins have a default timeout. If a plugin is slow:

1. Check if external service is reachable (for network plugins)
2. Review plugin code for blocking operations
3. Consider increasing timeout in plugin manifest

## Display Issues

### Window appears in wrong position

Layer-shell positioning depends on compositor support:

1. Ensure gtk4-layer-shell is installed
2. Check compositor-specific settings for layer positioning
3. Try different anchor settings in config

### Window doesn't receive keyboard focus

1. Verify layer-shell is working:
   ```bash
   # Should show layer-shell in use
   grep -i "layer" /tmp/hamr-daemon.log
   ```

2. Some compositors require explicit focus rules for layer surfaces

### Blurry or scaled incorrectly

GTK4 respects `GDK_SCALE` environment variable:

```bash
# Force 1x scaling
GDK_SCALE=1 hamr-gtk

# Force 2x scaling
GDK_SCALE=2 hamr-gtk
```

## Performance Issues

### Slow search results

1. Check index size:
   ```bash
   du -sh ~/.local/share/hamr/index/
   ```

2. Rebuild index:
   ```bash
   hamr index --rebuild
   ```

3. Disable plugins you don't use in config

### High memory usage

1. Check plugin count:
   ```bash
   ls ~/.local/share/hamr/plugins/ | wc -l
   ```

2. Disable daemon plugins that aren't needed
3. Check for plugin memory leaks in logs

## Log Files

Debug builds write logs to `/tmp/`:

| Component | Log Path | Symlink |
|-----------|----------|---------|
| Daemon | `/tmp/hamr-daemon-YYYYMMDD_HHMMSS.log` | `/tmp/hamr-daemon.log` |
| TUI | `/tmp/hamr-tui-YYYYMMDD_HHMMSS.log` | `/tmp/hamr-tui.log` |

### Enable verbose logging

```bash
# Most verbose
RUST_LOG=trace hamr daemon

# Debug for hamr crates only
RUST_LOG=hamr=debug hamr daemon

# Warnings and errors only
RUST_LOG=warn hamr daemon
```

### Search logs for errors

```bash
# Find all errors
grep -i "error\|warn" /tmp/hamr-daemon.log

# Plugin-specific issues
grep "plugin" /tmp/hamr-daemon.log | tail -20

# Action handling
grep "handle_action\|Forwarding" /tmp/hamr-daemon.log
```

## Getting Help

If these steps don't resolve your issue:

1. Check existing [GitHub Issues](https://github.com/stewart86/hamr/issues)
2. Collect logs: `tail -200 /tmp/hamr-daemon.log > hamr-debug.log`
3. Open a new issue with:
   - Hamr version (`hamr --version`)
   - Compositor and version
   - Distribution
   - Relevant log output
