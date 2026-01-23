# Plugin Security Enhancements

## Responsibility
Detect tampering with bundled plugins and document the trust model so users understand the risks of user-installed plugins.

## Requirements

### Phase 1: Checksum Generation (build-time)
- [ ] Create `scripts/generate-plugin-checksums.sh` that:
  - Iterates all `plugins/*/` directories
  - Computes SHA256 for each `manifest.json` and `handler.py`
  - Outputs `plugins/checksums.json` with format: `{"plugin-id": {"manifest.json": "sha256...", "handler.py": "sha256..."}}`
- [ ] Add checksum generation to `.github/workflows/release.yml` before packaging
- [ ] Include `checksums.json` in release artifacts

### Phase 2: Runtime Verification (daemon)
- [ ] Add checksum verification in `hamr-core` or `hamr-daemon` plugin loading:
  - Load `checksums.json` from install directory
  - Compare against actual plugin files
  - Log warning if mismatch: `WARN plugin "foo" checksum mismatch - may be modified`
- [ ] Add `--strict-plugins` flag to daemon that exits on checksum mismatch

### Phase 3: CLI Audit Command
- [ ] Add `hamr plugins audit` command in `hamr-cli` that:
  - Lists all discovered plugins with columns: ID | Path | Status (verified/modified/unknown)
  - Exits 0 if all verified, 1 if any modified/unknown
  - Example output:
    ```
    apps        ~/.local/share/hamr/plugins/apps      verified
    clipboard   ~/.local/share/hamr/plugins/clipboard modified
    my-plugin   ~/.config/hamr/plugins/my-plugin      unknown (user plugin)
    ```

### Phase 4: Documentation
- [ ] Update SECURITY.md with:
  - Plugin trust model explanation
  - How checksums work
  - How to audit plugins
- [ ] Add security section to docs

## Acceptance Criteria
- [ ] `scripts/generate-plugin-checksums.sh` produces valid JSON
- [ ] Daemon logs checksum warnings on startup when plugins modified
- [ ] `hamr plugins audit` works and shows clear status per plugin
- [ ] SECURITY.md documents the checksum system

## Edge Cases
- Missing checksums.json: log info "checksums not available", skip verification
- User plugins in ~/.config/hamr/plugins: always show as "unknown (user plugin)" - expected
- Partial mismatch (manifest ok, handler modified): report as modified

## Dependencies
- None (can be implemented incrementally)
