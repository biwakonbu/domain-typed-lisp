#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

if ! command -v mdbook >/dev/null 2>&1; then
  echo "mdbook が見つかりません。'cargo install mdbook --locked' を実行してください。" >&2
  exit 1
fi

"$SCRIPT_DIR/generate-examples-catalog.sh"
mdbook serve "$REPO_ROOT/docs-site" --open
