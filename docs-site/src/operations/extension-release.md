# 拡張公開フロー

`editors/vscode-dtl` の配布は 2 段階です。

- GitHub Release への `.vsix` 添付: `.github/workflows/release.yml`
- VS Code Marketplace / Open VSX への追加公開: `.github/workflows/extension-release.yml`

## トリガー

- `v*` タグ push（例: `v0.2.0`）
- `workflow_dispatch`（`tag` 入力で preflight 検証のみ）

## シークレット

- `VSCE_PAT`: VS Code Marketplace publish token（未設定なら Marketplace publish を skip）
- `OVSX_PAT`: Open VSX publish token（未設定なら Open VSX publish を skip）

## 実行内容

1. `release.yml` で CLI バイナリと `dtl-*.vsix` を GitHub Release に添付
2. `extension-release.yml` でタグ版数と `editors/vscode-dtl/package.json` の `version` 一致を検証
3. syntax 資産を生成 (`tooling/dtl-syntax`)
4. VSIX を生成
5. `VSCE_PAT` があれば VS Code Marketplace へ publish
6. `OVSX_PAT` があれば Open VSX へ publish

`workflow_dispatch` の場合は preflight のみ実行し、publish ジョブは実行しません。  
GitHub Release だけで配布する運用では secrets は不要です。
