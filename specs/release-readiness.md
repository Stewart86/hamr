# Release Readiness Checklist

## Responsibility
Define and document the manual validation steps required before tagging a release, plus provide a helper script for the automated checks.

## Requirements
- [ ] Create `docs/releases/checklist.md` (or similar) enumerating manual smoke tests: `cargo build --release`, run hamr-daemon + hamr-gtk on a supported compositor, verify installer round-trip, confirm docs build, review plugin security warnings.
- [ ] Add `scripts/release-check.sh` to run the automated prerequisites (`cargo fmt --all -- --check`, `cargo clippy --all-targets`, `cargo test --all`, `mkdocs build`, optional `python -m compileall plugins`).
- [ ] Update CONTRIBUTING and release workflow docs to reference the checklist and require maintainers to confirm completion before tagging or approving releases.
- [ ] Ensure GitHub release workflow description/comments remind maintainers to complete the checklist (manual confirmation step).

## Acceptance Criteria
- [ ] A single authoritative checklist document exists and is linked from README/CONTRIBUTING.
- [ ] `scripts/release-check.sh` exits non-zero on validation failures and is referenced in docs.
- [ ] Release workflow and documentation explicitly call out the manual verification steps.

## Edge Cases
- Checklist should allow documenting when certain compositor/manual tests cannot be run (e.g., not on KDE) so the omission is intentional.

## Dependencies
- Docs refresh for linking to new release content.
