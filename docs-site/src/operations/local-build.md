# HTML 生成とローカル確認

## 前提

- `mdbook` がインストール済み

未導入の場合:

```bash
cargo install mdbook --locked
```

## ビルド

```bash
mdbook build docs-site
```

生成先:

- `docs-site/book/index.html`

## ローカル確認

```bash
mdbook serve docs-site --open
```

既定で `http://localhost:3000` に起動します。
