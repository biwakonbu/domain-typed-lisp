# ADT Parametric 化評価（v0.3）

## 1. 問題設定
- 現在の `data` は単相（monomorphic）であり、`ListSubject` / `ListResource` のような型ごとの重複定義が必要になる。
- 将来的に `Option<T>` / `List<T>` 相当を DSL で表現したい要求が想定される。

## 2. 評価軸
- 言語複雑度: 文法・型システム・診断の増分
- 検証コスト: `check` と `prove` の計算量影響
- 移行容易性: 既存 v0.2 スクリプト互換性
- 実装工数: parser / resolve / typecheck / prover 変更範囲

## 3. 選択肢

### Option A: v0.2 維持（単相のみ）
- 長所:
  - 既存コード変更が最小。
  - 証明探索の挙動が読みやすい。
- 短所:
  - ADT 定義重複が増える。
  - 共通ライブラリ化しにくい。

### Option B: `data` のみ parametric 化（関数は現行のまま）
- 例:
  - `(data (List T) (nil) (cons T (List T)))`
- 長所:
  - 再利用性向上の効果が大きい。
  - HM 型推論まで導入せずに済む。
- 短所:
  - 型適用（`(List Subject)`）の解決ルール追加が必要。
  - universe 宣言の扱い（型引数具体化単位）が複雑化。

### Option C: 関数多相まで含む全面導入
- 長所:
  - 表現力最大。
- 短所:
  - 型推論・単相化戦略・証明器連携が大幅に複雑化。
  - v0.3 スコープを超える。

## 4. 影響分析
- `parser`
  - 型式位置で `TypeApply` を解釈する必要がある。
- `name_resolve`
  - 型コンストラクタ解決と型引数 arity 検査が必要。
- `typecheck`
  - ADT constructor の型インスタンス化が必要。
  - エラーメッセージに「型引数不一致」を追加。
- `prover`
  - 実行時は単相化済み型のみ扱う設計にすれば、コアロジックの変更を抑制可能。

## 5. 判断
- v0.3 は Option A を維持し、parametric ADT は見送る。
- 理由:
  - 停止性解析導入（別 P2）と同時に入れると検証面の変更点が過多。
  - `prove` の universe 仕様を先に安定化したい。

## 6. 次期方針（v0.4 準備）
- Option B（`data` parametric 化のみ）を前提に設計を先行する。
- 先行タスク:
  - 型適用構文の最小仕様を `language-spec` 草案化。
  - universe と parametric ADT の整合規則を ADR 化。
  - 単相化（monomorphization）有無の比較ベンチを作成。

## 7. 結論
- v0.3 では「単相 ADT 維持」が妥当。
- ただし中期的に Option B は価値が高く、v0.4 で再評価する。
