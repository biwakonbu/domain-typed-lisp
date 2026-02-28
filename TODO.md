# TODO (v0.4)

最終更新: 2026-02-27

残件: 0

## P0（v0.4 核）
- [x] `lint` サブコマンドを追加し、`L-DUP-EXACT` / `L-DUP-MAYBE` / `L-DUP-SKIP-UNIVERSE` / `L-DUP-SKIP-EVAL-DEPTH` / `L-UNUSED-DECL` を実装する。
- [x] `fmt` サブコマンドを追加し、`--check` / `--stdout` を含む整形契約を実装する。
- [x] `doc --pdf` を追加し、Pandoc 失敗時に Markdown 成果物を維持して warning 化する。
- [x] Surface 構文（`syntax: surface` + タグ付きS式 + 日英キーワードエイリアス）を導入し、Core AST へデシュガする。
- [x] `spec.md` を日本語自然文 + Mermaid 3点セット（型/依存/証明要約）へ更新する。
- [x] `lint/fmt/doc-pdf` の統合テストを追加する。

## P1（厳密化・運用）
- [x] `L-DUP-MAYBE` を近似スケルトン判定から、有限モデルでの双方向検証（`rule/assert` 含意・`defn` 戻り一致）へ厳密化する。
- [x] `fmt` の `@context` 単位ブロック保持を強化し、複数コンテキストでの安定整形（idempotent）テストを追加する。
- [x] `docs/migration-v0.2.md` と `docs/troubleshooting-errors-ja.md` に v0.4（`lint/fmt/surface/doc --pdf`）の移行・障害対応を追記する。
- [x] CI に `dtl lint --deny-warnings` と `dtl fmt --check` を専用ジョブとして追加する。

## P2（将来）
- [x] `L-DUP-MAYBE` の `confidence` 算出を、モデルカバレッジと反例探索結果に基づく指標へ更新する。
- [x] Surface 構文の `syntax: auto` 判定衝突ケース（同一ファイル内混在）の診断を改善する（専用コード化）。

## P3（現時点の課題）
- [x] `cargo clippy --workspace --all-targets --all-features -- -D warnings` を通す（`collapsible_if`/`manual_is_multiple_of` 対応 + `result_large_err` 方針を crate 属性で明示）。
- [x] `--semantic-dup` で function 型パラメータを含む `defn` の同値比較を可能にする（有限関数モデル列挙 + `defn` 評価器の function 値対応）。
- [x] `--semantic-dup` の `defn` 同値評価で深い再帰を安全に扱えるようにする（深さ上限を適応化し、`L-DUP-SKIP-EVAL-DEPTH` で可視化）。
- [x] `selfdoc` の参照欠落 fail-fast 契約で `E-SELFDOC-REF` と欠落参照先の stderr 表示を統合テストで固定する。

## P4（v0.6 候補）
- [x] 相互再帰を条件付きで許可する停止性検査を導入する（SCC 単位の構造減少条件を仕様化）。
- [x] 同義語 alias 機能を導入する（constructor 正規名ポリシーとの整合を定義）。
- [x] `fmt` で selfdoc form を保持した再整形をサポートする（現状 `E-FMT-SELFDOC-UNSUPPORTED`）。
- [x] 言語仕様バージョン（v0.x）と crate バージョン（SemVer）の対応方針を明文化する。
- [x] VS Code Marketplace / Open VSX 公開フローを整備する（CI・配布手順・README 反映）。

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

## Archive: v0.3（完了済み）
- [x] `E-TOTAL` を「全再帰禁止」から「相互再帰禁止 + 自己再帰の構造減少判定」へ移行する。
- [x] 構造再帰の受理/拒否を固定する単体テストを追加する（tail 再帰正例、非 tail/非減少/相互再帰の負例）。
- [x] `E-TOTAL` 診断を機械可読化する（`reason` / `arg_indices` など）。
- [x] `docs/language-spec.md` と `docs/migration-v0.2.md` を v0.3 挙動へ同期する。
- [x] `prove` / `doc` まで含む再帰関数サンプルの E2E テストを追加する。
- [x] 構造減少判定の境界ケース（`let` alias / ネスト `match` / 複数 ADT 引数）をテストマトリクス化する。
