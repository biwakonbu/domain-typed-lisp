# v0.1.x -> v0.2 移行ガイド（v0.4 追補）

## 1. 破壊的変更
- CLI が `check` / `prove` / `doc` に分離された。
- `lint` / `fmt` サブコマンドが追加された。
- 関数再帰の規則が変更された。
  - 相互再帰は `E-TOTAL` で禁止。
  - 自己再帰は「tail position + ADT 引数の構造減少」の場合のみ許可。
- `data` の再帰定義（例: `(data List (nil) (cons Symbol List))`）を許可。
- `Symbol` と `Domain` の暗黙互換を廃止。
- `match` は網羅必須、到達不能分岐は `E-MATCH`。
- 証明実行には `universe` 宣言が必要（不足時 `E-PROVE`）。
- 構文モードが `core/surface` の二層になった（先頭 `; syntax: core|surface|auto`）。
- `syntax:auto` 判定で Core/Surface が同一ファイル混在すると `E-SYNTAX-AUTO` になる。

## 2. 置換指針

### 2.1 旧 `check` だけ利用していた場合
- 既存:
```bash
dtl check path/to/file.dtl
```
- 変更なし（そのまま利用可能）

### 2.2 証明を実行したい場合
```bash
dtl prove path/to/file.dtl --format json --out out
```

### 2.3 ドキュメントを生成したい場合
`doc` の既定フォーマットは `markdown` です。

```bash
dtl doc path/to/file.dtl --out out --format markdown
```

JSON 仕様成果物が必要な場合:

```bash
dtl doc path/to/file.dtl --out out_json --format json
```

PDF まで生成したい場合（Pandoc 利用可能環境）:

```bash
dtl doc path/to/file.dtl --out out --format markdown --pdf
```

Pandoc が無い場合も Markdown 成果物は生成され、PDF は warning になります。

### 2.4 lint を導入する場合
```bash
dtl lint path/to/file.dtl --format json
dtl lint path/to/file.dtl --format json --deny-warnings
dtl lint path/to/file.dtl --format json --semantic-dup
```

- `L-DUP-EXACT`: 確定重複
- `L-DUP-MAYBE`: 有限モデルでの双方向検証による重複候補（`--semantic-dup`）
  - `confidence` はモデルカバレッジ + 反例探索結果ベースで動的算出（0.00〜0.99）
- `L-DUP-SKIP-UNIVERSE`: semantic duplicate 判定スキップ
- `L-DUP-SKIP-EVAL-DEPTH`: `defn` 比較中に評価深さ上限へ到達
- `L-UNUSED-DECL`: 未使用宣言

### 2.5 fmt を導入する場合
```bash
dtl fmt path/to/file.dtl --check
dtl fmt path/to/file.dtl
dtl fmt path/to/file.dtl --stdout
```

- 既定は in-place 更新
- `--check` は差分検出のみ
- `--stdout` は単一入力時のみ有効

### 2.6 複雑シナリオを確認する場合

```bash
dtl check examples/complex_policy_import_entry.dtl --format json
dtl prove examples/complex_policy_import_entry.dtl --format json --out out_complex
dtl lint examples/semantic_dup_advanced.dtl --format json --semantic-dup
```

- `complex_policy_import_entry.dtl`: import + Surface + 複数 `@context` + 構造再帰 + `prove/doc`
- `semantic_dup_advanced.dtl`: `L-DUP-MAYBE` の厳密判定（`rule/assert/defn`）

## 3. DSL 修正パターン

### 3.1 ドメイン定数を ADT constructor に置換
- 旧:
```lisp
(sort Action)
(relation can-access (Subject Resource Action))
(rule (can-access ?u ?r read) ...)
```
- 新:
```lisp
(data Action (read))
(relation can-access (Subject Resource Action))
(rule (can-access ?u ?r (read)) ...)
```

### 3.2 証明対象に universe を追加
```lisp
(data Subject (alice) (bob))
(universe Subject ((alice) (bob)))
```

### 3.3 グローバル制約を assert へ移行
```lisp
(assert consistency ((u Subject))
  (not (and (allowed u) (not (allowed u)))))
```

### 3.4 関数再帰を構造再帰へ修正
非許可（非減少）:
```lisp
(data Nat (z) (s Nat))
(defn bad ((n Nat)) Bool
  (match n
    ((z) true)
    ((s m) (bad n))))
```

許可（tail + strict subterm 減少）:
```lisp
(data Nat (z) (s Nat))
(defn ok ((n Nat)) Bool
  (match n
    ((z) true)
    ((s m) (ok m))))
```

### 3.5 Surface 形式へ移行（任意）
Core のままでも互換ですが、可読性向上のため Surface へ統一可能です。

```lisp
; syntax: surface
(型 主体)
(データ 顧客種別 :コンストラクタ ((法人) (個人)))
(関係 契約締結可能 :引数 (主体 契約 顧客種別))
```

## 4. よくあるエラー
- `E-TYPE`: constructor 呼び出し漏れ（`read` ではなく `(read)`）。
- `E-TOTAL`: 非構造再帰（非 tail / 非減少）または相互再帰。
- `E-MATCH`: `match` の分岐不足 or 到達不能分岐。
- `E-PROVE`: `universe` 未宣言、または反例あり。

## 5. 移行時チェックリスト
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --lib --bins --tests`
- `dtl lint <FILE> --format json --deny-warnings`
- `dtl fmt <FILE> --check`
- `dtl prove <FILE> --format json --out out`
- `dtl doc <FILE> --out out --format markdown`
- `dtl doc <FILE> --out out --format markdown --pdf`
- `dtl doc <FILE> --out out_json --format json`
