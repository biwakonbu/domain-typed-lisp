# CLAUDE.md (docs-site/theme)

この階層は mdBook テーマ拡張資産。

## ファイル種別
- 手編集: `custom.css`
- 自動生成: `dtl-highlight.js`, `dtl-terms.js`

## 生成元
- `dtl-highlight.js`: `bun run --cwd tooling/dtl-syntax generate`
- `dtl-terms.js`: `python3 ./scripts/generate-glossary-assets.py`

## 編集ルール
- 生成対象ファイルは直接編集しない。
- 表示崩れを防ぐため、CSS変更後は `mdbook serve docs-site --open` で確認する。
