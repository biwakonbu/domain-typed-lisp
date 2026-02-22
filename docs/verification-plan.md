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
  - `prove` の JSON 契約ゴールデン固定（stdout/`proof-trace.json` 一致）
  - `doc --format markdown|json` の成果物切替契約
- property
  - 固定点の冪等性・単調性
  - 証明結果の順序不変性

## 品質ゲート
1. `cargo fmt --all -- --check`
2. `cargo clippy --workspace --all-targets --all-features -- -D warnings`
3. `cargo test --workspace --lib --bins --tests`
4. `cargo test --test integration_cli`
5. `cargo test --test e2e_examples`
6. `cargo test --test integration_prove_doc_cli`
7. `cargo test --test integration_prove_json_contract`
8. `cargo test --test unit_prover`
9. `cargo test --test property_logic`
10. `cargo bench --bench perf_scaling -- solve_facts/fact_scaling/20 --quick --noplot`
11. `cargo bench --bench perf_scaling -- solve_facts/rule_scaling/10 --quick --noplot`
12. `cargo bench --bench perf_scaling -- prove/minimize_counterexample/4 --quick --noplot`

## 実施順
1. 仕様更新（language-spec / migration）
2. テスト追加（失敗確認）
3. 実装反映
4. 回帰テスト + 品質ゲート

## 再現性
- Rust stable toolchain
- CI で同一コマンドを実行
  - `quality`: fmt/clippy/all tests/property/coverage
  - `cli-check`: `integration_cli` + `e2e_examples`
  - `cli-prove`: `integration_prove_doc_cli`（prove系）+ `integration_prove_json_contract`
  - `cli-doc`: `integration_prove_doc_cli`（doc系）
  - `bench`: `perf_scaling` の代表ケースをスモーク実行
