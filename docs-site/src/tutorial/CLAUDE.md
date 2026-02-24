# CLAUDE.md (docs-site/src/tutorial)

この階層はチュートリアル文書。

## ファイル種別
- 手編集: `index.md`, `quickstart.md`, `first-policy.md`, `semantic-dup.md`, `multi-file.md`
- 自動生成: `examples-catalog.md`

## 編集ルール
- `examples/*.dtl` を追加・削除したら `examples/catalog.tsv` を更新し、`examples-catalog.md` を再生成する。
- 実行コマンド例は `README.md` と CI で実行されるコマンド群に合わせる。

## 生成
- `./scripts/generate-examples-catalog.sh`
