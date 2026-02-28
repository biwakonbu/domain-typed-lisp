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

`v*` タグ時に `.github/workflows/extension-release.yml` から
VS Code Marketplace / Open VSX へ自動公開します。  
ローカル検証時は、このリポジトリから `.vsix` を生成してインストールしてください。

Cursor CLI を利用できる場合は、リポジトリルートで `make install` を実行すると
`dtl-*.vsix` の生成と `cursor --install-extension` まで一括で実行できます。

1. VSCode/Cursor の Extensions 画面を開く
2. `Install from VSIX...` を選択
3. `editors/vscode-dtl/*.vsix` を選択
