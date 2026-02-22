# domain-typed-lisp (dtl)

![CI](https://img.shields.io/github/actions/workflow/status/biwakonbu/domain-typed-lisp/ci.yml?branch=main&label=ci)
[![Coverage](https://codecov.io/gh/biwakonbu/domain-typed-lisp/branch/main/graph/badge.svg)](https://codecov.io/gh/biwakonbu/domain-typed-lisp)

Datalog 系 Horn 節 + 層化否定を土台に、Refinement 型 `{x:T | P(x)}` を統合した静的型解析向け Lisp DSL の MVP 実装です。

この言語での計算は目的ではなく、ドメイン上の論理的一貫性を型とランタイム導出で検証するための手段として扱います。

## MVP の目的
- `dtl check <FILE>...` で 1 つ以上の DSL ファイルを静的検査する。
- 型整合性、Refinement 含意判定、層化否定検査を実施する。
- 実行器は提供せず、静的解析 CLI に限定する。

## 非ゴール
- 高度なモジュールシステム（名前空間、公開制御、再エクスポート）
- 実行器・評価器
- SMT 連携
- 依存型機能

## クイックスタート
```bash
cargo build
cargo run -- check examples/access_control_ok.dtl
cargo run -- check examples/access_control_ok.dtl --format json
cargo run -- check examples/access_control_split_schema.dtl examples/access_control_split_policy.dtl
cargo run -- check examples/access_control_import_entry.dtl
```

## CLI 出力
- `dtl check <FILE>...`: 人間可読な診断を出力（既定）
- `dtl check <FILE>... --format json`: 機械可読 JSON を標準出力へ出力
  - 成功: `{"status":"ok","report":{"functions_checked":...,"errors":0}}`
  - 失敗: `{"status":"error","diagnostics":[{"code":"...","message":"...","source":"..."}]}`

## 検証コマンド
```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --lib --bins --tests
cargo test --test integration_cli
cargo test --test property_logic
cargo llvm-cov --workspace --all-features --summary-only --fail-under-lines 80
cargo bench --bench perf_scaling
```

## ドキュメント
- [MVP ゴール](docs/mvp-goal.md)
- [言語仕様](docs/language-spec.md)
- [複数ファイル入力の最小設計](docs/multi-file-minimal-design.md)
- [検証計画](docs/verification-plan.md)
- [テストマトリクス](docs/test-matrix.md)
- [ADR: MVP アーキテクチャ](docs/adr/0001-mvp-architecture.md)
