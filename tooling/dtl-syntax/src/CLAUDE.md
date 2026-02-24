# CLAUDE.md (tooling/dtl-syntax/src)

この階層は生成器の実装本体。

## 役割
- `syntax-spec.ts`: 単一ソースのトークン定義
- `generate-textmate.ts`: TextMate grammar 生成
- `generate-highlightjs.ts`: mdBook 用 highlight.js 生成
- `emit.ts`: 出力先への書き込みと `--check` 制御

## 編集ルール
- 仕様追加時は「spec -> generator -> test -> 生成物」の順で更新する。
- 生成出力の改行・順序を不必要に変えない（差分ノイズ防止）。
