# CLAUDE.md (tooling/dtl-syntax)

この階層は DTL 構文資産の生成器（TypeScript + Bun）。

## 主要コマンド
- `bun run --cwd tooling/dtl-syntax generate`
- `bun run --cwd tooling/dtl-syntax check-generated`
- `bun run --cwd tooling/dtl-syntax test`

## 出力先
- `editors/vscode-dtl/syntaxes/dtl.tmLanguage.json`
- `docs-site/theme/dtl-highlight.js`

## 編集ルール
- 出力は決定的（deterministic）であること。
- `--check` は書き込みせず差分検出だけを行うこと。
- 新規トークン追加時は `src/syntax-spec.ts` とテスト期待値を同時更新する。
