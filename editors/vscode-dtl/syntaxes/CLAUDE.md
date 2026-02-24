# CLAUDE.md (editors/vscode-dtl/syntaxes)

この階層は TextMate grammar 定義。

## ファイル種別
- 自動生成: `dtl.tmLanguage.json`
- 手編集: `markdown-dtl-fence.tmLanguage.json`

## 編集ルール
- DTL キーワード・トークン仕様変更は `tooling/dtl-syntax/src/syntax-spec.ts` で行い、再生成する。
- Markdown フェンス注入規則の変更時は `README.md` と `package.json` の grammar 登録を確認する。

## 生成
- `bun run --cwd tooling/dtl-syntax generate`
- `bun run --cwd tooling/dtl-syntax check-generated`
