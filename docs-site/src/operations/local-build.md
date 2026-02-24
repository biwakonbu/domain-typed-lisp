# HTML 生成とローカル確認

## 前提

- `mdbook` がインストール済み

未導入の場合:

```bash
cargo install mdbook --locked
```

## ビルド

```bash
# examples から利用例カタログを再生成
./scripts/generate-examples-catalog.sh

mdbook build docs-site
```

生成先:

- `docs-site/book/index.html`

## ローカル確認

```bash
# カタログ更新を含めて起動
./scripts/docs-site-serve.sh

# 直接 mdbook を使う場合
mdbook serve docs-site --open
```

既定で `http://localhost:3000` に起動します。
