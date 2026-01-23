# Release Readiness Checklist

## Responsibility
Define and document the manual validation steps required before tagging a release, plus provide a helper script for the automated checks.

## Requirements

### Automated Script
- [ ] Create `scripts/release-check.sh` that runs:
  ```bash
  #!/usr/bin/env bash
  set -euo pipefail
  echo "==> Running cargo fmt check..."
  cargo fmt --all -- --check
  echo "==> Running clippy..."
  cargo clippy --all-targets -- -D warnings
  echo "==> Running tests..."
  cargo test --all
  echo "==> Building release..."
  cargo build --release
  echo "==> Building docs..."
  mkdocs build
  echo "==> Compiling plugins (optional)..."
  python -m compileall plugins || echo "Python compile check skipped"
  echo "==> All checks passed!"
  ```
- [ ] Make script executable and add to repo

### Manual Checklist Document
- [ ] Create `docs/releases/checklist.md` with:
  1. Run `scripts/release-check.sh` - all checks pass
  2. Start daemon: `cargo run -p hamr-daemon --release`
  3. Start GTK: `cargo run -p hamr-gtk --release`
  4. Test basic search: type "firefox" - results appear
  5. Test plugin: type "2+2" - calculator shows result
  6. Test action: select result and press Enter - action executes
  7. Test installer: `./install.sh --check` shows expected output
  8. Review CHANGELOG.md for this version
  9. Verify version numbers in Cargo.toml match release tag

### Integration
- [ ] Add "Release Checklist" link to CONTRIBUTING.md
- [ ] Add comment to `.github/workflows/release.yml` referencing checklist
- [ ] Add checklist link to README.md development section

## Acceptance Criteria
- [ ] `scripts/release-check.sh` exists and exits 0 on healthy repo
- [ ] `docs/releases/checklist.md` has numbered steps maintainer can follow
- [ ] CONTRIBUTING.md links to checklist
- [ ] `mkdocs build` succeeds with new docs page

## Edge Cases
- Checklist should have "N/A" column for tests that can't run in certain environments (e.g., no Wayland)

## Dependencies
- None (can be created independently)
