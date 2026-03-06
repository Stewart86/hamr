#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

usage() {
    cat <<'EOF'
Usage: ./scripts/release.sh X.Y.Z [--update-deps]

Prepare a release commit and annotated tag.

This script:
  - updates the workspace version in Cargo.toml
  - regenerates checked-in AUR metadata from templates
  - refreshes Cargo.lock with cargo update -w
  - optionally refreshes external dependencies with cargo update
  - runs cargo build --locked and cargo test -q
  - creates the release commit and annotated tag

After it succeeds, push with:
  git push && git push --tags
EOF
}

fail() {
    printf 'error: %s\n' "$1" >&2
    exit 1
}

version=""
update_deps=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --update-deps)
            update_deps=true
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            if [[ -z "$version" ]]; then
                version="$1"
                shift
            else
                fail "unexpected argument: $1"
            fi
            ;;
    esac
done

[[ -n "$version" ]] || {
    usage
    exit 1
}

[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || fail "version must look like X.Y.Z"

cd "$REPO_ROOT"

[[ -z "$(git status --porcelain)" ]] || fail "working tree must be clean before cutting a release"

git rev-parse -q --verify "refs/tags/v$version" >/dev/null && fail "tag v$version already exists"

current_version="$({
python3 - <<'PY'
import tomllib
from pathlib import Path

data = tomllib.loads(Path("Cargo.toml").read_text(encoding="utf-8"))
print(data["workspace"]["package"]["version"])
PY
})"

[[ "$current_version" != "$version" ]] || fail "Cargo.toml is already set to $version"

python3 - <<'PY' "$version"
import re
import sys
from pathlib import Path

version = sys.argv[1]
path = Path("Cargo.toml")
text = path.read_text(encoding="utf-8")
updated, count = re.subn(
    r'(?m)^(version\s*=\s*")[^"]+("\s*)$',
    rf'\g<1>{version}\2',
    text,
    count=1,
)
if count != 1:
    raise SystemExit("failed to update workspace version in Cargo.toml")
path.write_text(updated, encoding="utf-8")
PY

python3 "$SCRIPT_DIR/render-aur-metadata.py" --version "$version" --in-place

cargo update -w

if [[ "$update_deps" == true ]]; then
    cargo update
fi

cargo build --locked
cargo test -q

git add Cargo.toml Cargo.lock pkg/aur/PKGBUILD pkg/aur/.SRCINFO pkg/aur-bin/PKGBUILD pkg/aur-bin/.SRCINFO
git diff --cached --quiet && fail "no release changes were staged"

git commit -m "chore: release v$version"
git tag -a "v$version" -m "v$version"

printf 'created release commit and tag v%s\n' "$version"
printf 'next: git push && git push --tags\n'
