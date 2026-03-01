# 形式化ロードマップ

## 方針
- phase1 は differential testing を品質ゲートにする。
- Lean / Coq による完全形式化は phase3 へ分離する。
- 形式化対象は production 実装そのものではなく、[semantics-core-v0.6.md](./semantics-core-v0.6.md) の core subset とする。

## phase1
- `logic_engine` / `prove` に対して独立参照意味論オラクルを実装する。
- generated + curated + metamorphic の 3 系統で production と照合する。
- mismatch は CI fail にする。

## phase2
- user-facing 比較実行の experimental な `--engine native|reference` は実装済み。以後は JSON 契約と UX の安定化を行う。
- 参照オラクルの coverage を広げ、unsupported fragment を段階的に減らす。
- recursive `defn Refine` を supported fragment に入れるかは `check_program` の静的意味論拡張後に判断する。

## phase3
- 形式証明リポジトリを本体と分離して管理する。
- 優先順は次の通り。
  1. stratified fixedpoint solver の soundness / completeness
  2. `assert` evaluator の soundness
  3. `defn Refine` evaluator の soundness
- 本体 repo にはロードマップと仕様リンクのみを残し、証明スクリプトは別 repo でもよい。
