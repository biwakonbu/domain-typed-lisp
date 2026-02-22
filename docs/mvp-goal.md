# MVP ゴール

## 対象
- ドメイン: アクセス制御 DSL
- 入力: 1 つ以上の S 式ファイル
- 出力: 人間可読または JSON の診断メッセージ

## ゴール
1. `dtl check <FILE>...` で以下を検査する。
   - 構文整合性
   - 名前解決（sort/relation/関数/変数）
   - 層化否定の妥当性
   - 型整合性（関数境界注釈必須 + let 推論）
   - Refinement 含意判定（CWA、半ナイーブ固定点）
2. 終了コードを固定する。
   - 成功: `0`
   - 失敗: `1`
3. CI とローカルの品質ゲートを一致させる。

## 完了条件
- テスト（unit/integration/property）が全通。
- `cargo fmt --check` / `cargo clippy -D warnings` が通過。
- `cargo llvm-cov` で行カバレッジ 80% 以上。

## 制約
- import は相対パスの最小対応（名前空間なし）
- 実行器なし
- SMT なし
- 述語論理は Datalog Horn 節 + 層化否定に限定
