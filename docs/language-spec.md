# 言語仕様（MVP）

## 0. 設計原則
- 本言語は汎用計算を目的としない。
- 計算は、型検査と論理導出（固定点評価）を通じてドメイン制約の正しさを検証するために用いる。

## 1. 形式
- S 式のみを受け付ける。
- 1 ファイルに複数トップレベルフォームを並べる。
- `import` により複数ファイルを連結できる。

## 2. トップレベルフォーム

### 2.1 import
```lisp
(import "relative/path.dtl")
```
- パスは import 元ファイルからの相対解決。
- 循環 import は `E-IMPORT`。

### 2.2 sort
```lisp
(sort Subject)
```

### 2.3 relation
```lisp
(relation has-role (Subject Role))
```

### 2.4 fact
```lisp
(fact has-role alice admin)
```

### 2.5 rule
```lisp
(rule (can-access ?u ?r read)
      (and (has-role ?u admin)
           (resource-public ?r)))
```
- 変数は `?` プレフィックス。
- 否定は `(not (pred ...))`。
- 層化否定のみ許可。

### 2.6 defn
```lisp
(defn can-read ((u Subject) (r Resource))
  (Refine b Bool (can-access u r read))
  (can-access u r read))
```

## 3. 型

```text
Type = Bool | Int | Symbol | Domain(SortId)
     | Fun(Vec<Type>, Type)
     | Refine { var, base, formula }
```

Refinement 型:
```text
{x:T | P(x)}
```
DSL 表現:
```lisp
(Refine x T <formula>)
```

## 4. 論理式

```text
Formula = true
        | (pred term*)
        | (and formula+)
        | (not formula)
```

## 5. 型規則（要点）
- 関数境界（引数/戻り値）は注釈必須。
- `let` 束縛は推論可。
- 関数適用で引数型を検査する。
- 戻り値は宣言型への部分型関係を要求。
- `Refine A <: Refine B` は基底型整合 + 含意 `A => B` で判定。

## 6. 含意判定
- 閉世界仮定（CWA）を採用。
- Datalog 固定点評価（半ナイーブ）で導出事実を計算。
- 含意不能は型エラー。

## 7. エラー分類
- `E-IO`: 入出力エラー
- `E-IMPORT`: import 解決エラー（循環依存など）
- `E-PARSE`: 構文エラー
- `E-RESOLVE`: 名前解決エラー
- `E-STRATIFY`: 層化違反
- `E-TYPE`: 型エラー
- `E-ENTAIL`: 含意失敗

## 8. CLI 出力形式
- 既定は人間可読なテキスト診断。
- `dtl check <FILE>... --format json` は以下を出力する。
  - 成功: `status="ok"` と `report`
  - 失敗: `status="error"` と `diagnostics[]`
- 診断には `source`（ファイルパス）が付与される場合がある。
