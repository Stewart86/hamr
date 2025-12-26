---
name: release
description: Create a new release for the Hamr launcher with version bump, git tag, and push
license: MIT
compatibility: opencode
metadata:
  project: hamr
  type: release-management
---

# Creating a Hamr Release

This skill helps you create releases for the Hamr launcher. It handles version bumping, git tagging, and pushing to trigger GitHub Actions.

## Release Process

### 1. Check Git Log for Changes Since Last Release

```bash
git log --oneline $(git describe --tags --abbrev=0)..HEAD
```

Review commit messages to:
- Determine version bump type (patch vs minor)
- **Check if any new plugins were added** (look for commits mentioning new plugins in `plugins/` directory)

### 2. Check for New Plugins and Update README

If a new plugin was added, verify it is documented in `README.md`:

1. List all plugin directories:
   ```bash
   ls -d plugins/*/
   ```

2. Check the "Built-in Plugins" table in `README.md` (around line 66-96)

3. If a new plugin is missing from the table, add it in alphabetical order:
   ```markdown
   | `plugin-name` | Description of what the plugin does |
   ```

4. Read the plugin's `manifest.json` to get the correct name and description:
   ```bash
   cat plugins/<plugin-name>/manifest.json
   ```

The Built-in Plugins table format:
```markdown
| Plugin | Description |
|--------|-------------|
| `apps` | App drawer with categories (like rofi/dmenu) |
| `new-plugin` | Description from manifest.json |
```

### 3. Check for Uncommitted Changes

```bash
git status --porcelain
```

Analyze changes to determine version bump type:
- **Patch** (e.g., `0.1.1` -> `0.1.2`): Bug fixes, minor updates, no new features
- **Minor** (e.g., `0.1.1` -> `0.2.0`): New features or significant changes (including new plugins)

### 4. Get Current Version

Read the current version from PKGBUILD:

```bash
grep '^pkgver=' PKGBUILD
```

### 5. Update PKGBUILD Version

Edit the `pkgver` line in PKGBUILD:

```bash
# Example: Update from 0.1.1 to 0.2.0
sed -i 's/pkgver=.*/pkgver=0.2.0/' PKGBUILD
```

Version rules:
- `pkgver` - Bump for code changes
- `pkgrel` - Reset to `1` when bumping pkgver, only increment for PKGBUILD-only changes

### 6. Commit and Push

```bash
git add -A && git commit -m "chore: bump version to X.Y.Z" && git push
```

### 7. Create and Push Tag

```bash
git tag vX.Y.Z && git push origin vX.Y.Z
```

The tag format is `vX.Y.Z` (with `v` prefix).

## What Happens After Release

GitHub Actions will automatically:
1. Update the AUR package
2. Create a GitHub Release with sorted release notes (by conventional commit type)

## Manual AUR Publish

If GitHub Actions fails or manual publish is needed:

```bash
./aur-publish.sh
```

## Version Bump Decision Guide

| Change Type | Version Bump | Example |
|-------------|--------------|---------|
| Bug fixes only | Patch | `0.1.1` -> `0.1.2` |
| Minor improvements | Patch | `0.1.1` -> `0.1.2` |
| New plugin | Minor | `0.1.1` -> `0.2.0` |
| New feature | Minor | `0.1.1` -> `0.2.0` |
| Breaking changes | Minor | `0.1.1` -> `0.2.0` |
| Major rewrite | Major | `0.1.1` -> `1.0.0` |

## Complete Release Commands

```bash
# 1. Check git log for changes since last release
git log --oneline $(git describe --tags --abbrev=0)..HEAD

# 2. List plugins and compare with README.md Built-in Plugins table
ls -d plugins/*/

# 3. If new plugin found, read its manifest and add to README.md
cat plugins/<new-plugin>/manifest.json
# Then edit README.md to add the plugin in alphabetical order

# 4. Check status
git status --porcelain

# 5. Update version in PKGBUILD (edit pkgver= line)

# 6. Commit and push
git add -A && git commit -m "chore: bump version to X.Y.Z" && git push

# 7. Tag and push
git tag vX.Y.Z && git push origin vX.Y.Z
```

## Checklist

Before releasing:
- [ ] Git log reviewed for new plugins
- [ ] README.md updated with any new plugins (in alphabetical order in Built-in Plugins table)
- [ ] All changes committed
- [ ] Tests passing (if applicable)
- [ ] PKGBUILD `pkgver` updated
- [ ] PKGBUILD `pkgrel` reset to `1`

After releasing:
- [ ] GitHub Actions workflow completed
- [ ] AUR package updated
- [ ] GitHub Release created
