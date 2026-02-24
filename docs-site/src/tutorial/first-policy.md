# 最初のポリシーを作る

## 1. 既存サンプルを使う

このリポジトリには `examples/my_first_policy.dtl` を同梱しています。
内容は最小構成の `sort/relation/fact/defn/assert/universe` です。

## 2. 実行手順

```bash
cargo run -- check examples/my_first_policy.dtl --format json
cargo run -- prove examples/my_first_policy.dtl --format json --out out_first
cargo run -- lint examples/my_first_policy.dtl --format json --semantic-dup
```

## 3. よくある失敗

- `E-RESOLVE`: 型/関係/関数の定義漏れ
- `E-TYPE`: 引数型や戻り型の不一致
- `L-DUP-SKIP-UNIVERSE`: `universe` 不足

詳細は [トラブルシュート（完全版）](../reference/troubleshooting.md) を参照してください。
