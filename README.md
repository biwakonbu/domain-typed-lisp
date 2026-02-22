# domain-typed-lisp (dtl)

![CI](https://img.shields.io/github/actions/workflow/status/biwakonbu/domain-typed-lisp/ci.yml?branch=main&label=ci)
[![Coverage](https://codecov.io/gh/biwakonbu/domain-typed-lisp/branch/main/graph/badge.svg)](https://codecov.io/gh/biwakonbu/domain-typed-lisp)

`dtl` は、ドメイン定義 DSL を純粋・非破壊に検査/証明/文書化するための Lisp 系言語です。

- 静的検査: 型整合・層化否定・`match` 網羅性・全域性（再帰禁止）
- 有限モデル証明: `assert` と `defn` 契約を universe 上で全探索
- ドキュメント生成: 証明成功時のみ `spec.md` または `spec.json` と `proof-trace.json` / `doc-index.json` を出力
- 識別子は Unicode 対応（通常 Atom は NFC 正規化。quoted Atom は非正規化・エスケープ非解釈）
- 意味固定は `data` constructor の正規名で行い、概念差分は型分離 + `defn` 変換で表現

## クイックスタート
```bash
cargo build
cargo run -- check examples/access_control_ok.dtl
cargo run -- prove examples/customer_contract_ja.dtl --format json --out out
cargo run -- doc examples/customer_contract_ja.dtl --out out --format markdown
cargo run -- doc examples/customer_contract_ja.dtl --out out_json --format json

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
- `--format json` の `diagnostics[].source` は、複数ファイル入力や `import` 経由でも実際のエラー発生ファイルを指す。

### `prove`
```bash
dtl prove <FILE>... [--format text|json] [--out DIR]
```
- 有限モデル検証を実行し、`--out` 指定時は `proof-trace.json` を生成する。

### `doc`
```bash
dtl doc <FILE>... --out DIR [--format markdown|json]
```
- すべての義務が証明された場合のみ成果物を出力する。
  - `--format markdown`: `spec.md` / `proof-trace.json` / `doc-index.json`
  - `--format json`: `spec.json` / `proof-trace.json` / `doc-index.json`

## 検証コマンド
```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --lib --bins --tests
cargo bench --bench perf_scaling -- solve_facts/fact_scaling/20 --quick --noplot
cargo bench --bench perf_scaling -- solve_facts/rule_scaling/10 --quick --noplot
cargo bench --bench perf_scaling -- prove/minimize_counterexample/4 --quick --noplot
```

## ドキュメント
- [言語仕様 v0.2](docs/language-spec.md)
- [言語解説ガイド v0.2](docs/language-guide-ja.md)
- [エラーコード別トラブルシュート v0.2](docs/troubleshooting-errors-ja.md)
- [v0.2 アーキテクチャ](docs/architecture-v0.2.md)
- [v0.2 移行ガイド](docs/migration-v0.2.md)
- [検証計画](docs/verification-plan.md)
- [テストマトリクス](docs/test-matrix.md)
- [v0.3 停止性解析設計](docs/termination-analysis-v0.3.md)
- [ADT Parametric 化評価 v0.3](docs/adt-parametric-evaluation-v0.3.md)
- [ADR 0001: import 名前空間と公開制御](docs/adr/0001-import-namespace.md)
