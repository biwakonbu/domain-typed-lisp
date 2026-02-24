# domain-typed-lisp (dtl)

![CI](https://img.shields.io/github/actions/workflow/status/biwakonbu/domain-typed-lisp/ci.yml?branch=main&label=ci)
[![Coverage](https://codecov.io/gh/biwakonbu/domain-typed-lisp/branch/main/graph/badge.svg)](https://codecov.io/gh/biwakonbu/domain-typed-lisp)

`dtl` は、ドメイン定義 DSL を純粋・非破壊に検査/証明/文書化するための Lisp 系言語です。

- 静的検査: 型整合・層化否定・`match` 網羅性・全域性（構造再帰判定）
- lint: 重複候補検出（`L-DUP-*`）と未使用宣言（`L-UNUSED-DECL`）
  - `L-DUP-MAYBE` は有限モデルでの双方向検証（`rule/assert` 含意・`defn` 戻り一致）
  - 深い再帰で比較不能な入力点は `L-DUP-SKIP-EVAL-DEPTH` で可視化
  - `confidence` はモデルカバレッジ + 反例探索結果に基づく動的スコア（0.00-0.99）
- format: Surface 形式への正規化整形（in-place / check / stdout）
  - `@context` ブロックを保持し、複数コンテキストでも idempotent 整形
- 有限モデル証明: `assert` と `defn` 契約を universe 上で全探索
- ドキュメント生成: 証明成功時のみ `spec.md` または `spec.json` と `proof-trace.json` / `doc-index.json` を出力（`--pdf` 対応）
- 識別子は Unicode 対応（通常 Atom は NFC 正規化。quoted Atom は非正規化・エスケープ非解釈）
- `syntax: auto` は Core/Surface 混在を検知すると `E-SYNTAX-AUTO` を返す
- 意味固定は `data` constructor の正規名で行い、概念差分は型分離 + `defn` 変換で表現

## クイックスタート
```bash
cargo build
cargo run -- check examples/access_control_ok.dtl
cargo run -- lint examples/access_control_ok.dtl --format json
cargo run -- fmt examples/access_control_ok.dtl --check
cargo run -- prove examples/customer_contract_ja.dtl --format json --out out
cargo run -- doc examples/customer_contract_ja.dtl --out out --format markdown
cargo run -- doc examples/customer_contract_ja.dtl --out out --format markdown --pdf
cargo run -- doc examples/customer_contract_ja.dtl --out out_json --format json

# 日本語ドメイン型サンプル
cargo run -- check examples/customer_contract_ja.dtl
cargo run -- prove examples/customer_contract_ja.dtl --format json --out out_ja

# 複雑シナリオ（マルチファイル + Surface + 複数 @context + prove/doc）
cargo run -- check examples/complex_policy_import_entry.dtl --format json
cargo run -- prove examples/complex_policy_import_entry.dtl --format json --out out_complex

# semantic duplicate 厳密判定サンプル
cargo run -- lint examples/semantic_dup_advanced.dtl --format json --semantic-dup
cargo run -- lint examples/semantic_dup_function_param.dtl --format json --semantic-dup

# ネスト match + let alias 構造再帰サンプル
cargo run -- check examples/recursive_nested_ok.dtl --format json

# 最小チュートリアルサンプル
cargo run -- check examples/my_first_policy.dtl --format json
```

## ドキュメントサイト（HTML）
```bash
# 初回のみ
cargo install mdbook --locked

# examples から利用例カタログを自動生成
./scripts/generate-examples-catalog.sh

# HTML を生成
./scripts/docs-site-build.sh

# ローカル確認
./scripts/docs-site-serve.sh
```

- 設定: `docs-site/book.toml`
- 利用例定義: `examples/catalog.tsv`（`[first]` などのセクション見出し + TSV 行）
- 目次: `docs-site/src/SUMMARY.md`
- 生成物: `docs-site/book/index.html`
- GitHub Pages 運用: `.github/workflows/docs-site.yml`

## シンタックスハイライト生成 / VSCode・Cursor 拡張
```bash
# 生成器の依存をインストール
bun install --cwd tooling/dtl-syntax

# TextMate grammar / mdBook highlight.js を生成
bun run --cwd tooling/dtl-syntax generate

# 生成物の差分チェック（CI 用）
bun run --cwd tooling/dtl-syntax check-generated

# 生成器テスト
bun run --cwd tooling/dtl-syntax test

# 拡張をパッケージ化（.vsix）
bun install --cwd editors/vscode-dtl
bun run --cwd editors/vscode-dtl package
```

- 共通構文定義: `tooling/dtl-syntax/src/syntax-spec.ts`
- TextMate 生成物: `editors/vscode-dtl/syntaxes/dtl.tmLanguage.json`
- mdBook 用ハイライト: `docs-site/theme/dtl-highlight.js`
- 拡張定義: `editors/vscode-dtl/package.json`

### 取得元とインストール
- 現在は VS Code Marketplace / Open VSX 未公開。
- 取得元はこのリポジトリ（`main`）のみ。
- インストール方法:
  1. `bun run --cwd editors/vscode-dtl package` で `editors/vscode-dtl/dtl-0.1.0.vsix` を生成
  2. VSCode/Cursor の Extensions 画面で `Install from VSIX...` を選択
  3. 生成した `.vsix` を指定してインストール

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
  - `--pdf`: markdown 出力後に `spec.pdf` 生成を試行（失敗は warning）
  - `--format json`: `spec.json` / `proof-trace.json` / `doc-index.json`

### `lint`
```bash
dtl lint <FILE>... [--format text|json] [--deny-warnings] [--semantic-dup]
```
- 重複検出と未使用宣言検出を warning として出力する。
- `--deny-warnings` を指定すると warning で exit code 1。

### `fmt`
```bash
dtl fmt <FILE>... [--check] [--stdout]
```
- 既定は in-place 整形。
- `--check` は差分検出のみ。
- `--stdout` は単一入力時に整形結果を標準出力。

## 検証コマンド
```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --lib --bins --tests
bun run --cwd tooling/dtl-syntax check-generated
bun run --cwd tooling/dtl-syntax test
bun run --cwd editors/vscode-dtl package
mdbook build docs-site
cargo bench --bench perf_scaling -- solve_facts/fact_scaling/20 --quick --noplot
cargo bench --bench perf_scaling -- solve_facts/rule_scaling/10 --quick --noplot
cargo bench --bench perf_scaling -- prove/minimize_counterexample/4 --quick --noplot
```

## ドキュメント
- [公開ドキュメントサイト](https://biwakonbu.github.io/domain-typed-lisp/)
- [ドキュメントサイト目次（mdBook）](docs-site/src/SUMMARY.md)
- [利用例カタログ](docs-site/src/tutorial/examples-catalog.md)
- [言語仕様 v0.4](docs/language-spec.md)
- [言語解説ガイド v0.4](docs/language-guide-ja.md)
- [エラーコード別トラブルシュート v0.4](docs/troubleshooting-errors-ja.md)
- [v0.2 アーキテクチャ](docs/architecture-v0.2.md)
- [v0.2 移行ガイド（v0.4 追補）](docs/migration-v0.2.md)
- [検証計画](docs/verification-plan.md)
- [テストマトリクス](docs/test-matrix.md)
- [複雑シナリオ集](docs/example-scenarios-ja.md)
- [v0.3 停止性解析設計](docs/termination-analysis-v0.3.md)
- [ADT Parametric 化評価 v0.3](docs/adt-parametric-evaluation-v0.3.md)
- [ADR 0001: import 名前空間と公開制御](docs/adr/0001-import-namespace.md)
