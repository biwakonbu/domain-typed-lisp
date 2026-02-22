# テストマトリクス

| ID | 種別 | 入力概要 | 期待結果 | 関連仕様 |
|---|---|---|---|---|
| P-01 | parser 正常 | sort/relation/fact/rule/defn 最小構成 | parse 成功 | language-spec §2 |
| P-02 | parser 異常 | 括弧不整合 | E-PARSE | language-spec §1 |
| R-01 | resolve 異常 | 未定義 relation | E-RESOLVE | language-spec §2 |
| R-02 | resolve 異常 | 重複定義 | E-RESOLVE | language-spec §2 |
| S-01 | stratify 正常 | 正常な否定なし規則 | 成功 | language-spec §2.4 |
| S-02 | stratify 異常 | 否定サイクル | E-STRATIFY | language-spec §2.4 |
| L-01 | logic 正常 | 固定点収束 | 期待事実導出 | language-spec §6 |
| L-02 | logic 正常 | 順序変更 | 同一導出結果 | language-spec §6 |
| L-03 | logic 正常 | CWA | 未証明述語は偽 | language-spec §6 |
| T-01 | type 正常 | 引数/戻り値整合 | 成功 | language-spec §5 |
| T-02 | type 異常 | 引数型不一致 | E-TYPE | language-spec §5 |
| T-03 | type 異常 | 含意不能 | E-ENTAIL | language-spec §6 |
| T-04 | type 正常 | 否定含意（空型） | 成功 | language-spec §6 |
| T-05 | type 正常 | 連言含意 + 規則導出 | 成功 | language-spec §6 |
| C-01 | CLI 正常 | 正常プログラム | exit 0 | mvp-goal |
| C-02 | CLI 異常 | 型エラー | exit 1 + 診断 | mvp-goal |
| C-03 | E2E | `examples/*.dtl` | JSON 出力契約を固定 | mvp-goal |
| C-04 | CLI 正常 | 複数ファイル入力 | exit 0 + 検査成功 | mvp-goal |
| C-05 | CLI 異常 | import 循環 | E-IMPORT | language-spec §2.1 |
| B-01 | bench | ルール数・事実数スケーリング | 性能傾向を計測 | verification-plan |
| PR-01 | property | 冪等性 | `solve(solve(KB)) == solve(KB)` | language-spec §6 |
| PR-02 | property | 単調性 | 事実追加で導出が減らない | language-spec §6 |
