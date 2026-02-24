# HTML 生成とローカル確認

## 前提

- `mdbook` がインストール済み

未導入の場合:

```bash
cargo install mdbook --locked
```

## ビルド

```bash
# 利用例定義（examples/catalog.tsv）を更新後に再生成
./scripts/generate-examples-catalog.sh
python3 ./scripts/generate-glossary-assets.py

mdbook build docs-site
```

生成先:

- `docs-site/book/index.html`
- `docs-site/src/tutorial/examples-catalog.md`（自動生成）
- `docs-site/src/reference/glossary.md`（自動生成）
- `docs-site/theme/dtl-terms.js`（自動生成）

## ローカル確認

```bash
# カタログ更新を含めて起動
./scripts/docs-site-serve.sh

# 直接 mdbook を使う場合
mdbook serve docs-site --open
```

既定で `http://localhost:3000` に起動します。
