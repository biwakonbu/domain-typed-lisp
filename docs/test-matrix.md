# テストマトリクス（v0.2）

| ID | 種別 | 入力概要 | 期待結果 | 関連仕様 |
|---|---|---|---|---|
| P-01 | parser 正常 | `data/assert/universe/match` を含む構成 | parse 成功 | language-spec §3/§4 |
| P-02 | parser 異常 | `data` constructor なし | `E-PARSE` | language-spec §3.3 |
| P-03 | parser 異常 | `match` arm 形状不正 | `E-PARSE` | language-spec §4 |
| R-01 | resolve 異常 | constructor 重複 | `E-DATA` | language-spec §3.3 |
| R-02 | resolve 異常 | 再帰 ADT | `E-DATA` | language-spec §3.3 |
| R-03 | resolve 異常 | 未定義 universe 型 | `E-RESOLVE` | language-spec §3.8 |
| R-04 | resolve 異常 | pattern の未知 constructor | `E-RESOLVE` | language-spec §4 |
| T-01 | type 正常 | constructor + 網羅 `match` | 成功 | language-spec §4/§7 |
| T-02 | type 異常 | 非網羅 `match` | `E-MATCH` | language-spec §7 |
| T-03 | type 異常 | 到達不能 `match` arm | `E-MATCH` | language-spec §7 |
| T-04 | type 異常 | 関数再帰 | `E-TOTAL` | language-spec §0/§7 |
| T-05 | type 異常 | `Domain` を `Symbol` に暗黙渡し | `E-TYPE` | language-spec §7 |
| L-01 | logic 正常 | ADT 項を含む rule 導出 | 導出成功 | language-spec §6 |
| PR-01 | prover 正常 | universe 完備 + 真の assert | 全義務 `proved` | language-spec §7 |
| PR-02 | prover 異常 | assert 失敗 | `result=failed` + 反例 | language-spec §7 |
| PR-03 | prover 異常 | universe 欠落 | `E-PROVE` | language-spec §7 |
| C-01 | CLI 正常 | `check` | exit 0 | language-spec §2 |
| C-02 | CLI 正常 | `prove --format json --out` | exit 0 + `proof-trace.json` | language-spec §2/§8 |
| C-03 | CLI 異常 | `prove` 失敗義務 | exit 1 + failed obligation | language-spec §2/§7 |
| C-04 | CLI 正常 | `doc` 証明成功ケース | `spec.md` / `proof-trace.json` / `doc-index.json` | language-spec §8 |
| C-05 | CLI 異常 | `doc` 未証明ケース | exit 1 + 生成抑止 | language-spec §8 |
