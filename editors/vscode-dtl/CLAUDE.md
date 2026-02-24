# CLAUDE.md (editors/vscode-dtl)

この階層は VSCode/Cursor 拡張本体。

## 主要ファイル
- `package.json`: 拡張メタデータ/寄与ポイント
- `language-configuration.json`: 括弧・コメント等
- `syntaxes/*.json`: TextMate grammar
- `snippets/*.code-snippets`: スニペット

## 生成物
- `syntaxes/dtl.tmLanguage.json` は `tooling/dtl-syntax` から生成
- `dtl-0.1.0.vsix` は `bun run --cwd editors/vscode-dtl package` で生成

## 編集ルール
- キーワード追加時は `tooling/dtl-syntax/src/syntax-spec.ts` を先に更新する。
- `package.json` の `contributes` と実ファイル配置を一致させる。

## 検証
- `bun install --cwd tooling/dtl-syntax`
- `bun run --cwd tooling/dtl-syntax generate`
- `bun install --cwd editors/vscode-dtl`
- `bun run --cwd editors/vscode-dtl package`
