# CLAUDE.md (docs-site/src/reference)

この階層はリファレンス文書。

## ファイル種別
- 手編集: `cli.md`, `codes.md`, `index.md`, `json-contracts.md`, `glossary-terms.json`
- include ラッパー: `language-spec.md`, `language-guide.md`, `troubleshooting.md`
- 自動生成: `glossary.md`

## 編集ルール
- 用語追加/修正は `glossary-terms.json` を編集し、`glossary.md` を再生成する。
- 診断コード追加時は `codes.md` と実装の整合を取る。

## 生成
- `python3 ./scripts/generate-glossary-assets.py`
- `python3 ./scripts/generate-glossary-assets.py --check`
