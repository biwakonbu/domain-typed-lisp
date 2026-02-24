# CLAUDE.md (tooling/dtl-syntax/test)

この階層は生成器のテスト。

## 編集ルール
- 生成結果の決定性と必須トークン包含を壊さない。
- 仕様追加時は `syntax-generator.test.ts` に期待値を追加する。

## 実行
- `bun run --cwd tooling/dtl-syntax test`
