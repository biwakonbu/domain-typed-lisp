# 言語仕様（v0.2）

## 0. 設計原則
- 本言語はドメイン整合性検証専用の DSL であり、汎用計算系ではない。
- 言語内計算は純粋（副作用なし）・非破壊であり、外部状態に依存しない。
- 関数は全域性を要求する。v0.2 では再帰（自己再帰・相互再帰）を禁止する。

## 1. 形式
- S 式のみを受け付ける。
- 1 ファイルに複数トップレベルフォームを記述できる。
- `import` で複数ファイルを連結し、単一 `Program` として検査する。
- 識別子（sort/data/relation/defn 名、定数など）は Unicode を許可する。
- Atom は NFC へ正規化して解釈する（`import` の引用符付きパス文字列は正規化しない）。
- キーワードは英語固定（`sort`/`data`/`relation`/`defn` など）。日本語キーワードは v0.2 では未対応。

### 1.1 Atom 正規化境界（引用符・エスケープ）
- `"` で始まり `"` で終わる Atom は quoted Atom とみなし、NFC 正規化しない。
- quoted Atom は「文字列リテラル」ではなく、エスケープ解釈（`\n` / `\t` / `\"` など）を行わない。
- quoted Atom 内のバックスラッシュはそのまま保持される。
- `import` は quoted Atom の先頭/末尾 `"` のみを除去して path として扱う。
- 字句境界は空白・`(`・`)`・`;` で決まるため、quoted Atom 内でも空白や `;` を含むトークンは v0.2 では未対応。

## 2. CLI
- `dtl check <FILE>... [--format text|json]`
  - 構文 / 名前解決 / 層化否定 / 型検査 / 全域性 / `match` 網羅性を検査する。
- `dtl prove <FILE>... [--format text|json] [--out DIR]`
  - 有限モデル上で証明義務を全探索し、証跡を生成する。
- `dtl doc <FILE>... --out DIR [--format markdown|json]`
  - 証明がすべて成功した場合のみドキュメント束を生成する。

### 2.1 diagnostics（`--format json`）
- エラー時は `status = "error"` と `diagnostics` 配列を返す。
- 各 diagnostic の `source` は、実際に診断が発生したファイルパスを指す。
  - 単一ファイル入力: その入力ファイル
  - 複数ファイル入力: 当該定義を含むファイル
  - `import` 利用時: import 先を含む実ファイル

## 3. トップレベルフォーム

### 3.1 import
```lisp
(import "relative/path.dtl")
```

### 3.2 sort
```lisp
(sort Subject)
```

### 3.3 data（単相・非再帰）
```lisp
(data Action
  (read)
  (write))
```

### 3.4 relation
```lisp
(relation can-access (Subject Resource Action))
```

### 3.5 fact
```lisp
(fact can-access alice doc1 (read))
```

### 3.6 rule
```lisp
(rule (can-access ?u ?r (read))
      (and (has-role ?u admin)
           (resource-public ?r)))
```

### 3.7 assert
```lisp
(assert policy-consistency ((u Subject))
  (not (and (allowed u)
            (not (allowed u)))))
```

### 3.8 universe（有限モデル境界）
```lisp
(universe Subject ((alice) (bob)))
```

### 3.9 defn
```lisp
(defn can-read ((u Subject) (r Resource))
  (Refine b Bool (can-access u r (read)))
  (can-access u r (read)))
```

## 4. 式
```text
Expr = Var | Symbol | Int | Bool
     | (name expr*)
     | (let ((x e)...) body)
     | (if cond then else)
     | (match expr (pattern expr)+)
```

`match` パターン:
```text
Pattern = _ | var | true | false | int | (Ctor pattern*)
```

## 5. 型
```text
Type = Bool | Int | Symbol
     | Domain(SortId)
     | Adt(DataId)
     | Fun(Vec<Type>, Type)
     | Refine { var, base, formula }
```

## 6. 論理式
```text
Formula = true
        | (pred term*)
        | (and formula+)
        | (not formula)

term = var | symbol | int | bool | (Ctor term*)
```

## 7. 検証意味論
- `check`
  - 関数再帰を禁止（`E-TOTAL`）。
  - `match` は網羅必須・到達不能分岐検出（`E-MATCH`）。
  - `Symbol` と `Domain` の暗黙互換は行わない。
  - 意味固定ポリシー:
    - `data` constructor を業務語彙の閉集合として利用する（正規名強制）。
    - `sort` は開集合として扱う。
    - 概念変更（v1/v2 差分や外部連携差分）は型を分離し、`defn` で明示変換する。
    - 同義語 alias 機能は v0.2 では提供しない。
- `prove`
  - 証明義務:
    - `defn` の戻り値 Refinement 含意
    - `assert` 義務
  - `universe` で宣言された有限集合に対して全代入を列挙し、固定点評価で成立判定する。
  - 失敗時は最小前提セット（包含最小）を反例として出力する。

## 8. 生成物
- `prove --out DIR`:
  - `proof-trace.json`（`schema_version = "1.0.0"`）
- `doc --out DIR --format markdown`:
  - `spec.md`
  - `proof-trace.json`
  - `doc-index.json`
- `doc --out DIR --format json`:
  - `spec.json`
  - `proof-trace.json`
  - `doc-index.json`
- 未証明義務が 1 つでもある場合、`doc` は失敗する。

## 9. エラー分類
- `E-IO`: 入出力エラー
- `E-IMPORT`: import 解決エラー
- `E-PARSE`: 構文エラー
- `E-RESOLVE`: 名前解決エラー
- `E-STRATIFY`: 層化違反
- `E-TYPE`: 型エラー
- `E-ENTAIL`: 含意失敗
- `E-TOTAL`: 全域性違反（再帰禁止違反）
- `E-DATA`: `data` 宣言違反（重複・再帰・constructor 不整合）
- `E-MATCH`: `match` 検査違反（非網羅・到達不能・型不整合）
- `E-PROVE`: 証明失敗 / universe 不備 / 反例検出
