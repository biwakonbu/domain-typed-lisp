# 利用例カタログ

<!-- このファイルは scripts/generate-examples-catalog.sh と examples/catalog.tsv から自動生成されます。 -->

`examples/` に同梱しているサンプルを、用途別に引けるように整理した一覧です。

## 最初に触る

| ファイル | 主用途 | 実行例 |
| --- | --- | --- |
| `examples/access_control_ok.dtl` | 基本的なアクセス制御（単一ファイル） | `cargo run -- prove examples/access_control_ok.dtl --format json --out out_access` |
| `examples/customer_contract_ja.dtl` | 日本語識別子を含む実運用寄りサンプル | `cargo run -- doc examples/customer_contract_ja.dtl --out out_ja_doc --format markdown` |
| `examples/my_first_policy.dtl` | 最小構成（sort/relation/fact/defn/assert/universe） | `cargo run -- check examples/my_first_policy.dtl --format json` |

## 重複判定（lint/semantic-dup）

| ファイル | 主用途 | 実行例 |
| --- | --- | --- |
| `examples/semantic_dup_advanced.dtl` | rule/assert/defn 横断の L-DUP-MAYBE 検証 | `cargo run -- lint examples/semantic_dup_advanced.dtl --format json --semantic-dup` |
| `examples/semantic_dup_function_param.dtl` | function 型引数を含む defn 同値比較 | `cargo run -- lint examples/semantic_dup_function_param.dtl --format json --semantic-dup` |

## 複数ファイル運用（import）

| ファイル | 主用途 | 実行例 |
| --- | --- | --- |
| `examples/access_control_import_entry.dtl` | import 経由エントリの基本形 | `cargo run -- check examples/access_control_import_entry.dtl --format json` |
| `examples/access_control_split_policy.dtl` | 分割ポリシー定義（entry から参照） | `cargo run -- check examples/access_control_import_entry.dtl --format json` |
| `examples/access_control_split_schema.dtl` | 分割スキーマ定義（entry から参照） | `cargo run -- check examples/access_control_import_entry.dtl --format json` |
| `examples/complex_policy_import_entry.dtl` | 複雑シナリオのエントリ | `cargo run -- prove examples/complex_policy_import_entry.dtl --format json --out out_complex` |
| `examples/complex_policy_rules.dtl` | 複雑シナリオの分割規則（entry から参照） | `cargo run -- prove examples/complex_policy_import_entry.dtl --format json --out out_complex` |
| `examples/complex_policy_schema.dtl` | 複雑シナリオの分割スキーマ（entry から参照） | `cargo run -- prove examples/complex_policy_import_entry.dtl --format json --out out_complex` |

## 停止性・再帰

| ファイル | 主用途 | 実行例 |
| --- | --- | --- |
| `examples/recursive_nested_ok.dtl` | ネスト match + let alias の構造減少 | `cargo run -- prove examples/recursive_nested_ok.dtl --format json --out out_recursive` |
| `examples/recursive_totality_ok.dtl` | 構造再帰の受理ケース | `cargo run -- check examples/recursive_totality_ok.dtl --format json` |

## エラー再現（診断確認）

| ファイル | 主用途 | 実行例 |
| --- | --- | --- |
| `examples/access_control_ng_unknown_relation.dtl` | E-RESOLVE 系の失敗確認 | `cargo run -- check examples/access_control_ng_unknown_relation.dtl --format json` |

## 使い分け指針

1. CLI 動作確認だけなら `my_first_policy` か `access_control_ok`。
2. CI 連携の JSON 契約確認なら `customer_contract_ja` か `complex_policy_import_entry`。
3. lint の厳密重複判定を試すなら `semantic_dup_advanced` と `semantic_dup_function_param`。
