# 形式化 theorem inventory（v0.6 core subset）

## 目的
- phase3 の proof repo で最初に定理化する対象を固定する。
- 優先順位、依存関係、完了条件を曖昧にしない。

## P0: fixedpoint

### `fixedpoint_step_monotone`
- 目的
  - 各 stratum の 1 step 演算子が単調であることを示す。
- 入力
  - ground fact 集合
  - stratified rule 集合
- 依存
  - `ground_substitution_closed`
  - `negative_literal_filter_sound`

### `fixedpoint_least_model_exists`
- 目的
  - 単調 step 演算子に対して最小固定点が存在することを示す。
- 依存
  - `fixedpoint_step_monotone`

### `fixedpoint_sound`
- 目的
  - solver が返した導出 fact は仕様意味論上の帰結であることを示す。
- 依存
  - `fixedpoint_least_model_exists`
  - `rule_instantiation_sound`

### `fixedpoint_complete`
- 目的
  - 仕様意味論で導出される ground fact を solver が取りこぼさないことを示す。
- 依存
  - `fixedpoint_least_model_exists`
  - `rule_instantiation_complete`

## P1: formula / expr

### `formula_eval_deterministic`
- 目的
  - `Formula` の評価が環境と導出 fact 集合に対して決定的であることを示す。

### `pattern_match_deterministic`
- 目的
  - `match` が最初に一致した arm のみを選ぶことを形式化し、選択結果が一意であることを示す。

### `expr_eval_deterministic`
- 目的
  - `if` / `let` / `match` / constructor / relation call / `defn` call を含む式評価が決定的であることを示す。
- 依存
  - `formula_eval_deterministic`
  - `pattern_match_deterministic`

## P2: obligation evaluator

### `assert_sound`
- 目的
  - `assert` obligation が `proved` なら、有限 universe 全域で仕様式が真であることを示す。
- 依存
  - `fixedpoint_sound`
  - `formula_eval_deterministic`

### `refine_sound`
- 目的
  - `defn Refine` obligation が `proved` なら、`body` が `Bool(true)` を返すすべての valuation で refine formula が真であることを示す。
- 依存
  - `fixedpoint_sound`
  - `expr_eval_deterministic`

## 補助補題
- `ground_substitution_closed`
  - ground substitution 後も term が ground のままである。
- `negative_literal_filter_sound`
  - negation filter が Closed World Assumption と整合する。
- `rule_instantiation_sound`
  - rule の 1 回の具体化が意味論的に正しい。
- `rule_instantiation_complete`
  - 意味論的に正しい 1 step 導出を rule 具体化が取りこぼさない。
- `constructor_normalization_preserves_value`
  - alias 正規化後も constructor 値の意味が保存される。
- `valuation_enumeration_complete`
  - `assert` / `Refine` で列挙する有限 valuation が universe 全域を被覆する。

## proof repo 初回マイルストーン
1. `fixedpoint_step_monotone`
2. `fixedpoint_least_model_exists`
3. `fixedpoint_sound`
4. `assert_sound`

## 非目標
- parser の完全形式化
- parametric ADT
- production 実装の Rust コードと 1 対 1 の同型化
