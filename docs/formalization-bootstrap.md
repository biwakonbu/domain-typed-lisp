# 形式化 bootstrap（phase3 用）

## 目的
- この文書は、`dtl` のコア意味論を Lean / Coq の別リポジトリへ切り出す際の bootstrap 契約を固定する。
- proof repo 側は production 実装を写経せず、[semantics-core-v0.6.md](./semantics-core-v0.6.md) の core subset を source of truth として採用する。

## proof repo に持ち込む入力
- 仕様
  - [semantics-core-v0.6.md](./semantics-core-v0.6.md)
  - [language-spec.md](./language-spec.md) の `prove` / `logic_engine` / `typecheck` 節
- theorem inventory
  - [formalization-theorem-inventory.md](./formalization-theorem-inventory.md)
- fixture
  - `tests/fixtures/semantics/if-condition-sensitive/`
  - `tests/fixtures/semantics/match-pattern-sensitive/`
  - `tests/fixtures/semantics/alias-canonicalization/`
  - `tests/fixtures/semantics/recursive-defn/`
  - `tests/fixtures/semantics/negative-stratified/`

## 推奨リポジトリ構成
```text
domain-typed-lisp-formalization/
  README.md
  docs/
    trusted-boundary.md
    theorem-status.md
  fixtures/
    semantics/
  lean/ or coq/
    CoreSyntax
    CoreValues
    FormulaSemantics
    Fixedpoint
    AssertSoundness
    RefineSoundness
  ci/
    proof-smoke.sh
```

## README に固定すべき事項
- 対象仕様バージョン: `dtl` language spec v0.6
- 信頼境界
  - parser / alias normalize / resolve / `check_program` / stratification は外側で成立済みとみなす
  - fixedpoint / `assert` / `defn Refine` evaluator を証明対象に置く
- phase3 の優先順位
  1. stratified fixedpoint solver の soundness / completeness
  2. `assert` evaluator の soundness
  3. `defn Refine` evaluator の soundness

## 初回コミットで入れるべき雛形
- `README.md`
  - 対象仕様、信頼境界、定理一覧、未対応 fragment
- `docs/trusted-boundary.md`
  - この repo で仮定する前提を列挙
- `docs/theorem-status.md`
  - theorem inventory の進捗表
- `fixtures/semantics/`
  - curated fixture から抜粋した最小例
- `lean/` または `coq/`
  - core syntax / semantics / theorem stub
- `ci/proof-smoke.sh`
  - toolchain install と stub compile の最小 smoke

## theorem stub の命名規約
- `fixedpoint_sound`
- `fixedpoint_complete`
- `assert_sound`
- `refine_sound`
- 補題は `eval_*`, `match_*`, `subst_*`, `ground_*`, `strata_*` の接頭辞で揃える。

## fixture 移送の優先順
1. `negative-stratified`
2. `match-pattern-sensitive`
3. `recursive-defn`

## publish 前の完了条件
- proof repo が CI 上で toolchain を自動セットアップできる。
- theorem inventory の P0 セットが stub compile する。
- curated fixture 3 件以上が README から参照される。
- current repo 側のリンク先がすべて解決する。

## この repo 側に残すもの
- 仕様文書
- roadmap
- theorem inventory
- bootstrap 契約

## この repo 側に残さないもの
- Lean / Coq の証明スクリプト本体
- proof assistant 固有の build 設定
- proof repo 固有の CI 詳細
