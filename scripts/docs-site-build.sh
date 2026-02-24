#!/usr/bin/env bash
set -euo pipefail

if ! command -v mdbook >/dev/null 2>&1; then
  echo "mdbook が見つかりません。'cargo install mdbook --locked' を実行してください。" >&2
  exit 1
fi

mdbook build docs-site
echo "生成完了: docs-site/book/index.html"
