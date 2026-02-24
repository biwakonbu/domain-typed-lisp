# GitHub Pages 運用

このリポジトリには `docs-site` を Pages 配信するワークフローを追加しています。

- ワークフロー: `.github/workflows/docs-site.yml`
- `./scripts/generate-examples-catalog.sh` で `examples/` から利用例カタログを再生成
- `main` ブランチ push 時に `mdbook build docs-site` を実行
- 成果物 `docs-site/book` を Pages へ deploy
- 現在の Pages 設定: `build_type=workflow` / 公開 URL `https://biwakonbu.github.io/domain-typed-lisp/`

## リポジトリ設定

1. GitHub のリポジトリ設定を開く
2. `Settings > Pages` で `Build and deployment` を `GitHub Actions` に設定

## 失敗時の確認ポイント

- `mdbook` のビルド失敗（リンク切れ・include パス誤り）
- Pages 権限不足（workflow permissions）
- `book.toml` の `site-url` と実際の公開パス不一致
