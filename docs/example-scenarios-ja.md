# 複雑シナリオ集（v0.4）

この文書は、実装済み機能を前提に「実運用で詰まりやすい複合ケース」をサンプルで示す。

## 1. マルチファイル + Surface + 複数 `@context` + `prove`

対象ファイル:
- `examples/complex_policy_schema.dtl`
- `examples/complex_policy_rules.dtl`
- `examples/complex_policy_import_entry.dtl`

このシナリオで確認できる点:
- Surface 構文（日本語タグ）
- `@context` 分割された1ファイル内構成
- `import` による分割定義の統合
- `defn`（構造再帰 + `Refine`）と `assert` の同時証明

実行コマンド:

```bash
cargo run -- check examples/complex_policy_import_entry.dtl --format json
cargo run -- prove examples/complex_policy_import_entry.dtl --format json --out out_complex
cargo run -- doc examples/complex_policy_import_entry.dtl --out out_complex_doc --format markdown
```

期待:
- `check`: `status=ok`
- `prove`: `status=ok` かつ `defn::注文可能判定` / `assert::注文矛盾なし` が `proved`
- `doc`: `spec.md` / `proof-trace.json` / `doc-index.json` を生成

---

## 2. `lint --semantic-dup` の有限モデル厳密判定

対象ファイル:
- `examples/semantic_dup_advanced.dtl`

このシナリオで確認できる点:
- `rule` / `assert` / `defn` の同値候補を `L-DUP-MAYBE` で検出
- 変数名差分・`and ... true`・`if true ... false` のような構文差分があっても同値判定

実行コマンド:

```bash
cargo run -- lint examples/semantic_dup_advanced.dtl --format json --semantic-dup
```

期待:
- `diagnostics[].lint_code` に `L-DUP-MAYBE` が3件（`rule` / `assert` / `defn`）
- `diagnostics[].confidence` は固定値ではなく、探索カバレッジに応じて変動

---

## 3. ネスト `match` + `let` alias 構造再帰

対象ファイル:
- `examples/recursive_nested_ok.dtl`

このシナリオで確認できる点:
- `let` alias を挟んだ strict subterm 再帰
- ネスト `match` 下での自己再帰
- `check` と `prove` の両方通過

実行コマンド:

```bash
cargo run -- check examples/recursive_nested_ok.dtl --format json
cargo run -- prove examples/recursive_nested_ok.dtl --format json --out out_recursive
```

期待:
- `check`: `status=ok`
- `prove`: `status=ok`

---

## 4. `fmt` の `@context` 保持

対象ファイル:
- `examples/complex_policy_rules.dtl`
- `examples/semantic_dup_advanced.dtl`

実行コマンド:

```bash
cargo run -- fmt examples/complex_policy_rules.dtl examples/semantic_dup_advanced.dtl --check
```

期待:
- exit code 0
- `; @context:` 区切りが保持され、再整形しても差分が出ない（idempotent）
