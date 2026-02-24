# DTL Syntax Extension

`domain-typed-lisp` (`.dtl`) 用の VSCode/Cursor 拡張です。

## 機能

- `.dtl` ファイルのシンタックスハイライト
- Core / Surface キーワード、タグ、`?var`、型、真偽値、整数、コメントの色分け
- Markdown fenced code (`dtl` / `lisp`) で DTL ハイライトを適用

## 開発手順

```bash
# 生成物を更新（リポジトリルートで実行）
bun install --cwd tooling/dtl-syntax
bun run --cwd tooling/dtl-syntax generate

# VSIX を作成
bun install --cwd editors/vscode-dtl
bun run --cwd editors/vscode-dtl package
```

## インストール（ローカル）

現在は VS Code Marketplace / Open VSX 未公開です。  
このリポジトリから `.vsix` を生成してインストールしてください。

1. VSCode/Cursor の Extensions 画面を開く
2. `Install from VSIX...` を選択
3. `editors/vscode-dtl/*.vsix` を選択
