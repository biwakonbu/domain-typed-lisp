# 言語解説ガイド（v0.5）

この文書は、`docs/language-spec.md` の仕様項目を「なぜその設計か」「実際にどう書くか」「どこで失敗するか」の観点で補足する実践向けガイドです。

## 1. 対象バージョン
- DSL 仕様: v0.5（`docs/language-spec.md` 準拠）
- 実装: `dtl` 本体（Rust, `rust-toolchain.toml` は `1.93.0`）

## 2. この DSL が解く問題
`dtl` は「業務ドメインの制約」を、次の 3 段階で扱います。

1. `check`: 構文・名前解決・型・全域性などの静的検査
2. `prove`: 有限モデル（`universe`）での証明義務検証
3. `doc`: 証明済み仕様のみを文書化

重要なのは、**汎用プログラミング言語ではなく、検証 DSL** である点です。副作用を排除し、再帰は停止性条件付きで扱います。

## 3. まず覚える 6 つの概念

### 3.1 `sort`: 開集合の型名
`sort` は「ドメイン軸の型名」です。値の一覧までは固定しません。

```dtl
(sort 主体)
(sort 契約)
```

### 3.2 `data`: 閉集合の語彙（ADT）
`data` は constructor 群で値語彙を固定します。

```dtl
(data 顧客種別 (法人) (個人))
```

この場合、`顧客種別` は `法人`/`個人` 以外を受け付けません。語彙統制の主役です。

### 3.3 `relation` + `fact` + `rule`: 論理知識
宣言 (`relation`)・事実 (`fact`)・規則 (`rule`) を定義します。

```dtl
(relation 契約締結可能 (主体 契約 顧客種別))
(fact 契約締結可能 山田 基本契約 (法人))
(rule (契約締結可能 ?担当 ?契約ID ?種別)
      (and (担当顧客種別 ?担当 ?種別)
           (契約登録 ?契約ID)))
```

`rule` 変数は `?x` 形式です。ヘッドの変数は正リテラル側で束縛されている必要があります（安全性制約）。

### 3.4 `defn`: 型付き関数（構造再帰）
関数は pure です。自己再帰は次を満たす場合のみ許可されます。
- tail position にあること
- ADT 引数の少なくとも 1 つが strict subterm（`match` 分解で得た部分値）へ減少すること

相互再帰は許可されません。戻り値に `Refine` を使うと、契約として証明対象になります。

```dtl
(defn 契約可否 ((担当 主体) (契約ID 契約) (種別 顧客種別))
  (Refine b Bool (契約締結可能 担当 契約ID 種別))
  (契約締結可能 担当 契約ID 種別))
```

### 3.5 `assert`: グローバル制約
`assert` は「常に成り立つべき条件」を定義します。`prove` で義務化されます。

### 3.6 `universe`: 有限モデル境界
`prove` は全探索なので、対象型の有限値集合を `universe` で与えます。

```dtl
(universe 主体 (山田 佐藤))
(universe 契約 (基本契約 特約))
(universe 顧客種別 ((法人) (個人)))
```

## 4. 書き方の実務ルール

### 4.1 Core / Surface 二層構文
- Core: 既存の英語キーワード S 式（後方互換）。
- Surface: タグ付き可読構文（日本語/英語エイリアス）。
- 先頭コメント `; syntax: core|surface|auto` で明示できます（省略時 auto 判定）。
- auto 判定で Core/Surface を同一ファイルに混在させると `E-SYNTAX-AUTO` になります。

Surface 例:
```dtl
; syntax: surface
(型 主体)
(データ 顧客種別 :コンストラクタ ((法人) (個人)))
(関係 契約締結可能 :引数 (主体 契約 顧客種別))
```

### 4.2 識別子は日本語可（Unicode）
識別子は日本語可です。例えば `契約可否` や `顧客種別` をそのまま使えます。

### 4.3 NFC 正規化
識別子 Atom は NFC 正規化されます。見た目同一の合成差異（例: `ガ` と `ガ`）は同一視されます。

ただし quoted Atom（`"..."`）は NFC 正規化されません。`import` の `"path"` は quoted Atom として扱われます（ファイルパス互換のため）。

### 4.4 quoted Atom の境界
- quoted Atom は v0.5 で文字列リテラルとして扱われます。
- 対応エスケープ: `\\\\` / `\\\"` / `\\n` / `\\t` / `\\r`
- 未対応エスケープは `E-PARSE` で失敗します。
- quoted Atom 内の空白・`;`・括弧は 1 トークンとして保持されます。

### 4.5 `Symbol` と `Domain` / `Adt` は別物
`Symbol` を `Domain`/`Adt` 引数に暗黙で渡せません。明示的に型を合わせます。

## 5. 型システムの読み方

`Type` は次で構成されます。

```text
Bool | Int | Symbol | Domain | Adt | Fun | Refine
```

- `Domain(SortId)`: `sort` 由来
- `Adt(DataId)`: `data` 由来
- `Refine`: 論理式で制約した型

### 5.1 意味固定の基本方針
- 値語彙を固定したい: `data` を使う
- 軸だけ定義したい: `sort` を使う
- 概念を変換したい: 型を分けて `defn` で変換

同義語 alias で吸収する設計は、v0.5 でも採用していません。

## 6. `match` の重要挙動
- `Bool` と `Adt` については網羅性チェックされます。
- 到達不能分岐も検出されます。
- パターン constructor の型・引数個数も検証されます。

例（非網羅）:

```dtl
(defn bad ((u 顧客種別)) Bool
  (match u
    ((法人) true)))
```

`(個人)` が不足し `E-MATCH` になります。

## 7. `check` / `prove` / `doc` の違い

### 7.1 `check`
失敗の主因は次です。
- `E-PARSE`: 形が不正
- `E-SYNTAX-AUTO`: `syntax: auto` 判定で Core/Surface が混在
- `E-RESOLVE`: 名前未定義、重複、unsafe rule
- `E-TYPE`: 型不一致
- `E-TOTAL`: 非構造再帰（非 tail / 非減少）または相互再帰
- `E-MATCH`: 非網羅/到達不能

### 7.2 `prove`
`assert` と `Refine` 契約を有限モデルで評価します。`universe` 不足や反例で失敗します。

### 7.3 `doc`
未証明義務が 1 つでもあれば失敗します。成果物は「証明成功時のみ」生成されます。

- `--format markdown`（既定）: `spec.md` / `proof-trace.json` / `doc-index.json`
- `--format json`: `spec.json` / `proof-trace.json` / `doc-index.json`
- `--pdf`（markdown 時）: `spec.pdf` 追加生成を試行。失敗時も Markdown 生成は成功扱いです。

### 7.4 `lint`
```bash
cargo run -- lint examples/customer_contract_ja.dtl --format json
```
- `L-DUP-EXACT`: 確定重複
- `L-DUP-MAYBE`: 有限モデルでの双方向検証による重複候補（`--semantic-dup`）
- `L-DUP-SKIP-UNIVERSE`: semantic duplicate 判定を universe 不足でスキップ
- `L-DUP-SKIP-EVAL-DEPTH`: `defn` 比較で評価深さ上限に到達した入力点をスキップ
- `L-UNUSED-DECL`: 未使用宣言
- `--deny-warnings` を付けると warning で exit 1

### 7.5 `fmt`
```bash
cargo run -- fmt examples/customer_contract_ja.dtl --check
cargo run -- fmt examples/customer_contract_ja.dtl
```
- 既定は in-place 更新
- `--check` は差分検出のみ
- `--stdout` は単一ファイル入力時のみ
- selfdoc form（`project/module/reference/contract/quality-gate`）を含む入力は `E-FMT-SELFDOC-UNSUPPORTED` で失敗

### 7.6 `selfdoc`
```bash
cargo run -- selfdoc --repo . --out out_selfdoc --format json
```
- `.dtl-selfdoc.toml` を読み取り、リポジトリを走査して `selfdoc.generated.dtl` を生成します。
- その後、生成 DSL に対して `prove/doc` を実行し、`spec.json` / `proof-trace.json` / `doc-index.json` を出力します。
- 設定ファイルが無い場合はテンプレートを stderr 出力し、`exit code 2` で終了します。

## 8. チュートリアル: `check -> prove -> doc` 一気通貫

### 8.1 前提
- 作業ディレクトリ: リポジトリルート
- 入力ファイル: `examples/customer_contract_ja.dtl`

### 8.2 `check`: 静的検査を先に確定させる

```bash
cargo run -- check examples/customer_contract_ja.dtl --format json
```

成功時の代表出力:

```json
{"status":"ok","report":{"functions_checked":1,"errors":0}}
```

この段階で `status=error` の場合、`prove` や `doc` へ進む意味はありません。先に `diagnostics` を解消します。

複数ファイル入力（`check a.dtl b.dtl`）や `import` 併用時でも、`diagnostics[].source` には実際の失敗ファイルが入ります。`source` を起点に修正対象を特定してください。

### 8.3 `prove`: 証明義務を有限モデルで検証する

```bash
cargo run -- prove examples/customer_contract_ja.dtl --format json --out out_ja
```

この実行で次の 2 つを確認します。
- 標準出力 JSON の `status` が `ok`
- `out_ja/proof-trace.json` が生成される

`proof-trace.json` の最小確認:

```bash
jq '.schema_version, .obligations[] | {id, kind, result}' out_ja/proof-trace.json
```

`result` が `failed` の義務が 1 つでもある場合、`doc` は失敗します。

### 8.4 `doc`: 証明済み仕様だけを成果物化する

Markdown 仕様を出力:

```bash
cargo run -- doc examples/customer_contract_ja.dtl --out out_ja --format markdown

# PDF も必要な場合（Pandoc 環境）
cargo run -- doc examples/customer_contract_ja.dtl --out out_ja --format markdown --pdf
```

JSON 仕様を出力:

```bash
cargo run -- doc examples/customer_contract_ja.dtl --out out_ja_json --format json
```

期待成果物:
- Markdown モード: `spec.md`, `proof-trace.json`, `doc-index.json`
- JSON モード: `spec.json`, `proof-trace.json`, `doc-index.json`

## 9. 成果物の読み方

### 9.1 `doc-index.json`
- バンドルの入口です。
- `files` に含まれるファイルが「その run の正」です。
- `status` は現状 `ok` 固定です。
- v0.5 では `schema_version=2.0.0`, `profile`, `intermediate.dsl` を持ちます。
- `intermediate.dsl` は通常 `null`、`selfdoc` 実行時は `selfdoc.generated.dtl` です。

### 9.2 `proof-trace.json`
- `schema_version`: トレース契約バージョン
- `profile`: `standard` または `selfdoc`
- `summary`: `total/proved/failed` の要約
- `obligations[].id`: `defn::...` または `assert::...`
- `obligations[].result`: `proved` / `failed`
- `counterexample`: 失敗時のみ出現（`valuation`, `premises`, `missing_goals`）

実務では `failed` 義務の `missing_goals` から、欠落ルール/事実/定義ミスを逆引きします。

### 9.3 `spec.md` / `spec.json`
- `sort` / `data` / `relation` / `assert` の公開仕様
- `proof_status`（JSON）または `Proof Status`（Markdown）で義務の最終状態を追跡

## 10. 失敗時のデバッグ手順（推奨）
1. `check --format json` で構文・解決・型を先に潰す
2. `prove --format json --out ...` で `proof-trace.json` を必ず保存する
3. 失敗義務の `counterexample.missing_goals` を修正対象として扱う
4. 修正後に `doc` を再実行して成果物生成まで確認する

エラーコードごとの原因と対処は `docs/troubleshooting-errors-ja.md` を参照してください。

## 11. 複雑シナリオ実践

### 11.1 マルチファイル + Surface + 複数 `@context`

対象:
- `examples/complex_policy_schema.dtl`
- `examples/complex_policy_rules.dtl`
- `examples/complex_policy_import_entry.dtl`

```bash
cargo run -- check examples/complex_policy_import_entry.dtl --format json
cargo run -- prove examples/complex_policy_import_entry.dtl --format json --out out_complex
```

このケースでは、`import` 分割・`@context` ブロック・構造再帰 `defn`・`assert` を同時に確認できます。

### 11.2 `lint --semantic-dup` の厳密判定

対象:
- `examples/semantic_dup_advanced.dtl`

```bash
cargo run -- lint examples/semantic_dup_advanced.dtl --format json --semantic-dup
```

`rule` / `assert` / `defn` それぞれで `L-DUP-MAYBE` が出ることを確認できます。
`confidence` は固定値ではなく、モデルカバレッジと反例探索結果に応じて変動します。

### 11.3 ネスト `match` + `let` alias 構造再帰

対象:
- `examples/recursive_nested_ok.dtl`

```bash
cargo run -- check examples/recursive_nested_ok.dtl --format json
cargo run -- prove examples/recursive_nested_ok.dtl --format json --out out_recursive
```

strict subterm 判定の境界（`let` alias、ネスト `match`）を再現できます。

## 12. よくある設計ミスと対策

### ミス 1: 値語彙を `sort` で固定しようとする
- 症状: 期待より緩い（未知値が入りうる）
- 対策: `data` へ移行し constructor で閉じる

### ミス 2: 外部用語差分を alias で吸収しようとする
- 症状: 概念差分と表記差分が混ざる
- 対策: 型を分離し、`defn` で明示変換する

### ミス 3: rule ヘッド変数が未束縛
- 症状: `unsafe rule` の `E-RESOLVE`
- 対策: 正リテラルに束縛述語を追加する

## 13. 既知の制約（2026-02-23 時点）

- `L-DUP-MAYBE` の `confidence` は近似指標であり、確率的保証値ではない（モデル境界と評価可能性に依存）。
- function 型を含む `defn` 同値評価は有限関数モデル列挙を行うため、`universe` の組み合わせが大きいと探索コストが急増する。
- 深い再帰では `L-DUP-SKIP-EVAL-DEPTH` が出る場合がある。`depth_limit`/`checked`/`skipped` を確認し、必要なら入力モデル（`universe`）を調整する。

## 14. 参考ドキュメント
- 形式仕様: `docs/language-spec.md`
- エラーコード別対処: `docs/troubleshooting-errors-ja.md`
- アーキテクチャ: `docs/architecture-v0.2.md`
- 移行: `docs/migration-v0.2.md`
- テスト観点: `docs/test-matrix.md`
- 複雑シナリオ集: `docs/example-scenarios-ja.md`
- v0.3 停止性解析設計: `docs/termination-analysis-v0.3.md`
- v0.3 ADT parametric 化評価: `docs/adt-parametric-evaluation-v0.3.md`
- ADR 0001 import 名前空間: `docs/adr/0001-import-namespace.md`
