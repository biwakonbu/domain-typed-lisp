# 自己証明判定基準（厳密）

## 目的
- `dtl selfcheck` が「自己記述の完全性」を機械可読で判定するための基準を固定する。

## 判定条件
1. `selfdoc` 抽出が成功すること（`E-SELFDOC-*` が無い）。
2. 生成された `proof-trace.json` の全義務が `proved` であること。
3. `proof-trace.json.claim_coverage` が `proved_claims == total_claims` かつ `total_claims > 0` を満たすこと。

## claim_coverage の定義
- `total_claims`: CLI サブコマンド総数（`clap` 定義を真値とする）。
- `proved_claims`: README/language-spec の構造化 CLI 契約テーブルから抽出できたサブコマンド数。

## 構造化 CLI 契約テーブル
- マーカー:
  - `<!-- selfdoc:cli-contracts:start -->`
  - `<!-- selfdoc:cli-contracts:end -->`
- 必須列:
  - `subcommand`
  - `impl_path`

## 失敗時の扱い
- `selfcheck` は exit code 1 を返す。
- `proof-trace.json` は失敗時も出力し、原因追跡を可能にする。
