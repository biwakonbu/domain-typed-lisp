# テストマトリクス（v0.6）

| ID | 種別 | 入力概要 | 期待結果 | 関連仕様 |
|---|---|---|---|---|
| P-01 | parser 正常 | `data/assert/universe/match` を含む構成 | parse 成功 | language-spec §3/§4 |
| P-02 | parser 異常 | `data` constructor なし | `E-PARSE` | language-spec §3.3 |
| P-03 | parser 異常 | `match` arm 形状不正 | `E-PARSE` | language-spec §4 |
| P-04 | parser 正常 | surface タグ構文（`型/データ/関係`） | parse 成功 | language-spec §3.11 |
| P-05 | parser 異常 | surface 主要フォームでタグ欠落 | `E-PARSE` | language-spec §3.11 |
| P-06 | parser 異常 | `syntax:auto` で Core/Surface 混在 | `E-SYNTAX-AUTO` | language-spec §1/§9 |
| P-07 | parser 正常 | selfdoc Surface フォーム（`project/module/reference/contract/quality-gate`） | parse 成功（`fact` へデシュガ） | language-spec §3.12 |
| P-08 | parser 異常 | quoted Atom の未対応エスケープ | `E-PARSE` | language-spec §1.1 |
| P-09 | parser 正常 | constructor alias（Core `alias` / Surface `同義語`） | parse 成功 | language-spec §3.2 |
| R-01 | resolve 異常 | constructor 重複 | `E-DATA` | language-spec §3.3 |
| R-02 | resolve 正常 | 再帰 ADT（`(data List (nil) (cons Symbol List))`） | 成功 | language-spec §3.3 |
| R-03 | resolve 異常 | 未定義 universe 型 | `E-RESOLVE` | language-spec §3.8 |
| R-04 | resolve 異常 | pattern の未知 constructor | `E-RESOLVE` | language-spec §4 |
| R-05 | resolve 異常 | alias 循環/未定義 canonical | `E-RESOLVE` | language-spec §3.2 |
| T-01 | type 正常 | constructor + 網羅 `match` | 成功 | language-spec §4/§7 |
| T-02 | type 異常 | 非網羅 `match` | `E-MATCH` | language-spec §7 |
| T-03 | type 異常 | 到達不能 `match` arm | `E-MATCH` | language-spec §7 |
| T-04 | type 異常 | 非構造再帰（非 tail / 非減少） | `E-TOTAL` | language-spec §0/§7 |
| T-05 | type 異常 | `Domain` を `Symbol` に暗黙渡し | `E-TYPE` | language-spec §7 |
| T-06 | type 異常 | 相互再帰（非減少エッジあり） | `E-TOTAL` + `reason=non_decreasing_argument` | language-spec §0/§7 |
| T-07 | type 正常 | `let` alias 経由の strict subterm 再帰 | 成功 | language-spec §7 |
| T-08 | type 正常 | ネスト `match` 下での strict subterm 再帰 | 成功 | language-spec §7 |
| T-09 | type 正常 | 複数 ADT 引数のうち 1 つが減少する再帰 | 成功 | language-spec §7 |
| T-10 | type 異常 | 複数 ADT 引数で減少なし | `E-TOTAL` + `arg_indices` | language-spec §2.1/§7 |
| L-01 | logic 正常 | ADT 項を含む rule 導出 | 導出成功 | language-spec §6 |
| PR-01 | prover 正常 | universe 完備 + 真の assert | 全義務 `proved` | language-spec §7 |
| PR-02 | prover 異常 | assert 失敗 | `result=failed` + 反例 | language-spec §7 |
| PR-03 | prover 異常 | universe 欠落 | `E-PROVE` | language-spec §7 |
| S-01 | semantics differential | stratified negation fixture | production/reference の導出 fact 集合が一致 | semantics-core v0.6 |
| S-02 | semantics differential | ADT fact/rule fixture | production/reference の導出 fact 集合が一致 | semantics-core v0.6 |
| S-03 | semantics differential | supported fragment の generated `prove` 入力 | obligation の `id/kind/result/valuation/missing_goals` が一致 | semantics-core v0.6 |
| S-04 | semantics differential | `assert` 失敗 fixture | production/reference とも同じ反例 valuation / missing_goals | semantics-core v0.6 |
| S-05 | semantics differential | recursive `defn Refine` fixture | `check_program` の semantic fallback 後に production/reference の obligation 結果が一致 | semantics-core v0.6 |
| M-01 | metamorphic | fact 順序入替 | `solve_facts` / `prove` 結果不変 | semantics-core v0.6 |
| M-02 | metamorphic | rule 順序入替 | `solve_facts` / `prove` 結果不変 | semantics-core v0.6 |
| M-03 | metamorphic | alias 版 / canonical 版 | `prove` 結果不変 | semantics-core v0.6 |
| M-04 | metamorphic | universe 値順序入替 | `prove` 結果不変 | semantics-core v0.6 |
| M-05 | metamorphic | alpha-renaming | `prove` 結果不変 | semantics-core v0.6 |
| M-06 | metamorphic | `fmt` 前後 | `prove` 結果不変 | semantics-core v0.6 |
| M-07 | metamorphic | import 分割版 / 単一版 | CLI `prove --format json` の `proof` が一致 | semantics-core v0.6 |
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
| C-20 | CLI 正常 | `fmt` + selfdoc form | exit 0 + selfdoc form 保持整形 | language-spec §2/§9 |
| C-21 | CLI 異常 | `selfdoc` 設定ファイル欠如 | exit 2 + テンプレ出力 | language-spec §2 |
| C-22 | CLI 正常 | `selfdoc --format json` | `selfdoc.generated.dtl` + `spec.json` + `proof-trace.json` + `doc-index.json` | language-spec §2/§8 |
| C-23 | CLI 異常 | `selfdoc` 参照欠落 | `E-SELFDOC-REF` fail-fast | language-spec §9 |
| C-24 | CLI 異常 | `selfdoc` 構造化契約テーブル欠如 + `dtl <subcommand>` 文字列 | `E-SELFDOC-CONTRACT` | language-spec §2/§9 |
| C-25 | CLI 正常 | `selfcheck --format json`（coverage 完備） | exit 0 + `status=ok` + `claim_coverage=100%` | language-spec §2/§8 |
| C-26 | CLI 異常 | `selfcheck`（coverage 不足） | `E-SELFCHECK` + `status=error` | language-spec §2/§9 |
| C-27 | CLI 異常 | `selfcheck`（証明義務失敗） | `status=error` + failed obligation | language-spec §2/§7/§8 |
| C-28 | CLI 正常 | `prove --engine reference`（supported fragment） | native と同じ義務結果を返し、`proof.engine=reference` を持つ | language-spec §2/§7 |
| C-29 | CLI 正常 | `prove --engine reference`（function 型量化あり） | `status=ok` | language-spec §2/§7 |
| C-30 | CLI 正常 | `doc --engine reference`（function 型量化あり） | `spec.json` / `proof-trace.json` / `doc-index.json` を生成 | language-spec §2/§8 |
| C-31 | CLI 正常 | `selfdoc --engine reference` | `selfdoc.generated.dtl` + `proof-trace.json` を生成し、`proof.engine=reference` を持つ | language-spec §2/§8 |
| C-32 | CLI 正常 | `selfcheck --engine reference` | `status=ok` かつ `proof.engine=reference` | language-spec §2/§8 |
