# domain-typed-lisp (dtl)

![CI](https://img.shields.io/github/actions/workflow/status/biwakonbu/domain-typed-lisp/ci.yml?branch=main&label=ci)
[![Coverage](https://codecov.io/gh/biwakonbu/domain-typed-lisp/branch/main/graph/badge.svg)](https://codecov.io/gh/biwakonbu/domain-typed-lisp)

`dtl` は、ドメイン定義 DSL を純粋・非破壊に検査/証明/文書化するための Lisp 系言語です。

- 静的検査: 型整合・層化否定・`match` 網羅性・全域性（再帰禁止）
- 有限モデル証明: `assert` と `defn` 契約を universe 上で全探索
- ドキュメント生成: 証明成功時のみ `spec.md` と `proof-trace.json` を出力
- 識別子は Unicode 対応（内部では NFC 正規化、`import` パス文字列は除外）
- 意味固定は `data` constructor の正規名で行い、概念差分は型分離 + `defn` 変換で表現

## クイックスタート
```bash
cargo build
cargo run -- check examples/access_control_ok.dtl
cargo run -- prove examples/access_control_ok.dtl --format json --out out
cargo run -- doc examples/access_control_ok.dtl --out out

# 日本語ドメイン型サンプル
cargo run -- check examples/customer_contract_ja.dtl
cargo run -- prove examples/customer_contract_ja.dtl --format json --out out_ja
```

## CLI

### `check`
```bash
dtl check <FILE>... [--format text|json]
```
- 構文/名前解決/層化否定/型検査/全域性/`match` を検査する。

### `prove`
```bash
dtl prove <FILE>... [--format text|json] [--out DIR]
```
- 有限モデル検証を実行し、`--out` 指定時は `proof-trace.json` を生成する。

### `doc`
```bash
dtl doc <FILE>... --out DIR [--format markdown|json]
```
- すべての義務が証明された場合のみ `spec.md` / `proof-trace.json` / `doc-index.json` を出力する。

## 検証コマンド
```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --lib --bins --tests
```

## ドキュメント
- [言語仕様 v0.2](docs/language-spec.md)
- [v0.2 アーキテクチャ](docs/architecture-v0.2.md)
- [v0.2 移行ガイド](docs/migration-v0.2.md)
- [検証計画](docs/verification-plan.md)
- [テストマトリクス](docs/test-matrix.md)
