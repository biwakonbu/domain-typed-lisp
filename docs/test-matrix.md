# テストマトリクス（v0.4）

| ID | 種別 | 入力概要 | 期待結果 | 関連仕様 |
|---|---|---|---|---|
| P-01 | parser 正常 | `data/assert/universe/match` を含む構成 | parse 成功 | language-spec §3/§4 |
| P-02 | parser 異常 | `data` constructor なし | `E-PARSE` | language-spec §3.3 |
| P-03 | parser 異常 | `match` arm 形状不正 | `E-PARSE` | language-spec §4 |
| P-04 | parser 正常 | surface タグ構文（`型/データ/関係`） | parse 成功 | language-spec §3.10 |
| P-05 | parser 異常 | surface 主要フォームでタグ欠落 | `E-PARSE` | language-spec §3.10 |
| P-06 | parser 異常 | `syntax:auto` で Core/Surface 混在 | `E-SYNTAX-AUTO` | language-spec §1/§9 |
| R-01 | resolve 異常 | constructor 重複 | `E-DATA` | language-spec §3.3 |
| R-02 | resolve 正常 | 再帰 ADT（`(data List (nil) (cons Symbol List))`） | 成功 | language-spec §3.3 |
| R-03 | resolve 異常 | 未定義 universe 型 | `E-RESOLVE` | language-spec §3.8 |
| R-04 | resolve 異常 | pattern の未知 constructor | `E-RESOLVE` | language-spec §4 |
| T-01 | type 正常 | constructor + 網羅 `match` | 成功 | language-spec §4/§7 |
| T-02 | type 異常 | 非網羅 `match` | `E-MATCH` | language-spec §7 |
| T-03 | type 異常 | 到達不能 `match` arm | `E-MATCH` | language-spec §7 |
| T-04 | type 異常 | 非構造再帰（非 tail / 非減少） | `E-TOTAL` | language-spec §0/§7 |
| T-05 | type 異常 | `Domain` を `Symbol` に暗黙渡し | `E-TYPE` | language-spec §7 |
| T-06 | type 異常 | 相互再帰 | `E-TOTAL` | language-spec §0/§7 |
| T-07 | type 正常 | `let` alias 経由の strict subterm 再帰 | 成功 | language-spec §7 |
| T-08 | type 正常 | ネスト `match` 下での strict subterm 再帰 | 成功 | language-spec §7 |
| T-09 | type 正常 | 複数 ADT 引数のうち 1 つが減少する再帰 | 成功 | language-spec §7 |
| T-10 | type 異常 | 複数 ADT 引数で減少なし | `E-TOTAL` + `arg_indices` | language-spec §2.1/§7 |
| L-01 | logic 正常 | ADT 項を含む rule 導出 | 導出成功 | language-spec §6 |
| PR-01 | prover 正常 | universe 完備 + 真の assert | 全義務 `proved` | language-spec §7 |
| PR-02 | prover 異常 | assert 失敗 | `result=failed` + 反例 | language-spec §7 |
| PR-03 | prover 異常 | universe 欠落 | `E-PROVE` | language-spec §7 |
| C-01 | CLI 正常 | `check` | exit 0 | language-spec §2 |
| C-02 | CLI 正常 | `prove --format json --out` | exit 0 + `proof-trace.json` | language-spec §2/§8 |
| C-03 | CLI 異常 | `prove` 失敗義務 | exit 1 + failed obligation | language-spec §2/§7 |
| C-04 | CLI 正常 | `doc --format markdown` 証明成功ケース | `spec.md` / `proof-trace.json` / `doc-index.json` | language-spec §8 |
| C-05 | CLI 異常 | `doc` 未証明ケース | exit 1 + 生成抑止 | language-spec §8 |
| C-06 | CLI 正常 | `doc --format json` 証明成功ケース | `spec.json` / `proof-trace.json` / `doc-index.json` | language-spec §8 |
| C-07 | CLI 異常 | `check --format json` の `E-TOTAL` | `reason` / `arg_indices` を出力 | language-spec §2.1 |
| C-08 | CLI 正常 | `lint --format json` 重複ケース | `L-DUP-EXACT` warning を返却 | language-spec §2.1/§10 |
| C-09 | CLI 異常 | `lint --deny-warnings` | warning ありで exit 1 | language-spec §2 |
| C-10 | CLI 異常 | `fmt --check` 差分あり | exit 1 | language-spec §2 |
| C-11 | CLI 正常 | `doc --format markdown --pdf`（pandoc 不足） | Markdown 成果物生成 + warning | language-spec §8 |
| C-12 | CLI 正常 | `lint --semantic-dup` + universe 不足 | `L-DUP-SKIP-UNIVERSE` warning | language-spec §2/§10 |
| C-13 | CLI 正常 | `lint --semantic-dup`（同値 `assert/rule/defn`） | `L-DUP-MAYBE` を3種別で返却 | language-spec §10 |
| C-14 | CLI 正常 | `lint --semantic-dup`（非同値 defn） | `L-DUP-MAYBE` 非出力 | language-spec §10 |
| C-15 | E2E 正常 | `complex_policy_import_entry.dtl`（import + Surface + prove） | `check/prove` とも `status=ok` | language-spec §2/§3/§8 |
| C-16 | E2E 正常 | `recursive_nested_ok.dtl`（ネスト `match` + `let` alias 再帰） | `check/prove` とも `status=ok` | language-spec §7 |
| C-17 | CLI 正常 | `lint --semantic-dup`（探索量差分あり同値 assert） | `confidence` が探索量に応じて増加 | language-spec §10 |
| C-18 | CLI 正常 | `lint --semantic-dup`（function 型パラメータ defn） | `L-DUP-MAYBE` を返却 | language-spec §10 |
| C-19 | CLI 正常 | `lint --semantic-dup`（深い再帰 defn） | `L-DUP-SKIP-EVAL-DEPTH` warning | language-spec §10 |
