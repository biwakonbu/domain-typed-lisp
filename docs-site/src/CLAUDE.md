# CLAUDE.md (docs-site/src)

この階層は mdBook の本文ソース。

## 主要ファイル
- `SUMMARY.md`: 目次
- `index.md`: トップページ
- `tutorial/`, `reference/`, `operations/`: セクション本文

## 編集ルール
- ページ追加/削除時は必ず `SUMMARY.md` を更新する。
- `reference/language-*.md` と `reference/troubleshooting.md` は include ラッパー。原本は `docs/` 側。
- 生成ファイルは手編集しない（詳細は下位 `CLAUDE.md`）。
