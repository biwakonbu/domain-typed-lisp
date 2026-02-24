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
{"status":"ok","proof":{"obligations":[{"id":"assert::...","result":"proved"}]}}
```

失敗:

```json
{"status":"error","proof":{"obligations":[{"result":"failed"}]}}
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

契約の実測例はテストを参照してください。
- `tests/e2e_examples.rs`
- `tests/integration_prove_json_contract.rs`
- `tests/integration_lint_fmt_doc_pdf_cli.rs`
