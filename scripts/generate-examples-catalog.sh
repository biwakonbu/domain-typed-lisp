#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
EXAMPLES_DIR="$REPO_ROOT/examples"
OUT_FILE="$REPO_ROOT/docs-site/src/tutorial/examples-catalog.md"

if [ ! -d "$EXAMPLES_DIR" ]; then
  echo "examples ディレクトリが見つかりません: $EXAMPLES_DIR" >&2
  exit 1
fi

category_for() {
  case "$1" in
    my_first_policy.dtl|access_control_ok.dtl|customer_contract_ja.dtl)
      echo "first"
      ;;
    semantic_dup_*.dtl)
      echo "dup"
      ;;
    access_control_import_entry.dtl|access_control_split_*.dtl|complex_policy_*.dtl)
      echo "multi"
      ;;
    recursive_*.dtl)
      echo "recursive"
      ;;
    access_control_ng_*.dtl|*_ng_*.dtl)
      echo "error"
      ;;
    *)
      echo "uncategorized"
      ;;
  esac
}

purpose_for() {
  case "$1" in
    my_first_policy.dtl)
      echo "最小構成（sort/relation/fact/defn/assert/universe）"
      ;;
    access_control_ok.dtl)
      echo "基本的なアクセス制御（単一ファイル）"
      ;;
    customer_contract_ja.dtl)
      echo "日本語識別子を含む実運用寄りサンプル"
      ;;
    semantic_dup_advanced.dtl)
      echo "rule/assert/defn 横断の L-DUP-MAYBE 検証"
      ;;
    semantic_dup_function_param.dtl)
      echo "function 型引数を含む defn 同値比較"
      ;;
    access_control_import_entry.dtl)
      echo "import 経由エントリの基本形"
      ;;
    access_control_split_schema.dtl)
      echo "分割スキーマ定義（entry から参照）"
      ;;
    access_control_split_policy.dtl)
      echo "分割ポリシー定義（entry から参照）"
      ;;
    complex_policy_import_entry.dtl)
      echo "複雑シナリオのエントリ"
      ;;
    complex_policy_schema.dtl)
      echo "複雑シナリオの分割スキーマ（entry から参照）"
      ;;
    complex_policy_rules.dtl)
      echo "複雑シナリオの分割規則（entry から参照）"
      ;;
    recursive_totality_ok.dtl)
      echo "構造再帰の受理ケース"
      ;;
    recursive_nested_ok.dtl)
      echo "ネスト match + let alias の構造減少"
      ;;
    access_control_ng_unknown_relation.dtl)
      echo "E-RESOLVE 系の失敗確認"
      ;;
    *)
      echo "用途未分類（要カタログ更新）"
      ;;
  esac
}

command_for() {
  case "$1" in
    my_first_policy.dtl)
      echo "cargo run -- check examples/my_first_policy.dtl --format json"
      ;;
    access_control_ok.dtl)
      echo "cargo run -- prove examples/access_control_ok.dtl --format json --out out_access"
      ;;
    customer_contract_ja.dtl)
      echo "cargo run -- doc examples/customer_contract_ja.dtl --out out_ja_doc --format markdown"
      ;;
    semantic_dup_advanced.dtl)
      echo "cargo run -- lint examples/semantic_dup_advanced.dtl --format json --semantic-dup"
      ;;
    semantic_dup_function_param.dtl)
      echo "cargo run -- lint examples/semantic_dup_function_param.dtl --format json --semantic-dup"
      ;;
    access_control_import_entry.dtl|access_control_split_schema.dtl|access_control_split_policy.dtl)
      echo "cargo run -- check examples/access_control_import_entry.dtl --format json"
      ;;
    complex_policy_import_entry.dtl|complex_policy_schema.dtl|complex_policy_rules.dtl)
      echo "cargo run -- prove examples/complex_policy_import_entry.dtl --format json --out out_complex"
      ;;
    recursive_totality_ok.dtl)
      echo "cargo run -- check examples/recursive_totality_ok.dtl --format json"
      ;;
    recursive_nested_ok.dtl)
      echo "cargo run -- prove examples/recursive_nested_ok.dtl --format json --out out_recursive"
      ;;
    access_control_ng_unknown_relation.dtl)
      echo "cargo run -- check examples/access_control_ng_unknown_relation.dtl --format json"
      ;;
    *)
      echo "cargo run -- check examples/$1 --format json"
      ;;
  esac
}

append_row() {
  local category="$1"
  local file="$2"
  local purpose="$3"
  local command="$4"
  local row
  printf -v row '| `%s` | %s | `%s` |\n' "$file" "$purpose" "$command"

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
    uncategorized)
      rows_uncategorized+="$row"
      ;;
  esac
}

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

rows_first=""
rows_dup=""
rows_multi=""
rows_recursive=""
rows_error=""
rows_uncategorized=""

while IFS= read -r path; do
  filename="$(basename "$path")"
  file="examples/$filename"
  category="$(category_for "$filename")"
  purpose="$(purpose_for "$filename")"
  command="$(command_for "$filename")"
  append_row "$category" "$file" "$purpose" "$command"
done < <(find "$EXAMPLES_DIR" -maxdepth 1 -type f -name "*.dtl" | sort)

{
  echo "# 利用例カタログ"
  echo
  echo "<!-- このファイルは scripts/generate-examples-catalog.sh で自動生成されます。 -->"
  echo
  echo "\`examples/\` に同梱しているサンプルを、用途別に引けるように整理した一覧です。"
  echo
  echo "## 最初に触る"
  echo
  render_table "$rows_first"
  echo "## 重複判定（lint/semantic-dup）"
  echo
  render_table "$rows_dup"
  echo "## 複数ファイル運用（import）"
  echo
  render_table "$rows_multi"
  echo "## 停止性・再帰"
  echo
  render_table "$rows_recursive"
  echo "## エラー再現（診断確認）"
  echo
  render_table "$rows_error"
  if [ -n "$rows_uncategorized" ]; then
    echo "## 未分類（要整理）"
    echo
    render_table "$rows_uncategorized"
  fi
  echo "## 使い分け指針"
  echo
  echo "1. CLI 動作確認だけなら \`my_first_policy\` か \`access_control_ok\`。"
  echo "2. CI 連携の JSON 契約確認なら \`customer_contract_ja\` か \`complex_policy_import_entry\`。"
  echo "3. lint の厳密重複判定を試すなら \`semantic_dup_advanced\` と \`semantic_dup_function_param\`。"
} >"$OUT_FILE"

echo "生成完了: $OUT_FILE"
