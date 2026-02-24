# CLAUDE.md (docs)

この階層は仕様・設計文書の原本。

## 重要
- `docs-site/src/reference/language-spec.md` は `docs/language-spec.md` を include している。
- `docs-site/src/reference/language-guide.md` は `docs/language-guide-ja.md` を include している。
- `docs-site/src/reference/troubleshooting.md` は `docs/troubleshooting-errors-ja.md` を include している。

## 編集ルール
- 仕様変更はまず `docs/` 側を更新する。
- バージョン表記（v0.2/v0.3/v0.4）を本文と目次で整合させる。
- 実装追随が必要な変更は `README.md` と `docs-site/src/reference/codes.md` も確認する。

## 最低確認
- `mdbook build docs-site`
