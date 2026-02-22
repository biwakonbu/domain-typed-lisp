# v0.1.x -> v0.2 移行ガイド

## 1. 破壊的変更
- CLI が `check` / `prove` / `doc` に分離された。
- 関数再帰（自己再帰・相互再帰）は `E-TOTAL` で禁止。
- `Symbol` と `Domain` の暗黙互換を廃止。
- `match` は網羅必須、到達不能分岐は `E-MATCH`。
- 証明実行には `universe` 宣言が必要（不足時 `E-PROVE`）。

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
```bash
dtl doc path/to/file.dtl --out out
```

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

## 4. よくあるエラー
- `E-TYPE`: constructor 呼び出し漏れ（`read` ではなく `(read)`）。
- `E-TOTAL`: 関数に再帰呼び出しが残っている。
- `E-MATCH`: `match` の分岐不足 or 到達不能分岐。
- `E-PROVE`: `universe` 未宣言、または反例あり。

## 5. 移行時チェックリスト
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --lib --bins --tests`
- `dtl prove <FILE> --format json --out out`
- `dtl doc <FILE> --out out`
