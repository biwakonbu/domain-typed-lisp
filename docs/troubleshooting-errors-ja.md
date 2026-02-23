# エラーコード別トラブルシュート（v0.4）

この文書は `dtl` v0.4 の主要エラーコードと lint warning について、原因と復旧手順を運用視点で整理したものです。

## 1. 対象バージョン
- DSL 仕様: v0.4（`docs/language-spec.md` 準拠）
- 実装: `dtl` 本体（Rust, `rust-toolchain.toml` は `1.93.0`）

## 2. 使い方
1. まず `check --format json` または `prove --format json` で `diagnostics` を取得する。
2. つぎに `lint --format json` で `diagnostics[].lint_code` を確認する。
3. `code` / `lint_code` ごとに本書の該当節を参照し、再現条件を 1 つずつ潰す。
4. 最後に `fmt --check` と `doc` まで通して成果物生成を確認する。

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

### 3.5 `E-SYNTAX-AUTO`
- 症状: `syntax:auto 判定衝突` で parse が停止する。
- 原因: 同一ファイル内に Core 形式（例: `(relation p (Subject))`）と Surface 形式（例: `(関係 p :引数 (主体))`）が混在。
- 対処:
1. ファイル全体を Core か Surface のどちらかに統一する。
2. 暫定運用では先頭に `; syntax: core` または `; syntax: surface` を明示する。
3. 混在を維持したい場合はファイル分割し、`import` で統合する。

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

## 6. `E-TOTAL`

### 6.1 典型症状
- `recursive function is not in tail position`
- `recursive function is not structurally decreasing`
- `mutual recursion is not allowed`

### 6.2 主な原因
- 再帰呼び出しが tail position にない
- `match` 分解した部分値ではなく、元引数や非減少式を再帰に渡している
- 相互再帰（`f -> g -> f`）を使っている

### 6.3 確認手順
1. 再帰呼び出しが式の最終位置にあるかを確認する（`if` 条件式や引数式内は非 tail）。
2. ADT 引数について、`match` で分解した部分値（例: `(s m)` の `m`）を渡しているか確認する。
3. call graph を確認し、再帰が単一関数の自己再帰に閉じているか確認する。

### 6.4 最小修正例（非減少 -> 減少）

誤り:

```lisp
(data Nat (z) (s Nat))
(defn bad ((n Nat)) Bool
  (match n
    ((z) true)
    ((s m) (bad n))))
```

修正:

```lisp
(data Nat (z) (s Nat))
(defn ok ((n Nat)) Bool
  (match n
    ((z) true)
    ((s m) (ok m))))
```

---

## 7. `E-MATCH`

### 7.1 典型症状
- 非網羅分岐（`non-exhaustive match`）
- 到達不能分岐（`unreachable match arm`）
- constructor パターン型不一致

### 7.2 主な原因
- ADT constructor の一部しか列挙していない
- `_` を先頭で使い後続分岐を死文化している
- パターンの constructor 引数個数が誤っている

### 7.3 確認手順
1. `data` 宣言の constructor 一覧と `match` 分岐が一致しているか確認する。
2. `_` は原則末尾に置く。
3. constructor パターンの子パターン数が定義と一致するか確認する。

### 7.4 最小修正例（非網羅）

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

## 8. `E-PROVE`

### 8.1 典型症状
- `missing universe declaration for type: ...`
- `universe ... must not be empty`
- 反例付きで `status=error`

### 8.2 主な原因
- 量化変数型に対する `universe` 宣言漏れ
- `universe` が空集合
- 仕様（`assert` / `Refine`）を満たす rule/fact が不足

### 8.3 確認手順
1. `assert` / `defn` パラメータ型すべてに `universe` があるか確認する。
2. `proof-trace.json` の `counterexample.missing_goals` を最優先で読む。
3. 欠落ゴールを導出できる `fact` または `rule` を追加し再検証する。

### 8.4 最小修正例（universe 漏れ）

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

## 9. 運用上の優先順位
1. `E-PARSE` を最優先で解消する（後続フェーズの意味がないため）。
2. 次に `E-RESOLVE` / `E-TYPE` / `E-TOTAL` / `E-MATCH` を `check` 段階でゼロ化する。
3. `E-PROVE` を `proof-trace.json` ベースで解消する。
4. 最後に `lint` warning（重複/未使用）を解消し、`fmt --check` を通す。

この順を崩すと、誤差分（解析不能な連鎖エラー）で調査コストが増えます。

---

## 10. `L-DUP-EXACT`（lint warning）

### 10.1 典型症状
- 同一 `fact` / `rule` / `assert` / `defn` が重複警告される

### 10.2 主な原因
- import 先を含めた二重定義
- コピー&ペーストによる同一宣言の残骸

### 10.3 確認手順
1. 警告に出る最初の定義位置（line/column）を確認する。
2. 意図的重複でない限り片方を削除する。
3. 差分を `fmt --check` で固定する。

---

## 11. `L-DUP-MAYBE` / `L-DUP-SKIP-UNIVERSE`（lint warning）

### 11.1 典型症状
- `L-DUP-MAYBE`: 有限モデル上で同値の重複候補
- `L-DUP-SKIP-UNIVERSE`: semantic duplicate 判定スキップ

### 11.2 主な原因
- `rule/assert` が有限モデル上で双方向含意になる
- `defn` が全入力で同じ戻り値を返す
- `--semantic-dup` 実行時に universe が不足

### 11.3 確認手順
1. `--semantic-dup` を付けた実行か確認する。
2. `L-DUP-SKIP-UNIVERSE` が出る場合は不足型の `universe` を追加する。
3. `L-DUP-MAYBE` は `confidence`（0.00〜0.99）を併せて確認し、低スコアは追加検証（universe 拡張・反例作成）を行う。
4. 厳密判定の再現には `examples/semantic_dup_advanced.dtl` を使い、`rule/assert/defn` の3種別で検証する。

---

## 12. `L-UNUSED-DECL`（lint warning）

### 12.1 典型症状
- 未使用 `relation` / `defn` / `sort` / `data` / `universe` が警告される

### 12.2 主な原因
- 過去仕様の残骸
- import 再編後の参照切れ

### 12.3 確認手順
1. 実際に参照されているか検索する。
2. 将来使用予定がなければ削除する。
3. 将来使用予定があるなら TODO へ明示する。

---

## 13. `fmt --check` が失敗する

### 13.1 典型症状
- exit code 1（差分あり）

### 13.2 主な原因
- フォーマット未適用
- Surface 形式への未変換（既定レンダリングとの差分）

### 13.3 確認手順
1. `dtl fmt <FILE>` を実行する。
2. 再度 `dtl fmt <FILE> --check` を実行する。
3. CI では `--deny-warnings` と併用して運用する。
