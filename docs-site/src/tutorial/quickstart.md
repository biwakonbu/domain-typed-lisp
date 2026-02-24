# クイックスタート

## 1. ビルド

```bash
cargo build
```

## 2. 静的検査

```bash
cargo run -- check examples/customer_contract_ja.dtl --format json
```

期待値:
- `status=ok`
- `report.functions_checked > 0`

## 3. 証明と成果物生成

```bash
cargo run -- prove examples/customer_contract_ja.dtl --format json --out out_ja
cargo run -- doc examples/customer_contract_ja.dtl --out out_ja_doc --format markdown
```

期待値:
- `out_ja/proof-trace.json` が生成される
- `out_ja_doc/spec.md` が生成される

## 4. lint / fmt 運用

```bash
cargo run -- lint examples/customer_contract_ja.dtl --format json --deny-warnings
cargo run -- fmt examples/customer_contract_ja.dtl --check
```

期待値:
- warning がなければ `lint` は exit 0
- 差分がなければ `fmt --check` は exit 0

## 5. semantic duplicate の確認

```bash
cargo run -- lint examples/semantic_dup_advanced.dtl --format json --semantic-dup

# function 型パラメータを含む defn 同値比較サンプル
cargo run -- lint examples/semantic_dup_function_param.dtl --format json --semantic-dup
```

期待値:
- `L-DUP-MAYBE` が `rule/assert/defn` の3種別で出る
- `semantic_dup_function_param.dtl` では `defn passthrough_a` / `passthrough_b` が `L-DUP-MAYBE`
