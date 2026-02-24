#!/usr/bin/env bash
set -euo pipefail

if ! command -v mdbook >/dev/null 2>&1; then
  echo "mdbook が見つかりません。'cargo install mdbook --locked' を実行してください。" >&2
  exit 1
fi

mdbook serve docs-site --open
