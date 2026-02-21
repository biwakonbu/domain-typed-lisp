# ADR 0001: MVP アーキテクチャ

## Status
Accepted

## Context
- 目的は静的型解析向け DSL の MVP を短期間で成立させること。
- 必要要件は Datalog 系述語推論 + Refinement 型 + CLI 検証。

## Decision
- 実装言語: Rust
- 主要構成:
  - `parser`: S 式パーサ
  - `name_resolve`: 宣言/参照整合
  - `stratify`: 否定層化検査
  - `logic_engine`: 固定点導出
  - `typecheck`: 双方向型検査（境界注釈 + let 推論）
  - `diagnostics`: 統一診断
- 依存 crate:
  - `chumsky`（パース補助）
  - `ariadne`（診断整形）
  - `datafrog`（Datalog 評価補助）
  - `proptest`（property test）

## Consequences
- 利点: MVP の実装速度と検証容易性を確保。
- 欠点: フル Prolog 相当の表現力は対象外。

## Future
- import 対応
- SMT 連携
- 実行器導入
