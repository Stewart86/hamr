# Testing Plugins

The best way to test your plugin is to **use it directly in Hamr**. Hamr captures and displays JSON errors in the UI, making visual testing the most effective approach.

## Running Hamr in Dev Mode

For plugin development, run Hamr in dev mode from the repository:

```bash
cd ~/path/to/hamr
./dev
```

Dev mode:

- Stops any running production Hamr
- Runs Hamr from the current directory
- Auto-reloads when plugin files change
- Shows logs directly in the terminal
- Restores production Hamr on exit (Ctrl+C)

Use `./dev --no-restore` to not restart production Hamr after exiting.

## Visual Testing

### Basic Testing Workflow

1. **Run Hamr in dev mode** - `./dev` from the hamr directory
2. **Open Hamr** and type `/your-plugin-name`
3. **Interact with your plugin** - type searches, select items, click actions
4. **Errors are shown in the UI** - invalid JSON, missing fields, Python exceptions
5. **Edit your plugin** - Hamr auto-reloads on file changes
6. **Check terminal for logs** - errors and debug output appear in the dev terminal

### Alternative: Check Logs Separately

If running Hamr as a systemd service instead of dev mode:

```bash
# Watch logs while testing
journalctl --user -u hamr -f
```

### Manual Handler Testing

Test your handler directly from the command line:

```bash
# Test initial step
echo '{"step": "initial"}' | ./handler.py

# Test search
echo '{"step": "search", "query": "test"}' | ./handler.py

# Test action
echo '{"step": "action", "selected": {"id": "item-1"}}' | ./handler.py

# Validate JSON output
echo '{"step": "initial"}' | ./handler.py | jq .
```

---

## Schema Requirements

Hamr validates all responses. Invalid responses show errors in the UI.

| Response Type  | Required Fields                                    |
| -------------- | -------------------------------------------------- |
| `results`      | `type`, `results[]` with `id` and `name`           |
| `card`         | `type`, `card.content`                             |
| `execute`      | `type`                                             |
| `imageBrowser` | `type`, `imageBrowser.directory`                   |
| `gridBrowser`  | `type`, `gridBrowser.items[]` with `id` and `name` |
| `form`         | `type`, `form.fields[]` with `id`, `type`          |
| `prompt`       | `type`, `prompt` object                            |
| `error`        | `type`, `message`                                  |
| `noop`         | `type` only                                        |

---

## Debugging Tips

### Check Handler Output

```bash
# Run handler and validate JSON
echo '{"step": "initial"}' | ./handler.py | jq .

# Check specific field
echo '{"step": "initial"}' | ./handler.py | jq '.results[0].id'
```

### Check Hamr Logs

```bash
# Follow logs in real-time
journalctl --user -u hamr -f

# Show recent errors
journalctl --user -u hamr --since "5 min ago" | grep -i error
```

### Common Issues

| Issue                 | Solution                                                       |
| --------------------- | -------------------------------------------------------------- |
| Plugin doesn't appear | Check `supportedCompositors` in manifest                       |
| Handler errors        | Check `journalctl --user -u hamr -f`                           |
| Invalid JSON          | Test with `echo '{"step": "initial"}' \| ./handler.py \| jq .` |
| Missing fields        | Check schema requirements above                                |
