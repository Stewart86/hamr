# Release Checklist

Manual checklist for smoke testing before publishing a release.

## Pre-flight Checks

Run the automated checks first:

```bash
./scripts/release-check.sh
```

This script validates:

- `cargo fmt --all -- --check` (code formatting)
- `cargo clippy --all-targets -- -D warnings` (lint checks)
- `cargo test --all` (unit/integration tests)
- `cargo build --release` (release build)
- `mkdocs build --strict` (documentation)
- `python3 -m compileall -q plugins` (plugin syntax)
- Plugin checksum verification

All checks must pass before proceeding.

## Manual Smoke Tests

### 1. Daemon Startup

```bash
# Start daemon in foreground
cargo run -p hamr-daemon

# In another terminal, check socket exists
ls -la /run/user/$(id -u)/hamr.sock
```

**Expected**: Daemon starts without errors, socket file is created.

### 2. Basic Search (TUI)

```bash
# With daemon running, start TUI
cargo run -p hamr-tui
```

1. Type a query (e.g., "firefox")
2. Verify results appear
3. Press `Enter` on a result
4. Verify action executes

**Expected**: Search works, results display correctly, actions trigger.

### 3. Plugin Communication

```bash
# Test a daemon plugin (shell)
cargo run -p hamr-cli -- test shell "ls"

# Test a non-daemon plugin (calculate)
cargo run -p hamr-cli -- test calculate "2+2"
```

**Expected**: Both commands return results without errors.

### 4. Index Functionality

```bash
# View index statistics
cargo run -p hamr-cli -- index

# Test index search
cargo run -p hamr-cli -- query "shutdown"
```

**Expected**: Index stats show item counts, query returns relevant results.

### 5. Plugin Audit

```bash
cargo run -p hamr-cli -- plugins audit
```

**Expected**: All bundled plugins show `VERIFIED` status.

### 6. GTK UI (if applicable)

```bash
# Requires Wayland compositor with layer-shell support
cargo run -p hamr-gtk
```

1. Launcher window appears
2. Search works
3. Results are selectable
4. Window closes on action or Escape

**Expected**: UI renders correctly, interactions work smoothly.

### 7. Configuration Loading

```bash
# Check config is loaded
cat ~/.config/hamr/config.json

# Modify a setting and restart daemon
# Verify change takes effect
```

**Expected**: Configuration changes are reflected after daemon restart.

## Version Bump Checklist

Before tagging a release:

- [ ] Update version in `Cargo.toml` (workspace level)
- [ ] Update version in child crate `Cargo.toml` files if needed
- [ ] Update `CHANGELOG.md` with release notes
- [ ] Commit version bump: `git commit -m "chore: bump version to vX.Y.Z"`
- [ ] Create and push tag: `git tag vX.Y.Z && git push origin vX.Y.Z`

## Post-Release Verification

After the release workflow completes:

1. Check [GitHub Releases](https://github.com/stewart86/hamr/releases) for new draft
2. Verify artifacts are attached:
   - `hamr-linux-x86_64.tar.gz`
   - `hamr-linux-aarch64.tar.gz`
   - `checksums.txt`
3. Download and extract a tarball, verify binaries run
4. If using AUR: verify PKGBUILD was updated
5. Publish the draft release

## Rollback Procedure

If critical issues are found after release:

1. **Immediate**: Edit GitHub release to mark as pre-release or delete
2. **Revert tag**: `git tag -d vX.Y.Z && git push origin :refs/tags/vX.Y.Z`
3. **Fix issues** on main branch
4. **Re-tag** with same or incremented version

## Troubleshooting Release Issues

| Issue | Cause | Solution |
|-------|-------|----------|
| Workflow fails at build | Missing dependency | Check system deps in workflow |
| Checksums mismatch | Plugin files modified | Re-run `./scripts/generate-plugin-checksums.sh` |
| AUR update fails | Version format issue | Ensure tag format is `vX.Y.Z` |
| Artifacts missing | Build step failed | Check workflow logs for errors |
