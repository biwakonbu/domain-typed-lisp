# 検証計画（v0.2）

## テスト戦略
- unit
  - parser: `data/assert/universe/match/constructor項`
  - resolve: constructor 解決、再帰 ADT 検出、universe 整合
  - typecheck: 再帰禁止 (`E-TOTAL`)、`match` 網羅/到達不能 (`E-MATCH`)
  - logic_engine: ADT 構造値の導出と一致判定
  - prover: 有限モデル全探索、反例最小化、証跡生成
- integration
  - `check/prove/doc` の終了コード・JSON 契約・出力ファイル契約
- property
  - 固定点の冪等性・単調性
  - 証明結果の順序不変性

## 品質ゲート
1. `cargo fmt --all -- --check`
2. `cargo clippy --workspace --all-targets --all-features -- -D warnings`
3. `cargo test --workspace --lib --bins --tests`
4. `cargo test --test integration_cli`
5. `cargo test --test integration_prove_doc_cli`
6. `cargo test --test unit_prover`
7. `cargo test --test property_logic`

## 実施順
1. 仕様更新（language-spec / migration）
2. テスト追加（失敗確認）
3. 実装反映
4. 回帰テスト + 品質ゲート

## 再現性
- Rust stable toolchain
- CI で同一コマンドを実行
