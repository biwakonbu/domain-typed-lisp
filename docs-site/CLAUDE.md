# CLAUDE.md (docs-site)

この階層は mdBook サイト定義。

## 構成
- 設定: `book.toml`
- 原稿: `src/`
- テーマ拡張: `theme/`
- 生成物: `book/`（コミット対象外）

## 編集ルール
- `src/` の原稿だけ変更しても、生成ファイル更新が必要なケースがある。
- docs 変更時は次を順に実行する。
  1. `./scripts/generate-examples-catalog.sh`
  2. `python3 ./scripts/generate-glossary-assets.py`
  3. `mdbook build docs-site`

## 注意
- `theme/dtl-highlight.js` と `theme/dtl-terms.js` は生成フローを優先して更新する。
