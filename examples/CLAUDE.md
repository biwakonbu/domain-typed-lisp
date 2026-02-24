# CLAUDE.md (examples)

この階層は CLI 動作確認とドキュメント参照用のサンプル群。

## 主要ファイル
- `*.dtl`: サンプル入力
- `catalog.tsv`: 利用例カタログの定義（生成元）

## 編集ルール
- `*.dtl` を追加・削除したら `catalog.tsv` を必ず同期更新する。
- `catalog.tsv` は section (`[first]` など) + 3列TSV（file/purpose/command）を守る。
- import サンプルはエントリファイル経由で実行可能な状態を維持する。

## 検証
- `./scripts/generate-examples-catalog.sh`
- `cargo run -- check examples/my_first_policy.dtl --format json`
