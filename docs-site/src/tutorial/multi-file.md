# 複数ファイル運用（import/@context）

`import` 分割と `@context` を併用する実運用例です。

## 1. check

```bash
cargo run -- check examples/complex_policy_import_entry.dtl --format json
```

確認点:
- `status=ok`
- `functions_checked=2`

## 2. prove

```bash
cargo run -- prove examples/complex_policy_import_entry.dtl --format json --out out_complex
```

確認点:
- `out_complex/proof-trace.json` が生成される

## 3. doc

```bash
cargo run -- doc examples/complex_policy_import_entry.dtl --out out_complex_doc --format markdown
```

確認点:
- `out_complex_doc/spec.md`
- `out_complex_doc/doc-index.json`

## 4. fmt の安定性

```bash
cargo run -- fmt examples/complex_policy_schema.dtl --check
cargo run -- fmt examples/complex_policy_rules.dtl --check
```

`@context` ブロックを維持したまま idempotent であることを確認します。
