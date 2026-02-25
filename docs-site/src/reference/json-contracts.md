# JSON 契約リファレンス

CI や外部連携では、以下のトップレベル構造を前提に固定します。

## check

成功:

```json
{"status":"ok","report":{"functions_checked":1,"errors":0}}
```

失敗:

```json
{"status":"error","diagnostics":[{"code":"E-TYPE","message":"..."}]}
```

## prove

成功:

```json
{"status":"ok","proof":{"schema_version":"2.1.0","profile":"standard","summary":{"total":1,"proved":1,"failed":0},"claim_coverage":{"total_claims":1,"proved_claims":1},"obligations":[{"id":"assert::...","result":"proved"}]}}
```

失敗:

```json
{"status":"error","proof":{"schema_version":"2.1.0","profile":"standard","summary":{"total":1,"proved":0,"failed":1},"claim_coverage":{"total_claims":1,"proved_claims":0},"obligations":[{"result":"failed"}]}}
```

## lint

```json
{
  "status":"ok",
  "diagnostics":[
    {"severity":"warning","lint_code":"L-DUP-MAYBE","message":"...","confidence":0.93}
  ]
}
```

`--deny-warnings` 時は warning があれば `status="error"`。

## doc

`--format markdown`:
- `spec.md`
- `proof-trace.json`
- `doc-index.json`

`--format json`:
- `spec.json`
- `proof-trace.json`
- `doc-index.json`

`spec.json`（v2）必須フィールド:
- `schema_version: "2.0.0"`
- `profile: "standard" | "selfdoc"`
- `summary: {total, proved, failed}`
- `self_description: {project, modules, references, contracts, quality_gates}`

`doc-index.json`（v2）必須フィールド:
- `schema_version: "2.0.0"`
- `profile`
- `intermediate.dsl`（通常 `null`、`selfdoc` では `"selfdoc.generated.dtl"`）

## selfdoc

`dtl selfdoc --out DIR` は次を生成します。
- `selfdoc.generated.dtl`
- `proof-trace.json`
- `doc-index.json`
- `spec.md` または `spec.json`

## selfcheck

`dtl selfcheck --out DIR --format json` は `prove` 互換の JSON を返し、追加で `claim_coverage=100%` を要求します。

契約の実測例はテストを参照してください。
- `tests/e2e_examples.rs`
- `tests/integration_prove_json_contract.rs`
- `tests/integration_lint_fmt_doc_pdf_cli.rs`
- `tests/integration_selfdoc_cli.rs`
