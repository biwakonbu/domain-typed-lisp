# semantic-dup 実践

`lint --semantic-dup` の運用で必要な観点だけをまとめます。

## 1. 基本ケース（rule/assert/defn）

```bash
cargo run -- lint examples/semantic_dup_advanced.dtl --format json --semantic-dup
```

確認点:
- `L-DUP-MAYBE` が 3件（`rule/assert/defn`）
- `confidence` が 0.00〜0.99 で返る

## 2. function 型パラメータを含む `defn`

```bash
cargo run -- lint examples/semantic_dup_function_param.dtl --format json --semantic-dup
```

確認点:
- `defn passthrough_a` / `passthrough_b` が `L-DUP-MAYBE`

## 3. 深い再帰の比較スキップ

深い再帰では入力点の一部が `L-DUP-SKIP-EVAL-DEPTH` になる場合があります。

確認点:
- warning メッセージの `depth_limit` / `checked` / `skipped` / `depth_limited`
- `skipped` が多い場合は `universe` を縮小して再実行

## 4. 運用ルール

1. まず `L-DUP-SKIP-UNIVERSE` を解消（universe 補完）
2. 次に `L-DUP-SKIP-EVAL-DEPTH` を確認（深さ/モデル調整）
3. 最後に `L-DUP-MAYBE` をレビュー対象として扱う
