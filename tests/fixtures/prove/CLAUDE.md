# CLAUDE.md (tests/fixtures/prove)

この階層は `prove --format json` の契約フィクスチャ。

## ファイル対応
- `ok.dtl` -> `ok.stdout.json`, `ok.proof-trace.json`
- `ng.dtl` -> `ng.stdout.json`

## 更新手順
1. 仕様変更後に `cargo test --test integration_prove_json_contract` を実行して差分を確認。
2. 意図した差分のみ fixture JSON を更新。
3. `cargo test --test integration_prove_json_contract` を再実行して固定化。
