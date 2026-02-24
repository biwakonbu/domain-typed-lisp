#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

if ! command -v mdbook >/dev/null 2>&1; then
  echo "mdbook が見つかりません。'cargo install mdbook --locked' を実行してください。" >&2
  exit 1
fi

if ! command -v python3 >/dev/null 2>&1; then
  echo "python3 が見つかりません。" >&2
  exit 1
fi

"$SCRIPT_DIR/generate-examples-catalog.sh"
python3 "$SCRIPT_DIR/generate-glossary-assets.py"
mdbook build "$REPO_ROOT/docs-site"
echo "生成完了: $REPO_ROOT/docs-site/book/index.html"
