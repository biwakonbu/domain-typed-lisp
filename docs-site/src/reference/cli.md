# CLI リファレンス

## check

```bash
dtl check <FILE>... [--format text|json]
```

- 構文/名前解決/層化否定/型/全域性/`match` を検査

## prove

```bash
dtl prove <FILE>... [--format text|json] [--out DIR]
```

- 有限モデル検証を実行
- `--out` で `proof-trace.json` を出力

## doc

```bash
dtl doc <FILE>... --out DIR [--format markdown|json] [--pdf]
```

- 証明成功時のみ成果物を出力
- `--pdf` は markdown 出力時のみ有効（失敗は warning）

## selfdoc

```bash
dtl selfdoc [--repo PATH] [--config PATH] --out DIR [--format markdown|json] [--pdf]
```

- `scan -> extract -> render selfdoc DSL -> parse/prove/doc` を一気通貫で実行
- `--config` 省略時は `<repo>/.dtl-selfdoc.toml`
- README の `<!-- selfdoc:cli-contracts:start -->` テーブルから CLI 契約を抽出

## selfcheck

```bash
dtl selfcheck [--repo PATH] [--config PATH] --out DIR [--format text|json] [--doc-format markdown|json] [--pdf]
```

- `selfdoc` フロー + 厳密チェック（`claim_coverage = 100%` 必須）
- 失敗時も `proof-trace.json` を出力

## lint

```bash
dtl lint <FILE>... [--format text|json] [--deny-warnings] [--semantic-dup]
```

- `--semantic-dup` で有限モデル同値判定を有効化
- `--deny-warnings` で warning を exit 1 化

## fmt

```bash
dtl fmt <FILE>... [--check] [--stdout]
```

- 既定は in-place
- `--check` は差分検出のみ
- `--stdout` は単一入力のみ
