# TODO (v0.2)

## P0
- [ ] `check/prove/doc` を CI 必須ジョブに反映する。
- [ ] `prove` の JSON 契約をバージョン付きで固定し、互換テストを追加する。
- [ ] `doc --format json` の成果物仕様を実装する（現状は `spec.md` 固定）。

## P1
- [ ] `prove` の義務抽出を強化する（`if` / `match` を含む `defn` 本体の論理化）。
- [ ] universe 未宣言型に対する診断を source/span 付きで改善する。
- [ ] 反例最小化の性能計測を追加する（大きい universe での探索コスト評価）。

## P2
- [ ] v0.3 に向けた停止性解析の設計（再帰禁止からの段階移行）を策定する。
- [ ] ADT の parametric 化要否を評価する。
- [ ] import 名前空間設計（公開制御・再エクスポート）の ADR を作成する。

## Technical Debt
- [ ] 複数ファイル入力時、resolve/typecheck 診断の `source` 付与を厳密化する（Span にファイル識別子導入）。
- [ ] ベンチ自動実行を CI に組み込む。
