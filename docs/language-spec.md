# 言語仕様（v0.6）

## 0. 設計原則
- 本言語はドメイン整合性検証専用の DSL であり、汎用計算系ではない。
- 言語内計算は純粋（副作用なし）・非破壊であり、外部状態に依存しない。
- 関数は全域性を要求する。v0.6 では SCC 単位で再帰を判定し、全再帰エッジで構造減少を満たす場合のみ相互再帰を許可する。
- 構文は Core（英語キーワード）と Surface（タグ付き可読構文）の二層で提供し、同一 AST に収束する。
- 言語仕様バージョンと crate SemVer の対応は `docs/versioning-policy.md` に従う。

## 1. 形式
- S 式のみを受け付ける（Core/Surface ともに括弧構造を維持）。
- 1 ファイルに複数トップレベルフォームを記述できる。
- `import` で複数ファイルを連結し、単一 `Program` として検査する。
- 識別子（sort/data/relation/defn 名、定数など）は Unicode を許可する。
- Atom は NFC へ正規化して解釈する（`import` の引用符付きパス文字列は正規化しない）。
- `; syntax: core|surface|auto` pragma で構文モードを明示できる。省略時は auto 判定。
- auto 判定で Core/Surface の同一ファイル混在を検知した場合は `E-SYNTAX-AUTO` で失敗する。
- Surface は日英キーワードエイリアスを受理する（例: `sort`/`型`）。

### 1.1 Atom 正規化境界（引用符・エスケープ）
- `"` で始まり `"` で終わる Atom は quoted Atom とみなし、NFC 正規化しない。
- quoted Atom は v0.6 で文字列リテラルとして扱い、`\\` / `\"` / `\n` / `\t` / `\r` を解釈する。
- quoted Atom 内の空白・`;`・括弧はトークン境界として分割されない。
- 未対応エスケープは `E-PARSE` で失敗する。
- `import` は quoted Atom の先頭/末尾 `"` を除去した値（エスケープ展開後）を path として扱う。

## 2. CLI
- `dtl check <FILE>... [--format text|json]`
  - 構文 / 名前解決 / 層化否定 / 型検査 / 全域性 / `match` 網羅性を検査する。
- `dtl prove <FILE>... [--format text|json] [--engine native|reference] [--out DIR]`
  - 有限モデル上で証明義務を全探索し、証跡を生成する。
  - `native` は既定エンジン、`reference` は独立参照意味論による experimental エンジン。
- `dtl doc <FILE>... --out DIR [--format markdown|json] [--engine native|reference]`
  - 証明がすべて成功した場合のみドキュメント束を生成する。
  - `--engine reference` を指定すると、`prove` と同じ参照意味論で `proof-trace.json` を生成する。
- `dtl selfdoc [--repo PATH] [--config PATH] --out DIR [--format markdown|json] [--engine native|reference] [--pdf]`
  - `scan -> extract -> render selfdoc DSL -> parse/prove/doc` を実行し、自己記述成果物を生成する。
  - README または language-spec の `<!-- selfdoc:cli-contracts:start -->` 契約テーブルから CLI 契約を抽出する。
  - `--config` 省略時は `<repo>/.dtl-selfdoc.toml` を使用する。
  - 設定ファイル未配置時はテンプレートを stderr に出力し `exit code = 2` で終了する。
- `dtl selfcheck [--repo PATH] [--config PATH] --out DIR [--format text|json] [--doc-format markdown|json] [--engine native|reference] [--pdf]`
  - `selfdoc` と同一フローを実行し、`claim_coverage = 100%` かつ全義務 `proved` の場合のみ成功する。
  - 失敗時も `proof-trace.json` は出力する。
- `dtl lint <FILE>... [--format text|json] [--deny-warnings] [--semantic-dup]`
  - 重複検出（`L-DUP-*`）と未使用宣言（`L-UNUSED-DECL`）を警告として出力する。
- `dtl fmt <FILE>... [--check] [--stdout]`
  - AST 正規化 + Surface 形式レンダリングを行う。既定は in-place 更新。
  - `; @context:` をブロック単位で保持し、複数コンテキストでも安定整形（idempotent）を保証する。
  - selfdoc form（`project/module/reference/contract/quality-gate`）を保持した整形をサポートする。

### 2.1 diagnostics（`--format json`）
- エラー時は `status = "error"` と `diagnostics` 配列を返す。
- 各 diagnostic の `source` は、実際に診断が発生したファイルパスを指す。
  - 単一ファイル入力: その入力ファイル
  - 複数ファイル入力: 当該定義を含むファイル
  - `import` 利用時: import 先を含む実ファイル
- `E-TOTAL` には機械可読フィールドを付与する。
  - `reason`: 停止性違反カテゴリ（`non_tail_recursive_call` / `recursive_call_arity_mismatch` / `no_adt_parameter` / `non_decreasing_argument`）
  - `arg_indices`: `reason = non_decreasing_argument` の場合のみ出力。構造減少を要求した引数位置（1始まり）。
- `lint --format json` は `diagnostics[].severity/lint_code/category/confidence` を返す。

## 3. トップレベルフォーム

### 3.1 import
```dtl
(import "relative/path.dtl")
```

### 3.2 alias
```dtl
(alias 閲覧 read)
```

Surface:

```dtl
(同義語 :別名 閲覧 :正規 read)
```

### 3.3 sort
```dtl
(sort Subject)
```

### 3.4 data（単相・再帰許可）
```dtl
(data Action
  (read)
  (write))
```

### 3.5 relation
```dtl
(relation can-access (Subject Resource Action))
```

### 3.6 fact
```dtl
(fact can-access alice doc1 (read))
```

### 3.7 rule
```dtl
(rule (can-access ?u ?r (read))
      (and (has-role ?u admin)
           (resource-public ?r)))
```

### 3.8 assert
```dtl
(assert policy-consistency ((u Subject))
  (not (and (allowed u)
            (not (allowed u)))))
```

### 3.9 universe（有限モデル境界）
```dtl
(universe Subject ((alice) (bob)))
```

### 3.10 defn
```dtl
(defn can-read ((u Subject) (r Resource))
  (Refine b Bool (can-access u r (read)))
  (can-access u r (read)))
```

### 3.11 Surface（タグ付き）例
```dtl
; syntax: surface
(型 主体)
(データ 顧客種別 :コンストラクタ ((法人) (個人)))
(関係 契約締結可能 :引数 (主体 契約 顧客種別))
(事実 契約締結可能 :項 (山田 基本契約 (法人)))
(規則 :頭 (契約締結可能 ?担当 ?契約ID ?種別)
      :本体 (and (担当顧客種別 ?担当 ?種別)
                 (契約登録 ?契約ID)))
```

### 3.12 selfdoc Surface（タグ付き）例
```dtl
; syntax: surface
(プロジェクト :名前 "domain-typed-lisp" :概要 "自己記述 DSL")
(モジュール :名前 "README.md" :パス "README.md" :カテゴリ doc)
(参照 :元 "README.md" :先 "docs/language-spec.md")
(契約 :名前 "cli::check" :出典 "README.md" :パス "src/main.rs")
(品質ゲート :名前 "ci:quality:1" :コマンド "cargo test" :出典 ".github/workflows/ci.yml" :必須 yes)
```

- 日英エイリアス: `project/プロジェクト`, `module/モジュール`, `reference/参照`, `contract/契約`, `quality-gate/品質ゲート`
- これらは parser フロントで既存 Core `fact` 群へデシュガされる。

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
- コア意味論と trusted boundary の詳細は [semantics-core-v0.6.md](./semantics-core-v0.6.md) を参照する。
- `check`
  - 再帰（自己再帰/相互再帰）は SCC 単位で判定し、SCC 内の各再帰エッジ（`caller -> callee`）が次を満たす場合のみ許可する。
    - 再帰呼び出しが tail position にある。
    - callee の ADT 引数位置の少なくとも 1 つが、caller の ADT パラメータ由来 strict subterm に減少している。
  - `match` は網羅必須・到達不能分岐検出（`E-MATCH`）。
  - `Symbol` と `Domain` の暗黙互換は行わない。
  - 意味固定ポリシー:
    - `data` constructor を業務語彙の閉集合として利用する（正規名強制）。
    - `sort` は開集合として扱う。
    - 概念変更（v1/v2 差分や外部連携差分）は型を分離し、`defn` で明示変換する。
    - constructor 同義語は top-level `alias`（Surface: `同義語`）で定義し、内部では正規名へ正規化する。
- `prove`
  - 証明義務:
    - `defn` の戻り値 Refinement 含意
    - `assert` 義務
  - `universe` で宣言された有限集合に対して全代入を列挙し、固定点評価で成立判定する。
  - `reference` engine は function-typed quantified variable を含む valuation を有限関数モデルとして列挙できる。
  - 失敗時は最小前提セット（包含最小）を反例として出力する。

## 8. 生成物
- `prove --out DIR`:
  - `proof-trace.json`（`schema_version = "2.2.0"`）
  - 必須フィールド: `profile`（`standard|selfdoc`）, `engine`（`native|reference`）, `summary`（`total/proved/failed`）, `claim_coverage`（`total_claims/proved_claims`）
- `doc --out DIR --format markdown`:
  - `spec.md`
  - `proof-trace.json`
  - `doc-index.json`
  - `--pdf` 指定時は `spec.pdf` を追加生成（依存ツール不足時は warning 扱い）
- `doc --out DIR --format json`:
  - `spec.json`
  - `proof-trace.json`
  - `doc-index.json`
- `spec.json` は v0.6 で `profile` / `summary` / `self_description` を必須で持つ。
- `doc-index.json` は `schema_version = "2.0.0"` で、`profile` / `intermediate.dsl` / `pdf` を持つ。
- `selfdoc --out DIR` は上記に加え `selfdoc.generated.dtl` を出力する。
- 未証明義務が 1 つでもある場合、`doc` は失敗する。

## 9. エラー分類
- `E-IO`: 入出力エラー
- `E-IMPORT`: import 解決エラー
- `E-PARSE`: 構文エラー
- `E-SYNTAX-AUTO`: auto 構文判定衝突（Core/Surface 混在）
- `E-RESOLVE`: 名前解決エラー
- `E-STRATIFY`: 層化違反
- `E-TYPE`: 型エラー
- `E-ENTAIL`: 含意失敗
- `E-TOTAL`: 全域性違反（非構造再帰 / 非 tail 再帰 / ADT 減少不成立）
- `E-DATA`: `data` 宣言違反（重複・型名衝突・constructor 不整合）
- `E-MATCH`: `match` 検査違反（非網羅・到達不能・型不整合）
- `E-PROVE`: 証明失敗 / universe 不備 / 反例検出
- `E-FMT-SELFDOC-UNSUPPORTED`: 廃止予定（v0.6 以降は selfdoc form を保持整形）
- `E-SELFDOC-CONFIG`: selfdoc 設定不正
- `E-SELFDOC-SCAN`: selfdoc 走査対象不正
- `E-SELFDOC-CLASSIFY`: selfdoc 分類不正
- `E-SELFDOC-REF`: selfdoc 参照抽出/参照先不整合
- `E-SELFDOC-CONTRACT`: CLI 契約抽出不整合
- `E-SELFDOC-GATE`: quality gate 抽出不整合
- `E-SELFCHECK`: selfcheck の coverage 不足

## 10. lint コード
- `L-DUP-EXACT`: 構文正規化後に確定重複
- `L-DUP-MAYBE`: 有限モデルでの双方向検証（`rule/assert` 含意・`defn` 戻り一致）による重複候補
- `L-DUP-SKIP-UNIVERSE`: semantic duplicate 判定を universe 不足でスキップ
- `L-DUP-SKIP-EVAL-DEPTH`: `defn` 比較中に評価深さ上限へ到達したため、入力点の一部を評価できずスキップ
- `L-UNUSED-DECL`: 未使用宣言

`L-DUP-MAYBE`/`L-DUP-SKIP-*` の判定前提:
- `--semantic-dup` 指定時のみ実行する。
- 必須 `universe` は relation 引数型 + `assert/defn` 量化変数型を合成して決定する。
- function 型パラメータを持つ `defn` は、`universe` 上の有限関数モデルを列挙して比較する。
- `confidence` はモデルカバレッジ（`checked_points / model_points`）と反例探索結果（counterexample 有無）から算出する。
- 出力範囲は `0.00`〜`0.99`（小数第2位丸め）。
