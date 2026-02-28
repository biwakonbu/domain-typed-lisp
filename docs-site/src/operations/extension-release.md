# 拡張公開フロー

`editors/vscode-dtl` の VS Code Marketplace / Open VSX 公開は
`.github/workflows/extension-release.yml` で自動化されています。

## トリガー

- `v*` タグ push（例: `v0.2.0`）
- `workflow_dispatch`（`tag` 入力で preflight 検証のみ）

## 必須シークレット

- `VSCE_PAT`: VS Code Marketplace publish token
- `OVSX_PAT`: Open VSX publish token

## 実行内容

1. タグ版数と `editors/vscode-dtl/package.json` の `version` 一致を検証
2. syntax 資産を生成 (`tooling/dtl-syntax`)
3. VSIX を生成
4. VS Code Marketplace へ publish
5. Open VSX へ publish

`workflow_dispatch` の場合は preflight のみ実行し、publish ジョブは実行しません。  
どれか 1 つでも失敗した場合は workflow を失敗として終了します。
