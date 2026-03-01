# 検証計画（v0.6）

## テスト戦略
- unit
  - parser: `core/surface`、`data/assert/universe/match/constructor項`
  - parser: selfdoc Surface（`project/module/reference/contract/quality-gate`）デシュガ
  - parser: quoted Atom escape（`\\`/`\"`/`\n`/`\t`/`\r`）境界
  - resolve: constructor 解決、再帰 ADT 許容、universe 整合
  - typecheck: 構造再帰判定（tail + strict subterm）/ SCC 内エッジ検査（相互再帰を条件付き許可） (`E-TOTAL`)、`match` 網羅/到達不能 (`E-MATCH`)
  - lint: `L-DUP-EXACT` / `L-DUP-MAYBE` / `L-DUP-SKIP-UNIVERSE` / `L-DUP-SKIP-EVAL-DEPTH` / `L-UNUSED-DECL`
  - fmt: in-place / `--check` / `--stdout` 契約
  - fmt: selfdoc form を保持した整形契約
  - logic_engine: ADT 構造値の導出と一致判定
  - prover: 有限モデル全探索、反例最小化、証跡生成
- differential
  - reference semantics に対する `logic_engine` 導出 fact 集合の一致検証
  - reference semantics に対する `prove` obligation (`id/kind/result/valuation/missing_goals`) の一致検証
  - curated fixture による unsupported in phase1 の明示検証
- metamorphic
  - fact / rule / universe 順序不変
  - alias 正規化不変
  - alpha-renaming 不変
  - `fmt` 前後で `prove` 結果不変
  - import 分割版と単一ファイル版で `prove` 結果不変
- integration
  - `check/prove/doc` の終了コード・JSON 契約・出力ファイル契約
  - `lint/fmt/doc --pdf` の終了コード・JSON 契約・出力ファイル契約
  - `lint --semantic-dup` の universe 不足時スキップ契約（`L-DUP-SKIP-UNIVERSE`）
  - `lint --semantic-dup` の深い再帰スキップ可視化契約（`L-DUP-SKIP-EVAL-DEPTH`）
  - `lint --semantic-dup` の同値候補検出契約（`assert/rule/defn` が `L-DUP-MAYBE`）
  - `lint --semantic-dup` の function 型パラメータ `defn` 同値検出契約
  - `lint --semantic-dup` の非同値除外契約（`L-DUP-MAYBE` 非出力）
  - `lint --semantic-dup` の `confidence` 動的算出契約（モデル探索量が増えるとスコアが上がる）
  - `syntax:auto` 混在衝突の専用診断契約（`E-SYNTAX-AUTO`）
  - `prove` の JSON 契約ゴールデン固定（stdout/`proof-trace.json` 一致）
  - `doc --format markdown|json` の成果物切替契約
  - `selfdoc` の終了コード・設定欠如 (`exit=2`)・成果物契約
  - `selfdoc` の fail-fast 参照検証契約（`E-SELFDOC-REF`）
  - `selfcheck` の coverage 完備 / coverage 不足 / 義務失敗契約
- property
  - 固定点の冪等性・単調性
  - 証明結果の順序不変性
  - supported fragment の generated program に対する production/reference 一致

## trusted boundary
- differential verification の入力は parser / alias normalize / resolve / stratify / typecheck 成功済み AST とする。
- `solve_facts()` / `prove_program()` 自体は信頼境界の外に置き、`tests/support/reference_semantics.rs` の独立実装と照合する。
- `check_program()` が拒否する入力は phase1 では unsupported として別扱いにする。

## 差異 triage
- `reference = spec`, `production != reference`
  - production bug
  - CI fail
- `reference != spec`
  - reference oracle bug
  - CI fail
- `spec` が曖昧
  - `docs/semantics-core-v0.6.md` を先に更新
- unsupported in phase1
  - generator から除外
  - curated fixture で明示的に検証

## 品質ゲート
1. `cargo fmt --all -- --check`
2. `cargo clippy --workspace --all-targets --all-features -- -D warnings`
3. `cargo test --workspace --lib --bins --tests`
4. `cargo test --test differential_logic_engine`
5. `cargo test --test differential_prover`
6. `cargo test --test metamorphic_semantics`
7. `mdbook build docs-site`
8. `cargo test --test integration_cli`
9. `cargo test --test e2e_examples`
10. `cargo test --test integration_lint_fmt_doc_pdf_cli`
11. `cargo test --test integration_prove_doc_cli`
12. `cargo test --test integration_prove_json_contract`
13. `cargo test --test unit_prover`
14. `cargo test --test property_logic`
15. `dtl lint examples/customer_contract_ja.dtl --format json --deny-warnings`
16. `dtl fmt examples/customer_contract_ja.dtl --check`
17. `dtl check examples/complex_policy_import_entry.dtl --format json`
18. `dtl prove examples/complex_policy_import_entry.dtl --format json --out out_complex`
19. `dtl lint examples/semantic_dup_advanced.dtl --format json --semantic-dup`
20. `dtl selfdoc --repo . --out out_selfdoc --format json`
21. `dtl selfcheck --repo . --out out_selfcheck --format json`
22. `cargo bench --bench perf_scaling -- solve_facts/fact_scaling/20 --quick --noplot`
23. `cargo bench --bench perf_scaling -- solve_facts/rule_scaling/10 --quick --noplot`
24. `cargo bench --bench perf_scaling -- prove/minimize_counterexample/4 --quick --noplot`

## 実施順
1. 仕様更新（`semantics-core-v0.6.md` / `language-spec` / `formalization-roadmap`）
2. reference oracle と curated fixture 追加
3. differential / metamorphic テスト追加
4. 実装反映
5. 回帰テスト + 品質ゲート

## 再現性
- Rust stable toolchain
- CI で同一コマンドを実行
  - `quality`: fmt/clippy/all tests/property/coverage
  - `docs-site`: `mdbook build docs-site`
  - `cli-check`: `integration_cli` + `e2e_examples`
  - `cli-lint-fmt`: `integration_lint_fmt_doc_pdf_cli`
  - `cli-prove`: `integration_prove_doc_cli`（prove系）+ `integration_prove_json_contract`
  - `cli-doc`: `integration_prove_doc_cli`（doc系）
  - `cli-selfdoc`: `integration_selfdoc_cli`
  - `cli-selfcheck`: `cargo run -- selfcheck --repo . --out <tmp> --format json`
  - `bench`: `perf_scaling` の代表ケースをスモーク実行
