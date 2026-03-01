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
- recursive `defn Refine` は `check_program` の semantic fallback により supported fragment へ入れた。残件は fallback の一般化ではなく、proof repo 側での定理化準備である。

## phase3
- 形式証明リポジトリを本体と分離して管理する。
- 本体 repo 側の bootstrap 資材は [formalization-bootstrap.md](./formalization-bootstrap.md) と [formalization-theorem-inventory.md](./formalization-theorem-inventory.md) に固定する。
- 優先順は次の通り。
  1. stratified fixedpoint solver の soundness / completeness
  2. `assert` evaluator の soundness
  3. `defn Refine` evaluator の soundness
- 本体 repo にはロードマップ・仕様・bootstrap 資材のみを残し、証明スクリプトは別 repo で管理する。

## phase3 の最小着手条件
- separate repo の README から [semantics-core-v0.6.md](./semantics-core-v0.6.md) と theorem inventory へリンクできること。
- prover の選定（Lean 4 または Coq）を固定し、toolchain を CI で再現できること。
- theorem inventory の P0 セットに対して、定理名・依存補題・未証明 stub をコンパイル可能な形で配置すること。
- current repo の curated semantics fixture から最低 3 件を proof repo 側へ転載し、意味論例として参照できること。
