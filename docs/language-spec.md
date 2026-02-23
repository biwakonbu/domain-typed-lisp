# 言語仕様（v0.4）

## 0. 設計原則
- 本言語はドメイン整合性検証専用の DSL であり、汎用計算系ではない。
- 言語内計算は純粋（副作用なし）・非破壊であり、外部状態に依存しない。
- 関数は全域性を要求する。v0.4 でも自己再帰は条件付き許可、相互再帰は禁止する。
- 構文は Core（英語キーワード）と Surface（タグ付き可読構文）の二層で提供し、同一 AST に収束する。

## 1. 形式
- S 式のみを受け付ける（Core/Surface ともに括弧構造を維持）。
- 1 ファイルに複数トップレベルフォームを記述できる。
- `import` で複数ファイルを連結し、単一 `Program` として検査する。
- 識別子（sort/data/relation/defn 名、定数など）は Unicode を許可する。
- Atom は NFC へ正規化して解釈する（`import` の引用符付きパス文字列は正規化しない）。
- `; syntax: core|surface` pragma で構文モードを明示できる。省略時は auto 判定。
- Surface は日英キーワードエイリアスを受理する（例: `sort`/`型`）。

### 1.1 Atom 正規化境界（引用符・エスケープ）
- `"` で始まり `"` で終わる Atom は quoted Atom とみなし、NFC 正規化しない。
- quoted Atom は「文字列リテラル」ではなく、エスケープ解釈（`\n` / `\t` / `\"` など）を行わない。
- quoted Atom 内のバックスラッシュはそのまま保持される。
- `import` は quoted Atom の先頭/末尾 `"` のみを除去して path として扱う。
- 字句境界は空白・`(`・`)`・`;` で決まるため、quoted Atom 内でも空白や `;` を含むトークンは v0.4 でも未対応。

## 2. CLI
- `dtl check <FILE>... [--format text|json]`
  - 構文 / 名前解決 / 層化否定 / 型検査 / 全域性 / `match` 網羅性を検査する。
- `dtl prove <FILE>... [--format text|json] [--out DIR]`
  - 有限モデル上で証明義務を全探索し、証跡を生成する。
- `dtl doc <FILE>... --out DIR [--format markdown|json]`
  - 証明がすべて成功した場合のみドキュメント束を生成する。
- `dtl lint <FILE>... [--format text|json] [--deny-warnings] [--semantic-dup]`
  - 重複検出（`L-DUP-*`）と未使用宣言（`L-UNUSED-DECL`）を警告として出力する。
- `dtl fmt <FILE>... [--check] [--stdout]`
  - AST 正規化 + Surface 形式レンダリングを行う。既定は in-place 更新。

### 2.1 diagnostics（`--format json`）
- エラー時は `status = "error"` と `diagnostics` 配列を返す。
- 各 diagnostic の `source` は、実際に診断が発生したファイルパスを指す。
  - 単一ファイル入力: その入力ファイル
  - 複数ファイル入力: 当該定義を含むファイル
  - `import` 利用時: import 先を含む実ファイル
- `E-TOTAL` には機械可読フィールドを付与する。
  - `reason`: 停止性違反カテゴリ（`mutual_recursion` / `non_tail_recursive_call` / `recursive_call_arity_mismatch` / `no_adt_parameter` / `non_decreasing_argument`）
  - `arg_indices`: `reason = non_decreasing_argument` の場合のみ出力。構造減少を要求した引数位置（1始まり）。
- `lint --format json` は `diagnostics[].severity/lint_code/category/confidence` を返す。

## 3. トップレベルフォーム

### 3.1 import
```lisp
(import "relative/path.dtl")
```

### 3.2 sort
```lisp
(sort Subject)
```

### 3.3 data（単相・再帰許可）
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

### 3.10 Surface（タグ付き）例
```lisp
; syntax: surface
(型 主体)
(データ 顧客種別 :コンストラクタ ((法人) (個人)))
(関係 契約締結可能 :引数 (主体 契約 顧客種別))
(事実 契約締結可能 :項 (山田 基本契約 (法人)))
(規則 :頭 (契約締結可能 ?担当 ?契約ID ?種別)
      :本体 (and (担当顧客種別 ?担当 ?種別)
                 (契約登録 ?契約ID)))
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
  - 自己再帰は次の条件を満たす場合のみ許可し、それ以外は `E-TOTAL`。
    - 再帰呼び出しが tail position にある。
    - 少なくとも 1 つの ADT 引数が strict subterm（`match` 分解で得た部分値）に減少している。
  - 相互再帰（SCC サイズ > 1）は `E-TOTAL`。
  - `match` は網羅必須・到達不能分岐検出（`E-MATCH`）。
  - `Symbol` と `Domain` の暗黙互換は行わない。
  - 意味固定ポリシー:
    - `data` constructor を業務語彙の閉集合として利用する（正規名強制）。
    - `sort` は開集合として扱う。
    - 概念変更（v1/v2 差分や外部連携差分）は型を分離し、`defn` で明示変換する。
    - 同義語 alias 機能は v0.4 でも提供しない。
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
  - `--pdf` 指定時は `spec.pdf` を追加生成（依存ツール不足時は warning 扱い）
- `doc --out DIR --format json`:
  - `spec.json`
  - `proof-trace.json`
  - `doc-index.json`
- `doc-index.json` は `pdf` セクション（`requested/generated/message`）を持つ。
- 未証明義務が 1 つでもある場合、`doc` は失敗する。

## 9. エラー分類
- `E-IO`: 入出力エラー
- `E-IMPORT`: import 解決エラー
- `E-PARSE`: 構文エラー
- `E-RESOLVE`: 名前解決エラー
- `E-STRATIFY`: 層化違反
- `E-TYPE`: 型エラー
- `E-ENTAIL`: 含意失敗
- `E-TOTAL`: 全域性違反（非構造再帰 / 非 tail 再帰 / 相互再帰）
- `E-DATA`: `data` 宣言違反（重複・型名衝突・constructor 不整合）
- `E-MATCH`: `match` 検査違反（非網羅・到達不能・型不整合）
- `E-PROVE`: 証明失敗 / universe 不備 / 反例検出

## 10. lint コード
- `L-DUP-EXACT`: 構文正規化後に確定重複
- `L-DUP-MAYBE`: 近似同値判定による重複候補
- `L-DUP-SKIP-UNIVERSE`: semantic duplicate 判定を universe 不足でスキップ
- `L-UNUSED-DECL`: 未使用宣言
