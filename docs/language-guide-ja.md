# 言語解説ガイド（v0.2）

この文書は、`docs/language-spec.md` の仕様項目を「なぜその設計か」「実際にどう書くか」「どこで失敗するか」の観点で補足する実践向けガイドです。

## 1. 対象バージョン
- DSL 仕様: v0.2（`docs/language-spec.md` 準拠）
- 実装: `dtl` 本体（Rust, `rust-toolchain.toml` は `1.93.0`）

## 2. この DSL が解く問題
`dtl` は「業務ドメインの制約」を、次の 3 段階で扱います。

1. `check`: 構文・名前解決・型・全域性などの静的検査
2. `prove`: 有限モデル（`universe`）での証明義務検証
3. `doc`: 証明済み仕様のみを文書化

重要なのは、**汎用プログラミング言語ではなく、検証 DSL** である点です。副作用と再帰を抑え、意味を固定しやすくしています。

## 3. まず覚える 6 つの概念

### 3.1 `sort`: 開集合の型名
`sort` は「ドメイン軸の型名」です。値の一覧までは固定しません。

```lisp
(sort 主体)
(sort 契約)
```

### 3.2 `data`: 閉集合の語彙（ADT）
`data` は constructor 群で値語彙を固定します。

```lisp
(data 顧客種別 (法人) (個人))
```

この場合、`顧客種別` は `法人`/`個人` 以外を受け付けません。語彙統制の主役です。

### 3.3 `relation` + `fact` + `rule`: 論理知識
宣言 (`relation`)・事実 (`fact`)・規則 (`rule`) を定義します。

```lisp
(relation 契約締結可能 (主体 契約 顧客種別))
(fact 契約締結可能 山田 基本契約 (法人))
(rule (契約締結可能 ?担当 ?契約ID ?種別)
      (and (担当顧客種別 ?担当 ?種別)
           (契約登録 ?契約ID)))
```

`rule` 変数は `?x` 形式です。ヘッドの変数は正リテラル側で束縛されている必要があります（安全性制約）。

### 3.4 `defn`: 型付き関数（再帰禁止）
関数は pure で再帰禁止です。戻り値に `Refine` を使うと、契約として証明対象になります。

```lisp
(defn 契約可否 ((担当 主体) (契約ID 契約) (種別 顧客種別))
  (Refine b Bool (契約締結可能 担当 契約ID 種別))
  (契約締結可能 担当 契約ID 種別))
```

### 3.5 `assert`: グローバル制約
`assert` は「常に成り立つべき条件」を定義します。`prove` で義務化されます。

### 3.6 `universe`: 有限モデル境界
`prove` は全探索なので、対象型の有限値集合を `universe` で与えます。

```lisp
(universe 主体 (山田 佐藤))
(universe 契約 (基本契約 特約))
(universe 顧客種別 ((法人) (個人)))
```

## 4. 書き方の実務ルール

### 4.1 キーワードは英語固定
`sort`/`data`/`relation`/`defn`/`match` などは英語です。日本語キーワードは v0.2 非対応です。

### 4.2 識別子は日本語可（Unicode）
識別子は日本語可です。例えば `契約可否` や `顧客種別` をそのまま使えます。

### 4.3 NFC 正規化
識別子 Atom は NFC 正規化されます。見た目同一の合成差異（例: `ガ` と `ガ`）は同一視されます。

ただし quoted Atom（`"..."`）は NFC 正規化されません。`import` の `"path"` は quoted Atom として扱われます（ファイルパス互換のため）。

### 4.4 quoted Atom の境界
- quoted Atom は**文字列リテラルではありません**。`\\n` / `\\t` / `\\\"` などのエスケープは解釈されません。
- バックスラッシュはそのまま保持されます。
- v0.2 の lexer は空白・`(`・`)`・`;` でトークン分割するため、quoted Atom 内の空白や `;` を含む記述は扱えません。

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

同義語 alias で吸収する設計は、v0.2 では採用していません。

## 6. `match` の重要挙動
- `Bool` と `Adt` については網羅性チェックされます。
- 到達不能分岐も検出されます。
- パターン constructor の型・引数個数も検証されます。

例（非網羅）:

```lisp
(defn bad ((u 顧客種別)) Bool
  (match u
    ((法人) true)))
```

`(個人)` が不足し `E-MATCH` になります。

## 7. `check` / `prove` / `doc` の違い

### 7.1 `check`
失敗の主因は次です。
- `E-PARSE`: 形が不正
- `E-RESOLVE`: 名前未定義、重複、unsafe rule
- `E-TYPE`: 型不一致
- `E-TOTAL`: 再帰
- `E-MATCH`: 非網羅/到達不能

### 7.2 `prove`
`assert` と `Refine` 契約を有限モデルで評価します。`universe` 不足や反例で失敗します。

### 7.3 `doc`
未証明義務が 1 つでもあれば失敗します。成果物は「証明成功時のみ」生成されます。

- `--format markdown`（既定）: `spec.md` / `proof-trace.json` / `doc-index.json`
- `--format json`: `spec.json` / `proof-trace.json` / `doc-index.json`

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

### 9.2 `proof-trace.json`
- `schema_version`: トレース契約バージョン
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

## 11. よくある設計ミスと対策

### ミス 1: 値語彙を `sort` で固定しようとする
- 症状: 期待より緩い（未知値が入りうる）
- 対策: `data` へ移行し constructor で閉じる

### ミス 2: 外部用語差分を alias で吸収しようとする
- 症状: 概念差分と表記差分が混ざる
- 対策: 型を分離し、`defn` で明示変換する

### ミス 3: rule ヘッド変数が未束縛
- 症状: `unsafe rule` の `E-RESOLVE`
- 対策: 正リテラルに束縛述語を追加する

## 12. 参考ドキュメント
- 形式仕様: `docs/language-spec.md`
- エラーコード別対処: `docs/troubleshooting-errors-ja.md`
- アーキテクチャ: `docs/architecture-v0.2.md`
- 移行: `docs/migration-v0.2.md`
- テスト観点: `docs/test-matrix.md`
- v0.3 停止性解析設計: `docs/termination-analysis-v0.3.md`
- v0.3 ADT parametric 化評価: `docs/adt-parametric-evaluation-v0.3.md`
- ADR 0001 import 名前空間: `docs/adr/0001-import-namespace.md`
