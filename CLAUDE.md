# CLAUDE.md (repo root)

このファイルはリポジトリ全体の共通運用ルール。より深い階層に `CLAUDE.md` がある場合は、その指示を優先する。

## リポジトリ概要
- Rust 製 CLI: `dtl`（`check` / `prove` / `doc` / `lint` / `fmt`）
- 主要コード: `src/`
- テスト: `tests/`
- ドキュメント原本: `docs/`
- ドキュメントサイト: `docs-site/`（mdBook）
- シンタックス生成器: `tooling/dtl-syntax/`
- VSCode/Cursor 拡張: `editors/vscode-dtl/`

## 変更時の原則
- 生成物は原則「生成元」を編集する。手編集は最小限。
- 仕様変更時は、実装・テスト・ドキュメントを同一コミット系列で同期する。
- 日本語ドキュメントでは用語の表記ゆれを避ける（`sort`/`data`/`relation` など）。

## 生成物マップ
- `docs-site/src/tutorial/examples-catalog.md`
  - 生成元: `examples/catalog.tsv`, `scripts/generate-examples-catalog.sh`
- `docs-site/src/reference/glossary.md`, `docs-site/theme/dtl-terms.js`
  - 生成元: `docs-site/src/reference/glossary-terms.json`, `scripts/generate-glossary-assets.py`
- `editors/vscode-dtl/syntaxes/dtl.tmLanguage.json`, `docs-site/theme/dtl-highlight.js`
  - 生成元: `tooling/dtl-syntax/src/*`, `bun run --cwd tooling/dtl-syntax generate`
- `docs-site/book/**`
  - 生成物: `mdbook build docs-site`
- `editors/vscode-dtl/*.vsix`
  - 生成物: `bun run --cwd editors/vscode-dtl package`

## 代表検証コマンド
- Rust:
  - `cargo fmt --all -- --check`
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  - `cargo test --workspace --lib --bins --tests`
- docs:
  - `./scripts/generate-examples-catalog.sh`
  - `python3 ./scripts/generate-glossary-assets.py --check`
  - `mdbook build docs-site`
- syntax / extension:
  - `bun run --cwd tooling/dtl-syntax check-generated`
  - `bun run --cwd tooling/dtl-syntax test`
  - `bun run --cwd editors/vscode-dtl package`

## 変更対象外（通常）
- `target/`
- `docs-site/book/`
- `**/node_modules/`
