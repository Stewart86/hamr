#!/usr/bin/env bash
# OCR indexing script wrapper
# Uses system python with click (minimal dependencies)

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Try venv first, fall back to system python
if [[ -n "$ILLOGICAL_IMPULSE_VIRTUAL_ENV" ]] && [[ -f "$(eval echo $ILLOGICAL_IMPULSE_VIRTUAL_ENV)/bin/activate" ]]; then
    source "$(eval echo $ILLOGICAL_IMPULSE_VIRTUAL_ENV)/bin/activate"
    "$SCRIPT_DIR/ocr-index.py" "$@"
    deactivate
else
    # Use system python - click is usually available or easy to install
    python3 "$SCRIPT_DIR/ocr-index.py" "$@"
fi
