# CLAUDE.md (tests)

この階層は Rust テスト群。

## テスト分類
- `unit_*`: モジュール単位
- `integration_*`: CLI/機能統合
- `e2e_examples.rs`: examples ベースの契約検証
- `property_logic.rs`: 性質テスト（proptest）
- `fixtures/`: JSON 契約・固定入出力

## 編集ルール
- CLI 出力契約を変えた場合、fixture と assertion を同時更新する。
- 失敗期待ケースは診断 `code` と主要 `message` を両方検証する。
- JSON 契約テストでは stderr 汚染を許容しない方針を維持する。

## 実行
- `cargo test --workspace --lib --bins --tests`
- `cargo test --test integration_cli`
- `cargo test --test integration_prove_json_contract`
- `cargo test --test e2e_examples`
