# バージョニング方針（v0.6）

## 1. 目的

`dtl` には次の 2 つのバージョン軸があります。

- 言語仕様バージョン（`docs/language-spec.md` の `v0.x`）
- crate バージョン（`Cargo.toml` の SemVer）

この文書は、両者の対応と更新規則を固定します。

## 2. 対応表

| 言語仕様 | crate SemVer | 互換性の前提 |
| --- | --- | --- |
| v0.5 | 0.1.x | 既存の v0.5 契約（CLI/JSON/文法） |
| v0.6 | 0.2.x | alias + 相互再帰判定強化 + selfdoc fmt 対応 |

## 3. 更新規則

### 3.1 言語仕様 `v0.x` を上げる条件

次のいずれかを満たす場合は `v0.(x+1)` へ更新する。

1. 文法互換を壊す変更（既存入力が parse/resolve/typecheck で失敗する）
2. 診断契約の破壊変更（`code/reason/arg_indices` の意味変更を含む）
3. CLI 契約の破壊変更（サブコマンド引数、終了コード意味の変更）
4. JSON 契約の破壊変更（必須フィールド削除、型変更）

### 3.2 crate SemVer を上げる条件

- `0.(y+1).0`（minor）
  - 上記 3.1 を伴うリリース
  - 互換影響が大きい実装変更（public API 追加/変更を含む）
- `0.y.(z+1)`（patch）
  - バグ修正、ドキュメント修正、CI/配布修正のみ
  - 既存の言語仕様/CLI/JSON 契約を壊さない変更

## 4. リリース判定手順

1. 変更差分が 3.1 に該当するかを判定する。
2. 該当する場合は言語仕様バージョンを更新する。
3. 言語仕様更新がある場合、crate は minor を上げる。
4. タグは crate バージョンに一致させる（例: `v0.2.0`）。
5. `editors/vscode-dtl/package.json.version` もタグと一致させる。

## 5. 運用上の注意

- 互換性に迷う変更は「破壊的」とみなし、言語仕様 + crate minor を上げる。
- リリース PR には必ず以下を含める。
  1. `docs/language-spec.md` のバージョン表記
  2. `Cargo.toml` の `version`
  3. `editors/vscode-dtl/package.json` の `version`
  4. 互換性判断の根拠（破壊/非破壊）
