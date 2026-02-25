# Lint / エラーコード

## lint コード

- `L-DUP-EXACT`: 確定重複
- `L-DUP-MAYBE`: 有限モデル上の同値候補
- `L-DUP-SKIP-UNIVERSE`: universe 不足で `semantic-dup` をスキップ
- `L-DUP-SKIP-EVAL-DEPTH`: 深い再帰で評価深さ上限に到達
- `L-UNUSED-DECL`: 未使用宣言

## エラーコード（主要）

- `E-PARSE`: 構文エラー
- `E-RESOLVE`: 名前解決エラー
- `E-TYPE`: 型エラー
- `E-TOTAL`: 全域性違反
- `E-MATCH`: `match` 検査違反
- `E-PROVE`: 証明失敗 / universe 不備
- `E-FMT-SELFDOC-UNSUPPORTED`: selfdoc form に対する fmt 非対応
- `E-SELFDOC-*`: selfdoc 設定/走査/分類/参照/契約/quality gate 抽出エラー
- `E-SELFCHECK`: selfcheck の claim coverage 不足

詳細と対処は [トラブルシュート（完全版）](./troubleshooting.md) を参照してください。
