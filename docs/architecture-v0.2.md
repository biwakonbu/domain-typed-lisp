# v0.2 アーキテクチャ

## 目的
- 純粋・非破壊な DSL としてドメイン制約を検証する。
- `check` / `prove` / `doc` を分離し、静的検査と証明トレース生成を明確化する。

## モジュール構成
- `parser`: S式から AST を構築（`data/assert/universe/match` 対応）
- `name_resolve`: 宣言解決、constructor 解決、再帰 ADT 禁止、universe 整合
- `stratify`: 層化否定検査
- `typecheck`: 型検査、再帰禁止（`E-TOTAL`）、`match` 網羅/到達不能（`E-MATCH`）
- `logic_engine`: ADT 構造項を含む固定点評価
- `prover`: 有限モデル全探索、証明義務評価、最小反例トレース生成
- `main`: CLI (`check/prove/doc`) と I/O 契約

## コア設計
- 言語内で可変状態・副作用を持たない。
- 関数再帰は v0.2 で禁止し、全域性を強制する。
- `Symbol` と `Domain` の暗黙互換は禁止する。
- 証明は `universe` 宣言の有限集合上で行う。

## 出力契約
- `prove --out DIR`: `proof-trace.json`（`schema_version = 1.0.0`）
- `doc --out DIR`: `spec.md` / `proof-trace.json` / `doc-index.json`
- 未証明義務がある場合 `doc` は失敗する。
