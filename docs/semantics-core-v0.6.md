# コア意味論（v0.6 / phase1）

## 目的
- この文書は `prove` と `logic_engine` の検証対象意味論を固定するための source of truth である。
- phase1 の一次目標は、production 実装がこの意味論と一致することの differential verification であり、全言語の完全形式証明ではない。

## 信頼境界
- 信頼するもの
  - parser による AST 構築
  - `normalize_program_aliases`
  - `resolve_program`
  - `check_program` の成功/失敗境界
  - `compute_strata`
- 独立照合するもの
  - stratified fixedpoint 評価
  - `assert` 義務判定
  - `defn Refine` 義務判定
  - `if` / `let` / `match` / `defn` 呼び出しの式評価

## 値意味論

```text
Value = Symbol(String)
      | Int(i64)
      | Bool(bool)
      | Adt { ctor: String, fields: Vec<Value> }
```

- `sort` は開集合として扱うが、`prove` 時には `universe` が有限境界を与える。
- `data` は constructor 名で閉じた ADT 値を構成する。
- `alias` は検証前に正規名へ正規化される。

## 論理式意味論

`Formula` は環境 `ρ` と導出済み事実集合 `D` に対して真偽値を返す。

```text
⟦true⟧ρ,D = true
⟦(pred term*)⟧ρ,D = instantiated tuple が D[pred] に含まれる
⟦(and f1 ... fn)⟧ρ,D = 全ての fi が true
⟦(not f)⟧ρ,D = not ⟦f⟧ρ,D
```

- `term` は `ρ` により ground value へ具体化される。
- 非 ground な論理式は phase1 の supported fragment では扱わない。

## ルール意味論
- `rule` は stratified Datalog + Closed World Assumption + 最小固定点で解釈する。
- strata は `compute_strata` の結果に従う。
- 各 stratum では次を収束まで反復する。
  - 正リテラルを join して代入集合を構成する。
  - 負リテラルで候補代入を filter する。
  - head を具体化して新しい tuple を追加する。
- `fact` は extensional fact、`rule` 導出は intensional fact である。

## `assert` 意味論

`assert name ((x1 T1) ... (xn Tn)) F` は次を意味する。

```text
∀ ρ ∈ Universe(T1) × ... × Universe(Tn). ⟦F⟧ρ,D = true
```

- 1 つでも偽になる valuation があれば obligation は `failed`。
- 反例は最初に見つかった valuation と不足ゴールで表現する。

## `defn Refine` 意味論

```text
(defn f ((x1 T1) ... (xn Tn))
  (Refine b Bool G)
  body)
```

は次を意味する。

```text
∀ ρ. let v = eval(body, ρ, D) in
  if v = Bool(true) then ⟦G⟧(ρ[b := Bool(true)]),D else true
```

- phase1 では `Bool` を返す `Refine` 義務のみ proof target とする。
- `body` が `Bool(false)` なら含意は vacuous truth とみなす。
- `body` が `Bool` 以外へ評価された場合は `E-PROVE` 扱いとする。
- 反例に含める前提は、`body` 評価中に真だった正の relation call の集合である。

## 式意味論
- `Var` / `Symbol` / `Int` / `Bool` は対応する値へ評価する。
- relation call は `Bool` を返し、真だった場合のみその ground fact を前提集合へ加える。
- constructor call は `Adt` を返す。
- `let` は逐次束縛で環境を拡張する。
- `if` は条件を `Bool` として評価し、選択された branch のみ評価する。
- `match` は最初に一致した arm のみ評価する。
- `defn` call は call-by-value で評価し、phase1 では memo を用いて再帰ループを防ぐ。

## supported fragment
- `check_program()` を通過した入力のみ differential verification の対象とする。
- `tests/support/reference_semantics.rs` は production の `solve_facts()` / `prove_program()` を呼ばずに、この意味論を別実装で評価する。
- `tests/differential_logic_engine.rs` は導出 fact 集合を比較する。
- `tests/differential_prover.rs` は obligation の `id/kind/result/valuation/missing_goals` を比較する。
- `tests/metamorphic_semantics.rs` は順序・alias・alpha-renaming・fmt・import 分割の不変性を確認する。
- `native` engine は既存の `ProofTrace` 契約を維持しつつ、`recursive defn Refine` の一部を `check_program()` の semantic fallback で通す。
- `reference` engine は独立 evaluator を用い、function-typed quantified variable を有限関数モデルとして列挙できる。
- phase3 の形式化引き継ぎ資材は [formalization-bootstrap.md](./formalization-bootstrap.md) と [formalization-theorem-inventory.md](./formalization-theorem-inventory.md) を参照する。

## phase1 で明示的に対象外
- parametric ADT
- parser 自体の独立形式検証
- `check_program()` が universe なしでは still `E-ENTAIL` になる recursive `defn Refine` の完全一般化

## 差異 triage
- reference = spec, production != reference
  - production bug
  - CI fail
  - `P0-semantics`
- reference != spec
  - reference oracle bug
  - CI fail
  - reference を先に修正
- spec が曖昧
  - まずこの文書を更新
  - 実装修正は仕様固定後
- unsupported in phase1
  - differential generator から除外
  - curated fixture では unsupported として明示検証する
