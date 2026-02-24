#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
EXAMPLES_DIR="$REPO_ROOT/examples"
CATALOG_DEF="$EXAMPLES_DIR/catalog.tsv"
OUT_FILE="$REPO_ROOT/docs-site/src/tutorial/examples-catalog.md"

if [ ! -d "$EXAMPLES_DIR" ]; then
  echo "examples ディレクトリが見つかりません: $EXAMPLES_DIR" >&2
  exit 1
fi

if [ ! -f "$CATALOG_DEF" ]; then
  echo "カタログ定義が見つかりません: $CATALOG_DEF" >&2
  exit 1
fi

render_table() {
  local rows="$1"
  echo "| ファイル | 主用途 | 実行例 |"
  echo "| --- | --- | --- |"
  if [ -n "$rows" ]; then
    printf "%s" "$rows"
  else
    echo "| - | 該当なし | - |"
  fi
  echo
}

category_title() {
  case "$1" in
    first)
      echo "最初に触る"
      ;;
    dup)
      echo "重複判定（lint/semantic-dup）"
      ;;
    multi)
      echo "複数ファイル運用（import）"
      ;;
    recursive)
      echo "停止性・再帰"
      ;;
    error)
      echo "エラー再現（診断確認）"
      ;;
    *)
      return 1
      ;;
  esac
}

append_row_for_category() {
  local category="$1"
  local row="$2"
  case "$category" in
    first)
      rows_first+="$row"
      ;;
    dup)
      rows_dup+="$row"
      ;;
    multi)
      rows_multi+="$row"
      ;;
    recursive)
      rows_recursive+="$row"
      ;;
    error)
      rows_error+="$row"
      ;;
    *)
      return 1
      ;;
  esac
}

rows_for_category() {
  case "$1" in
    first)
      printf "%s" "$rows_first"
      ;;
    dup)
      printf "%s" "$rows_dup"
      ;;
    multi)
      printf "%s" "$rows_multi"
      ;;
    recursive)
      printf "%s" "$rows_recursive"
      ;;
    error)
      printf "%s" "$rows_error"
      ;;
    *)
      return 1
      ;;
  esac
}

is_seen_file() {
  local target="$1"
  local item
  for item in "${seen_files[@]-}"; do
    if [ -z "$item" ]; then
      continue
    fi
    if [ "$item" = "$target" ]; then
      return 0
    fi
  done
  return 1
}

section_order=(first dup multi recursive error)

rows_first=""
rows_dup=""
rows_multi=""
rows_recursive=""
rows_error=""
seen_files=()

line_no=0
current_category=""
while IFS= read -r raw_line || [ -n "$raw_line" ]; do
  line_no=$((line_no + 1))
  line="${raw_line%$'\r'}"

  if [[ -z "${line//[[:space:]]/}" ]]; then
    continue
  fi

  if [[ "$line" =~ ^[[:space:]]*# ]]; then
    continue
  fi

  if [[ "$line" =~ ^\[([a-z0-9_-]+)\][[:space:]]*(.*)$ ]]; then
    category="${BASH_REMATCH[1]}"
    if ! category_title "$category" >/dev/null 2>&1; then
      echo "catalog.tsv:${line_no}: 未知の section です: $category" >&2
      exit 1
    fi
    current_category="$category"
    continue
  fi

  if [ -z "$current_category" ]; then
    echo "catalog.tsv:${line_no}: セクション見出し [first|dup|multi|recursive|error] より前に行データがあります" >&2
    exit 1
  fi

  IFS=$'\t' read -r file purpose command extra <<<"$line"

  if [ -n "$extra" ]; then
    echo "catalog.tsv:${line_no}: 列数が不正です（期待: 3列TSV）" >&2
    exit 1
  fi

  if [ -z "$file" ] || [ -z "$purpose" ] || [ -z "$command" ]; then
    echo "catalog.tsv:${line_no}: file/purpose/command は必須です" >&2
    exit 1
  fi

  if is_seen_file "$file"; then
    echo "catalog.tsv:${line_no}: 重複定義です: $file" >&2
    exit 1
  fi

  if [ ! -f "$EXAMPLES_DIR/$file" ]; then
    echo "catalog.tsv:${line_no}: examples/$file が存在しません" >&2
    exit 1
  fi

  printf -v row '| `%s` | %s | `%s` |\n' "examples/$file" "$purpose" "$command"
  append_row_for_category "$current_category" "$row"
  seen_files+=("$file")
done <"$CATALOG_DEF"

while IFS= read -r path; do
  filename="$(basename "$path")"
  if ! is_seen_file "$filename"; then
    echo "catalog.tsv: examples/$filename の定義が不足しています" >&2
    exit 1
  fi
done < <(find "$EXAMPLES_DIR" -maxdepth 1 -type f -name "*.dtl" | sort)

{
  echo "# 利用例カタログ"
  echo
  echo "<!-- このファイルは scripts/generate-examples-catalog.sh と examples/catalog.tsv から自動生成されます。 -->"
  echo
  echo "\`examples/\` に同梱しているサンプルを、用途別に引けるように整理した一覧です。"
  echo
  for category in "${section_order[@]}"; do
    title="$(category_title "$category")"
    rows="$(rows_for_category "$category")"
    echo "## $title"
    echo
    render_table "$rows"
    echo
  done
  echo "## 使い分け指針"
  echo
  echo "1. CLI 動作確認だけなら \`my_first_policy\` か \`access_control_ok\`。"
  echo "2. CI 連携の JSON 契約確認なら \`customer_contract_ja\` か \`complex_policy_import_entry\`。"
  echo "3. lint の厳密重複判定を試すなら \`semantic_dup_advanced\` と \`semantic_dup_function_param\`。"
} >"$OUT_FILE"

echo "生成完了: $OUT_FILE"
