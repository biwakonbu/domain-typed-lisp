# domain-typed-lisp ドキュメント

このサイトは `dtl` の利用者向けに、実務で必要な情報を 3 層で整理しています。

- チュートリアル: 最短で動かし、成果物を確認する手順
- リファレンス: CLI・Lint/エラーコード・言語仕様の一次情報
- 運用: HTML 生成と GitHub Pages 公開

## 最短導線

1. [クイックスタート](./tutorial/quickstart.md) で `check -> prove -> doc -> lint -> fmt` を通す
2. [利用例カタログ](./tutorial/examples-catalog.md) から用途に合うサンプルを選ぶ
3. [CLI リファレンス](./reference/cli.md) でオプション契約を確認する
4. [JSON 契約リファレンス](./reference/json-contracts.md) で CI 連携形式を固定する
5. 詳細仕様が必要になった時点で [言語仕様（完全版）](./reference/language-spec.md) を参照する

## 想定読者

- DSL 利用者（ポリシー記述者）
- CI/品質ゲート運用担当
- 言語仕様・検証契約を確認する開発者
