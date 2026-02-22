# ADR 0001: import 名前空間と公開制御

- Status: Accepted (design baseline for v0.3+)
- Date: 2026-02-22

## Context
- v0.2 の `import` は `(import "path.dtl")` のみを提供し、読み込んだ宣言は単一グローバル名前空間に平坦化される。
- この方式は小規模では単純だが、以下の問題がある。
  - 同名衝突を回避しづらい。
  - どの宣言を外部公開するかを制御できない。
  - ライブラリ化時に再エクスポート戦略を定義できない。

## Decision
次のモジュール境界ルールを採用する。

1. `import` は名前空間 alias を必須化する。
   - 目標構文: `(import "policy/base.dtl" as base)`
2. 参照は原則 `namespace/name` 形式に限定する。
   - 例: `base/can-access`
3. 各ファイルは公開シンボルを明示する。
   - 目標構文: `(export can-access deny-by-default)`
4. 再エクスポートは明示のみ許可する。
   - 目標構文: `(re-export base can-access)`
5. wildcard export/import は導入しない。

## Rationale
- 名前衝突耐性を高めるには、グローバル暗黙公開より明示的公開が適切。
- `namespace/name` で依存元を静的に追跡でき、診断メッセージの可観測性が上がる。
- wildcard を禁止することで、互換性破壊（追加公開による意図しない解決）を防げる。

## Consequences

### Positive
- 依存境界が明確になる。
- 大規模化時の宣言衝突を抑制できる。
- 将来のパッケージ管理導入時に拡張しやすい。

### Negative
- 既存 DSL との互換性を壊す可能性がある。
- parser / name_resolve / diagnostics / docs の変更範囲が広い。
- エントリファイルに alias 記述が増える。

## Compatibility Plan
1. v0.3:
   - 現行 `(import "path")` を維持しつつ、非推奨警告を追加しない（互換優先）。
   - 仕様書に「将来導入予定」として先行記載。
2. v0.4:
   - alias 付き `import` と `export` / `re-export` を導入。
   - 旧構文は段階的廃止（1 minor version 併存）。

## Alternatives Considered
- A: 現行の平坦 import を維持する  
  - 却下理由: スケール時の衝突問題が解消しない。
- B: wildcard ベースの公開制御  
  - 却下理由: 依存境界が不透明になり、変更耐性が下がる。

## Follow-up Tasks
- `language-spec` に module 構文章を追加（草案）。
- `name_resolve` に namespace-aware 解決器を追加。
- `E-RESOLVE` 診断に `namespace` / `symbol` の分離フィールドを追加。
