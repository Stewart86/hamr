Work on ONE task denoted with a checkbox `[ ]` and end response normally.

## Workflow
1. Study `progress.txt` (Codebase Patterns first)
2. Study `IMPLEMENTATION_PLAN.md`
3. Choose the next task based on dependencies/impact (your decision)
4. Implement that single task
5. Run quality checks:
   - `cargo fmt --all -- --check`
   - `cargo clippy --all-targets`
   - `cargo test --all`
   - `mkdocs build`
   - `python -m compileall plugins` (skip only when SDK/plugins untouched and note it)
6. If all pass:
   - Mark task `[x]` in `IMPLEMENTATION_PLAN.md`
   - Append entry to `progress.txt`
   - Commit: `git add -A && git commit -m "feat: [Task ID] - [description]"`
7. If checks fail: fix and rerun until green (no commits)

## Quality Requirements
- Commits must pass the validation commands above
- No broken builds or tests on main
- Follow existing patterns, keep scope tight

## Progress Log Format
Append to `progress.txt` per completed task:
```
## [Task ID] - DONE
- What: [Brief description]
- Files: [Changed files]
- Learned: [Patterns/gotchas]
---
```

If stuck:
1. Log attempts & failure reason
2. Try alternative approach
3. If still blocked, split/defer task in `IMPLEMENTATION_PLAN.md`
4. Stop without commit or progress entry; next iteration will continue

## Codebase Patterns
Keep discoveries at top of `progress.txt` under **Codebase Patterns**:
```
## Codebase Patterns
- pattern
```

## Stop Condition
- Persistent blocker? stop without marking task done
- Remaining `[ ]` tasks? end response normally
- All tasks `[x]`? output `<promise>COMPLETE</promise>` on its own line

## Project Info
```
crates/
  hamr-core/
  hamr-daemon/
  hamr-gtk/
  hamr-tui/
  hamr-cli/
  hamr-rpc/
  hamr-types/
plugins/
docs/
...
```

Validation: `cargo fmt --all -- --check`, `cargo clippy --all-targets`, `cargo test --all`, `mkdocs build`, `python -m compileall plugins`

## Important
- One task per iteration
- Mark task done only after validation
- Commit once per task
- Keep CI green
- Always read Codebase Patterns before coding
