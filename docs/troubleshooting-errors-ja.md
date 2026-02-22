# エラーコード別トラブルシュート（v0.2）

この文書は `dtl` v0.2 の主要エラーコードについて、原因と復旧手順を運用視点で整理したものです。

## 1. 対象バージョン
- DSL 仕様: v0.2（`docs/language-spec.md` 準拠）
- 実装: `dtl` 本体（Rust, `rust-toolchain.toml` は `1.93.0`）

## 2. 使い方
1. まず `check --format json` または `prove --format json` で `diagnostics` を取得する。
2. `code` ごとに本書の該当節を参照し、再現条件を 1 つずつ潰す。
3. 最後に `doc` まで通して成果物生成を確認する。

---

## 3. `E-PARSE`

### 3.1 典型症状
- S 式の括弧不整合
- フォーム構造の崩れ（引数位置の誤り）

### 3.2 主な原因
- `(` / `)` の不足または過剰
- `defn` / `rule` / `match` などの構文順序ミス

### 3.3 確認手順
1. エラー位置前後の 5〜10 行を目視確認する。
2. 直前のフォーム終端 `)` が閉じ切っているかを確認する。
3. `match` や `let` の入れ子深度を浅くして再実行する。

### 3.4 最小修正例

誤り:

```lisp
(defn f ((x Bool)) Bool
  (if x true false
```

修正:

```lisp
(defn f ((x Bool)) Bool
  (if x true false))
```

---

## 4. `E-RESOLVE`

### 4.1 典型症状
- 未定義識別子（relation/function/constructor/sort）
- 重複定義
- `unsafe rule`（ヘッド変数が正リテラルで束縛されない）

### 4.2 主な原因
- タイポまたは定義漏れ
- 同名宣言の重複投入（import を含む）
- `rule` の body が変数束縛を提供していない

### 4.3 確認手順
1. 当該識別子が `sort` / `data` / `relation` / `defn` のいずれかで定義済みか確認する。
2. import を含め、同名定義が 2 回以上入っていないか確認する。
3. `rule` は「ヘッド変数 ⊆ body の正リテラル変数」を満たすか確認する。

### 4.4 最小修正例（unsafe rule）

誤り:

```lisp
(relation p (Symbol))
(rule (p ?x) true)
```

修正:

```lisp
(relation base (Symbol))
(relation p (Symbol))
(rule (p ?x) (base ?x))
```

---

## 5. `E-TYPE`

### 5.1 典型症状
- 関数引数/戻り値の型不一致
- `if` 条件が `Bool` でない
- relation 引数に非対応式を渡している

### 5.2 主な原因
- `Symbol` と `Domain` / `Adt` の混同
- constructor 引数個数・型の不一致
- relation 引数に任意式（関数呼び出しなど）を渡している

### 5.3 確認手順
1. 関数シグネチャの引数型と実引数型を 1 対 1 で突き合わせる。
2. `data` 型には constructor か同型変数のみを渡す。
3. relation 呼び出し引数は変数/リテラル/constructor に限定する。

### 5.4 最小修正例（`Symbol` と `Adt` の混同）

誤り:

```lisp
(data 顧客種別 (法人) (個人))
(defn 判定 ((k 顧客種別)) Bool true)
(defn 呼出 ((x Symbol)) Bool (判定 x))
```

修正:

```lisp
(data 顧客種別 (法人) (個人))
(defn 判定 ((k 顧客種別)) Bool true)
(defn 呼出 () Bool (判定 (法人)))
```

---

## 6. `E-MATCH`

### 6.1 典型症状
- 非網羅分岐（`non-exhaustive match`）
- 到達不能分岐（`unreachable match arm`）
- constructor パターン型不一致

### 6.2 主な原因
- ADT constructor の一部しか列挙していない
- `_` を先頭で使い後続分岐を死文化している
- パターンの constructor 引数個数が誤っている

### 6.3 確認手順
1. `data` 宣言の constructor 一覧と `match` 分岐が一致しているか確認する。
2. `_` は原則末尾に置く。
3. constructor パターンの子パターン数が定義と一致するか確認する。

### 6.4 最小修正例（非網羅）

誤り:

```lisp
(data Subject (alice) (bob))
(defn is-alice ((u Subject)) Bool
  (match u
    ((alice) true)))
```

修正:

```lisp
(data Subject (alice) (bob))
(defn is-alice ((u Subject)) Bool
  (match u
    ((alice) true)
    ((bob) false)))
```

---

## 7. `E-PROVE`

### 7.1 典型症状
- `missing universe declaration for type: ...`
- `universe ... must not be empty`
- 反例付きで `status=error`

### 7.2 主な原因
- 量化変数型に対する `universe` 宣言漏れ
- `universe` が空集合
- 仕様（`assert` / `Refine`）を満たす rule/fact が不足

### 7.3 確認手順
1. `assert` / `defn` パラメータ型すべてに `universe` があるか確認する。
2. `proof-trace.json` の `counterexample.missing_goals` を最優先で読む。
3. 欠落ゴールを導出できる `fact` または `rule` を追加し再検証する。

### 7.4 最小修正例（universe 漏れ）

誤り:

```lisp
(sort Subject)
(relation allowed (Subject))
(assert everyone ((u Subject)) (allowed u))
```

修正:

```lisp
(sort Subject)
(relation allowed (Subject))
(universe Subject (alice bob))
(assert everyone ((u Subject)) (allowed u))
```

---

## 8. 運用上の優先順位
1. `E-PARSE` を最優先で解消する（後続フェーズの意味がないため）。
2. 次に `E-RESOLVE` / `E-TYPE` / `E-MATCH` を `check` 段階でゼロ化する。
3. 最後に `E-PROVE` を `proof-trace.json` ベースで解消する。

この順を崩すと、誤差分（解析不能な連鎖エラー）で調査コストが増えます。
