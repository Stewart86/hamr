# Contributing to Hamr

Contributions are welcome! Whether it's bug fixes, new plugins, documentation improvements, or feature suggestions.

## Ways to Contribute

### Bug Reports

Open an issue with:
- Steps to reproduce
- Expected vs actual behavior
- Hamr version and system info (Hyprland version, distro)
- Relevant logs from terminal output when running `qs -c hamr`

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

### QML (modules/, services/)

- Use `pragma Singleton` and `pragma ComponentBehavior: Bound` for singletons
- Imports: Qt/Quickshell imports first, then project imports (`qs.*`)
- Properties: Use `readonly property` for computed values, typed when possible
- Naming: `camelCase` for properties/functions, `id: root` for root element
- Keep comments minimal - code should be self-explanatory

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

**Note:** You can have the AUR version installed alongside development. The dev script temporarily takes over, then restores production on exit.

## Testing Plugins

```bash
# Run plugin tests
cd plugins
./test-all

# Test a specific plugin
HAMR_TEST_MODE=1 ./test-harness ./my-plugin/handler.py initial
```

## Pull Request Guidelines

- Keep PRs focused on a single change
- Include a clear description of what and why
- Reference any related issues
- Ensure existing tests still pass
- Add tests for new functionality

## Questions?

Open an issue or start a discussion. Happy to help!
