# AGENTS.md - Hamr Launcher Development

## Quick Reference for AI Agents

**Code Style (QML - modules/, services/):**

- do not include excessive comments
- Pragmas: `pragma Singleton` and `pragma ComponentBehavior: Bound` for singletons
- Imports: Quickshell/Qt imports first, then `qs.*` project imports
- Properties: Use `readonly property var` for computed, typed (`list<string>`, `int`) when possible
- Naming: `camelCase` for properties/functions, `id: root` for root element

**Code Style (Python - plugins/handler.py):**

- do not include excessive comments
- Imports: stdlib first (`json`, `os`, `sys`, `subprocess`, `pathlib`), then third-party
- Types: Use `list[dict]`, `str`, `bool` (Python 3.9+ style, not `List[Dict]`)
- Naming: `snake_case` functions/variables, `UPPER_SNAKE` constants
- Test mode: Check `TEST_MODE = os.environ.get("HAMR_TEST_MODE") == "1"` for mock data
- Errors: Return `{"type": "error", "message": "..."}` JSON, don't raise exceptions

---

## Project Scope

This is the **hamr** launcher - a standalone search bar / launcher for Quickshell.

## Repository

This repo lives at:

```
~/Projects/Personal/Qml/hamr/
```

Symlinked to `~/.config/quickshell/` for testing.

## Releasing

When user says "push and bump version" or "release":

1. **Check for uncommitted changes to determine if version bump should be patch version or minor version**:

    ```bash
    git status --porcelain
    ```

    If there are no features or features don't seem significant update, bump the version as a patch version (e.g., `0.1.1` -> `0.1.2`).
    Else, bump the version as a minor version (e.g., `0.1.1` -> `0.2.0`).

2. **Update version in PKGBUILD**:
    - `pkgver` - bump for code changes (e.g., `0.1.1` -> `0.2.0`)
    - `pkgrel` - bump for PKGBUILD-only changes, reset to `1` on pkgver bump

3. **Commit and push**:

    ```bash
    git add -A && git commit -m "chore: bump version to X.Y.Z" && git push
    ```

4. **Create and push tag**:

    ```bash
    git tag vX.Y.Z && git push origin vX.Y.Z
    ```

5. **GitHub Actions will automatically**:
    - Update AUR package
    - Create GitHub Release with sorted release notes (by conventional commit type)

**Manual AUR publish** (if needed):

```bash
./aur-publish.sh
```

## Testing

- Quickshell auto-reloads on file change when running in debug mode
- No manual reload needed during development
- View logs `journalctl --user -u quickshell -f`
