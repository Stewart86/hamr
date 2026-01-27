# Contributing to Hamr

Contributions are welcome! Whether it's bug fixes, new plugins, documentation improvements, or feature suggestions.

## Ways to Contribute

### Bug Reports

Open an issue with:
- Steps to reproduce
- Expected vs actual behavior
- Hamr version and system info (Hyprland version, distro)
- Relevant logs from `/tmp/hamr-daemon.log` and `/tmp/hamr-tui.log`

### Plugin Contributions

New plugins are always welcome. See [`plugins/README.md`](plugins/README.md) for the full protocol reference.

**Requirements:**
- Include a `test.sh` that validates your plugin (required)
- Ensure `HAMR_TEST_MODE=1` returns mock data (no real API calls in tests)
- Follow the existing code style (see below)
- Update `manifest.json` with clear name, description, and icon

Plugins without tests will not be accepted.

### Code Contributions

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/my-feature`)
3. Make your changes
4. Test thoroughly
5. Submit a pull request

## Code Style

### Rust (crates/)

- Edition 2024 with workspace-level dependency management
- Imports: `crate::` first, then `std::`, then external crates, then `tracing::{...}`
- Use `thiserror` for library errors with `Error` enum and `Result<T>` alias
- Use `#[serde(rename_all = "camelCase")]` for JSON protocol types
- Async: `tokio` runtime with `Arc<Mutex<T>>` for shared state
- Tests: `#[tokio::test]` with fixtures in `tests/fixtures.rs`
- Avoid inline comments - code should be self-documenting

See `AGENTS.md` for detailed code style guidelines.

### Python (plugins/)

- Imports: stdlib first, then third-party
- Types: Use Python 3.9+ style (`list[dict]`, not `List[Dict]`)
- Naming: `snake_case` for functions/variables, `UPPER_SNAKE` for constants
- Check `HAMR_TEST_MODE` environment variable for mock data in tests
- Return `{"type": "error", "message": "..."}` for errors, don't raise exceptions

## Development Setup

```bash
# Clone the repository
git clone https://github.com/stewart86/hamr.git
cd hamr

# Run in development mode
./dev
```

The `./dev` script:
- Stops any running production hamr (systemd service or manual)
- Runs hamr from the current directory with live reload
- Restores production hamr on exit (Ctrl+C)

Use `./dev --no-restore` to not restart production after exiting.

Hamr auto-reloads on file changes. Logs appear directly in your terminal.

Dev builds use a separate socket at `$XDG_RUNTIME_DIR/hamr-dev.sock`. The release
`hamr toggle` command will attempt the dev socket first so your compositor binding
still works while you run `cargo run -p hamr-daemon` and `cargo run -p hamr-gtk`.

The GTK dev build uses a separate application ID (`org.hamr.Launcher.Dev`) so it
can run alongside the production launcher during development.

**Note:** You can have the AUR version installed alongside development. The dev script temporarily takes over, then restores production on exit.

## Testing Plugins

Follow this checklist when developing or modifying plugins:

### Pre-flight Checks

1. **Verify manifest format** - Ensure `manifest.json` is valid JSON with required fields
2. **Check Python syntax** - Run `python3 -m py_compile handler.py`
3. **Audit plugin integrity** - Run `hamr plugins audit` to verify checksum status

### Manual Testing Workflow

4. **Start the daemon in a terminal:**
   ```bash
   cargo run -p hamr-daemon
   ```

5. **Open a second terminal for logs:**
   ```bash
   tail -f /tmp/hamr-daemon.log
   ```

6. **Test plugin with CLI:**
   ```bash
   # Test initial step
   cargo run -p hamr-cli -- test my-plugin ""

   # Test search step
   cargo run -p hamr-cli -- test my-plugin "search query"
   ```

7. **Test with TUI for interactive validation:**
   ```bash
   cargo run -p hamr-tui
   # Type your plugin prefix (e.g., "my:") to activate
   ```

8. **Enable debug logging if needed:**
   ```bash
   HAMR_PLUGIN_DEBUG=1 cargo run -p hamr-daemon
   RUST_LOG=debug cargo run -p hamr-daemon
   ```

### Verification Checklist

- [ ] Plugin activates with correct prefix
- [ ] Search results appear as expected
- [ ] Actions (copy, open URL, launch) work correctly
- [ ] Error states display meaningful messages
- [ ] No Python exceptions in daemon logs
- [ ] Plugin responds within reasonable time (<100ms for search)

See `plugins/sdk/README.md` for response types and SDK usage.

## Pull Request Guidelines

- Keep PRs focused on a single change
- Include a clear description of what and why
- Reference any related issues
- Ensure existing tests still pass
- Add tests for new functionality

## Release Process

For maintainers preparing a release, see the [Release Checklist](docs/releases/checklist.md) which covers:

- Automated pre-flight checks (`./scripts/release-check.sh`)
- Manual smoke tests (daemon, TUI, plugins, GTK)
- Version bump procedure
- Post-release verification

## Questions?

Open an issue or start a discussion. Happy to help!
