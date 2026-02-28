# CLAUDE.md (.github/workflows)

この階層は GitHub Actions ワークフロー本体。

## 既存ワークフローの責務
- `ci.yml`: 品質ゲート + docs + syntax + CLI + bench smoke
- `docs-site.yml`: docs-site ビルドと GitHub Pages デプロイ
- `release.yml`: tag (`v*`) トリガーのマルチプラットフォーム成果物配布
- `extension-release.yml`: tag (`v*`) トリガーの VS Code Marketplace / Open VSX 公開

## 編集ルール
- 既存の責務を跨いでジョブを混在させない。
- Rust/Bun/mdBook の実行コマンドはリポジトリ内スクリプト・READMEと一致させる。
- `paths` フィルタ更新時は、生成元ファイル（`scripts/`, `examples/`, `docs-site/src/reference/glossary-terms.json` など）を漏らさない。
