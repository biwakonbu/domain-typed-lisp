# TODO (v0.3)

最終更新: 2026-02-23

残件: 0

## P0
- [x] `E-TOTAL` を「全再帰禁止」から「相互再帰禁止 + 自己再帰の構造減少判定」へ移行する。
- [x] 構造再帰の受理/拒否を固定する単体テストを追加する（tail 再帰正例、非 tail/非減少/相互再帰の負例）。
- [x] `E-TOTAL` 診断を機械可読化する（`reason` / `arg_indices` など）。
- [x] `docs/language-spec.md` と `docs/migration-v0.2.md` を v0.3 挙動へ同期する。

## P1
- [x] `prove` / `doc` まで含む再帰関数サンプルの E2E テストを追加する。
- [x] 構造減少判定の境界ケース（`let` alias / ネスト `match` / 複数 ADT 引数）をテストマトリクス化する。

## Archive: v0.2（完了済み）
- [x] `check/prove/doc` を CI 必須ジョブに反映する。
- [x] `prove` の JSON 契約をバージョン付きで固定し、互換テストを追加する。
- [x] `doc --format json` の成果物仕様を実装する（現状は `spec.md` 固定）。
- [x] `prove` の義務抽出を強化する（`if` / `match` を含む `defn` 本体の論理化）。
- [x] universe 未宣言型に対する診断を source/span 付きで改善する。
- [x] 反例最小化の性能計測を追加する（大きい universe での探索コスト評価）。
- [x] `docs/language-guide-ja.md` をチュートリアル化する（`check -> prove -> doc` の一気通貫手順と成果物の読み方を追加）。
- [x] エラーコード別トラブルシュート集を作成する（`E-PARSE`/`E-RESOLVE`/`E-TYPE`/`E-MATCH`/`E-PROVE` の原因と対処を整理）。
- [x] v0.3 に向けた停止性解析の設計（再帰禁止からの段階移行）を策定する。  
  成果物: `docs/termination-analysis-v0.3.md`
- [x] ADT の parametric 化要否を評価する。  
  成果物: `docs/adt-parametric-evaluation-v0.3.md`
- [x] import 名前空間設計（公開制御・再エクスポート）の ADR を作成する。  
  成果物: `docs/adr/0001-import-namespace.md`
- [x] 複数ファイル入力時、resolve/typecheck 診断の `source` 付与を厳密化する（Span にファイル識別子導入）。
- [x] ベンチ自動実行を CI に組み込む。
- [x] 日本語サンプル `examples/customer_contract_ja.dtl` の `doc` 生成を E2E テストに追加する。
- [x] Atom 正規化の仕様境界を明確化する（引用符・エスケープを含む Atom の取り扱いを言語仕様に追記し、対応テストを追加）。
