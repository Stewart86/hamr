#!/bin/sh
set -eu
cd "$(dirname "$0")"
PYTHONDONTWRITEBYTECODE=1 python3 -B -m unittest -v test_handler
