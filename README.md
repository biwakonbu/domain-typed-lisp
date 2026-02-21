# domain-typed-lisp (dtl)

![CI](https://img.shields.io/github/actions/workflow/status/biwakonbu/domain-typed-lisp/ci.yml?branch=main&label=ci)
![Coverage](https://img.shields.io/badge/coverage-target%2080%25-blue)

Datalog 系 Horn 節 + 層化否定を土台に、Refinement 型 `{x:T | P(x)}` を統合した静的型解析向け Lisp DSL の MVP 実装です。

## MVP の目的
- `dtl check <FILE>` で単一ファイル DSL の静的検査を行う。
- 型整合性、Refinement 含意判定、層化否定検査を実施する。
- 実行器は提供せず、静的解析 CLI に限定する。

## 非ゴール
- モジュールシステム（`import` など）
- 実行器・評価器
- SMT 連携
- 依存型機能

## クイックスタート
```bash
cargo build
cargo run -- check examples/access_control_ok.dtl
```

## 検証コマンド
```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets
cargo test --test integration_cli
cargo test --test property_logic
cargo llvm-cov --workspace --all-features --summary-only --fail-under-lines 80
```

## ドキュメント
- [MVP ゴール](docs/mvp-goal.md)
- [言語仕様](docs/language-spec.md)
- [検証計画](docs/verification-plan.md)
- [テストマトリクス](docs/test-matrix.md)
- [ADR: MVP アーキテクチャ](docs/adr/0001-mvp-architecture.md)
