# CLAUDE.md (scripts)

この階層は docs/syntax 用の補助スクリプト。

## スクリプト責務
- `generate-examples-catalog.sh`: `examples/catalog.tsv` から `docs-site/src/tutorial/examples-catalog.md` を生成
- `generate-glossary-assets.py`: 用語台帳から `glossary.md` と `theme/dtl-terms.js` を生成
- `docs-site-build.sh` / `docs-site-serve.sh`: docs-site 生成とローカル確認

## 編集ルール
- 失敗時は非0で終了し、原因を stderr に出す。
- 相対パスをハードコードせず、`SCRIPT_DIR` / `REPO_ROOT` 基準で解決する。
- 生成対象と検証対象（`--check`）の挙動を一致させる。
