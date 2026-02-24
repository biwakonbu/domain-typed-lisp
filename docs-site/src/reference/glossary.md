# 用語集

<!-- このファイルは scripts/generate-glossary-assets.py と docs-site/src/reference/glossary-terms.json から自動生成されます。 -->

専門語の短義と定義を、本文中のツールチップと同一ソースで管理しています。

## 対象ページ

- `/reference/language-guide.html`
- `/reference/language-spec.html`
- `/reference/troubleshooting.html`
- `/tutorial/quickstart.html`
- `/tutorial/first-policy.html`

## 一致方式

- `token`: トークン境界で一致（前後が英数字/`_`/`-` でない場合のみ）
- `exact`: 完全一致

## DSL キーワード

<a id="term-assert"></a>
### `assert`

- 短義: 常に成り立つべきグローバル制約を宣言。
- 定義: 証明フェーズで義務化される論理条件を定義するトップレベルフォーム。
- 別名: なし
- 適用ページ: `/reference/language-guide.html`, `/reference/language-spec.html`, `/reference/troubleshooting.html`, `/tutorial/quickstart.html`, `/tutorial/first-policy.html`
- 一致方式: `token`

<a id="term-data"></a>
### `data`

- 短義: constructor 群で閉集合の語彙を定義する ADT 宣言。
- 定義: 許可する値集合を constructor で固定し、語彙統制を行うトップレベル宣言。
- 別名: なし
- 適用ページ: `/reference/language-guide.html`, `/reference/language-spec.html`, `/reference/troubleshooting.html`, `/tutorial/first-policy.html`
- 一致方式: `token`

<a id="term-defn"></a>
### `defn`

- 短義: 型付き純粋関数を定義するフォーム。
- 定義: 引数型・戻り型を明示し、構造再帰制約下で純粋関数を定義するトップレベルフォーム。
- 別名: なし
- 適用ページ: `/reference/language-guide.html`, `/reference/language-spec.html`, `/reference/troubleshooting.html`, `/tutorial/quickstart.html`, `/tutorial/first-policy.html`
- 一致方式: `token`

<a id="term-fact"></a>
### `fact`

- 短義: relation に対する具体的事実を与える宣言。
- 定義: 述語が成り立つ具体値の組を追加し、推論の基底を提供するトップレベルフォーム。
- 別名: なし
- 適用ページ: `/reference/language-guide.html`, `/reference/language-spec.html`, `/reference/troubleshooting.html`, `/tutorial/first-policy.html`
- 一致方式: `token`

<a id="term-match"></a>
### `match`

- 短義: ADT/Bool を分岐分解する式。
- 定義: constructor や真偽値ごとの分岐を記述し、網羅性・到達不能性の検査対象になる式。
- 別名: なし
- 適用ページ: `/reference/language-guide.html`, `/reference/language-spec.html`, `/reference/troubleshooting.html`
- 一致方式: `token`

<a id="term-relation"></a>
### `relation`

- 短義: 述語シグネチャを宣言する論理知識の入口。
- 定義: 事実や規則で利用する述語名と引数型を宣言するトップレベルフォーム。
- 別名: なし
- 適用ページ: `/reference/language-guide.html`, `/reference/language-spec.html`, `/reference/troubleshooting.html`, `/tutorial/first-policy.html`
- 一致方式: `token`

<a id="term-rule"></a>
### `rule`

- 短義: relation の導出条件を記述する推論規則。
- 定義: ヘッド述語がボディ条件から導かれることを定義する論理ルール。
- 別名: なし
- 適用ページ: `/reference/language-guide.html`, `/reference/language-spec.html`, `/reference/troubleshooting.html`, `/tutorial/quickstart.html`, `/tutorial/first-policy.html`
- 一致方式: `token`

<a id="term-sort"></a>
### `sort`

- 短義: 開集合のドメイン軸を宣言する型フォーム。
- 定義: 値集合を列挙せず、ドメイン軸の型名だけを定義するトップレベル宣言。
- 別名: なし
- 適用ページ: `/reference/language-guide.html`, `/reference/language-spec.html`, `/reference/troubleshooting.html`, `/tutorial/first-policy.html`
- 一致方式: `token`

<a id="term-universe"></a>
### `universe`

- 短義: 有限モデル検証の値境界を与える宣言。
- 定義: 量化対象型ごとに有限値集合を与え、prove の全探索空間を定義するフォーム。
- 別名: なし
- 適用ページ: `/reference/language-guide.html`, `/reference/language-spec.html`, `/reference/troubleshooting.html`, `/tutorial/first-policy.html`
- 一致方式: `token`

## CLI サブコマンド

<a id="term-check"></a>
### `check`

- 短義: 構文・名前解決・型などの静的検査コマンド。
- 定義: プログラムを実行せず、構文/名前解決/型/全域性/match 検査を行う CLI サブコマンド。
- 別名: なし
- 適用ページ: `/reference/language-guide.html`, `/reference/language-spec.html`, `/reference/troubleshooting.html`, `/tutorial/quickstart.html`, `/tutorial/first-policy.html`
- 一致方式: `token`

<a id="term-doc"></a>
### `doc`

- 短義: 証明成功時のみ仕様成果物を出力するコマンド。
- 定義: 未証明義務がない場合に限り、spec/proof-trace/doc-index を生成する CLI サブコマンド。
- 別名: なし
- 適用ページ: `/reference/language-guide.html`, `/reference/language-spec.html`, `/reference/troubleshooting.html`, `/tutorial/quickstart.html`
- 一致方式: `token`

<a id="term-fmt"></a>
### `fmt`

- 短義: Surface 形式へ正規化整形するコマンド。
- 定義: AST 正規化に基づき idempotent な整形を行う CLI サブコマンド。
- 別名: なし
- 適用ページ: `/reference/language-guide.html`, `/reference/troubleshooting.html`, `/tutorial/quickstart.html`
- 一致方式: `token`

<a id="term-lint"></a>
### `lint`

- 短義: 重複候補と未使用宣言を検出するコマンド。
- 定義: L-DUP 系と L-UNUSED-DECL を warning として報告する CLI サブコマンド。
- 別名: なし
- 適用ページ: `/reference/language-guide.html`, `/reference/language-spec.html`, `/reference/troubleshooting.html`, `/tutorial/quickstart.html`, `/tutorial/first-policy.html`
- 一致方式: `token`

<a id="term-prove"></a>
### `prove`

- 短義: 有限モデル上で証明義務を検証するコマンド。
- 定義: assert と Refine 契約を universe に基づいて全探索検証する CLI サブコマンド。
- 別名: なし
- 適用ページ: `/reference/language-guide.html`, `/reference/language-spec.html`, `/reference/troubleshooting.html`, `/tutorial/quickstart.html`, `/tutorial/first-policy.html`
- 一致方式: `token`

## 型システム

<a id="term-refine"></a>
### `Refine`

- 短義: 論理式で値を制約する精密化型。
- 定義: base 型の値に対して論理述語を課す型コンストラクタで、証明義務生成に関与する。
- 別名: なし
- 適用ページ: `/reference/language-guide.html`, `/reference/language-spec.html`
- 一致方式: `token`

## 意味論・検証概念

<a id="term-semantic-dup"></a>
### `semantic-dup`

- 短義: 有限モデル上の意味同値で重複候補を検出するモード。
- 定義: 構文一致ではなく評価結果の同値性を使って重複候補を検出する lint オプション。
- 別名: なし
- 適用ページ: `/reference/language-guide.html`, `/reference/language-spec.html`, `/reference/troubleshooting.html`, `/tutorial/quickstart.html`, `/tutorial/first-policy.html`
- 一致方式: `token`

<a id="term-strict-subterm"></a>
### `strict subterm`

- 短義: 再帰呼び出しで元引数より真に小さい部分値。
- 定義: match 分解で得た子要素など、構造的に減少していることを示す ADT 部分項。
- 別名: なし
- 適用ページ: `/reference/language-guide.html`, `/reference/language-spec.html`
- 一致方式: `exact`

<a id="term-tail-position"></a>
### `tail position`

- 短義: 式評価の最終位置で結果をそのまま返す場所。
- 定義: 追加計算を伴わず、呼び出し結果がそのまま関数結果になる式位置。
- 別名: なし
- 適用ページ: `/reference/language-guide.html`, `/reference/language-spec.html`, `/reference/troubleshooting.html`
- 一致方式: `exact`

## 診断コード

<a id="term-e-match"></a>
### `E-MATCH`

- 短義: match 検査違反を示す診断コード。
- 定義: 分岐非網羅、到達不能分岐、パターン型不一致を示すエラーコード。
- 別名: なし
- 適用ページ: `/reference/language-guide.html`, `/reference/language-spec.html`, `/reference/troubleshooting.html`
- 一致方式: `token`

<a id="term-e-parse"></a>
### `E-PARSE`

- 短義: 構文不正を示す診断コード。
- 定義: S 式の括弧不整合やフォーム構造崩れなど、構文解析段階の失敗を示すエラーコード。
- 別名: なし
- 適用ページ: `/reference/language-guide.html`, `/reference/language-spec.html`, `/reference/troubleshooting.html`
- 一致方式: `token`

<a id="term-e-prove"></a>
### `E-PROVE`

- 短義: 証明失敗または universe 不備を示す診断コード。
- 定義: 有限モデル検証での反例検出や必須 universe 欠落による失敗を示すエラーコード。
- 別名: なし
- 適用ページ: `/reference/language-spec.html`, `/reference/troubleshooting.html`
- 一致方式: `token`

<a id="term-e-resolve"></a>
### `E-RESOLVE`

- 短義: 名前解決失敗を示す診断コード。
- 定義: 未定義識別子、重複定義、unsafe rule などの名前解決違反を示すエラーコード。
- 別名: なし
- 適用ページ: `/reference/language-guide.html`, `/reference/language-spec.html`, `/reference/troubleshooting.html`, `/tutorial/first-policy.html`
- 一致方式: `token`

<a id="term-e-syntax-auto"></a>
### `E-SYNTAX-AUTO`

- 短義: Core/Surface 自動判定衝突を示す診断コード。
- 定義: syntax:auto で同一ファイルに Core と Surface が混在したときに返るエラーコード。
- 別名: なし
- 適用ページ: `/reference/language-guide.html`, `/reference/language-spec.html`, `/reference/troubleshooting.html`
- 一致方式: `token`

<a id="term-e-total"></a>
### `E-TOTAL`

- 短義: 停止性/全域性違反を示す診断コード。
- 定義: 非 tail 再帰、非減少再帰、相互再帰など全域性条件違反を示すエラーコード。
- 別名: なし
- 適用ページ: `/reference/language-guide.html`, `/reference/language-spec.html`, `/reference/troubleshooting.html`
- 一致方式: `token`

<a id="term-e-type"></a>
### `E-TYPE`

- 短義: 型不一致を示す診断コード。
- 定義: 関数引数/戻り値、relation 引数、条件式などの型整合性違反を示すエラーコード。
- 別名: なし
- 適用ページ: `/reference/language-guide.html`, `/reference/language-spec.html`, `/reference/troubleshooting.html`, `/tutorial/first-policy.html`
- 一致方式: `token`

<a id="term-l-dup-exact"></a>
### `L-DUP-EXACT`

- 短義: 構文正規化後に確定した重複警告。
- 定義: 正規化済みフォームが同一であることを根拠に重複と判定された lint コード。
- 別名: なし
- 適用ページ: `/reference/language-guide.html`, `/reference/language-spec.html`
- 一致方式: `token`

<a id="term-l-dup-maybe"></a>
### `L-DUP-MAYBE`

- 短義: 意味同値にもとづく重複候補警告。
- 定義: 有限モデルでの双方向検証結果から、重複の可能性を示す lint コード。
- 別名: なし
- 適用ページ: `/reference/language-guide.html`, `/reference/language-spec.html`, `/reference/troubleshooting.html`, `/tutorial/quickstart.html`
- 一致方式: `token`

<a id="term-l-dup-skip-eval-depth"></a>
### `L-DUP-SKIP-EVAL-DEPTH`

- 短義: 評価深さ上限到達で入力点を省略した警告。
- 定義: defn 比較時に再帰評価が深さ上限へ達し、判定対象の一部をスキップしたことを示す lint コード。
- 別名: なし
- 適用ページ: `/reference/language-guide.html`, `/reference/language-spec.html`, `/reference/troubleshooting.html`
- 一致方式: `token`

<a id="term-l-dup-skip-universe"></a>
### `L-DUP-SKIP-UNIVERSE`

- 短義: universe 不足で semantic-dup 判定を省略した警告。
- 定義: 必要な universe が不足し、重複候補の意味検証を実施できなかったことを示す lint コード。
- 別名: なし
- 適用ページ: `/reference/language-guide.html`, `/reference/language-spec.html`, `/reference/troubleshooting.html`, `/tutorial/first-policy.html`
- 一致方式: `token`

<a id="term-l-unused-decl"></a>
### `L-UNUSED-DECL`

- 短義: 未参照宣言を示す警告コード。
- 定義: relation/sort/data などの宣言が参照されていないことを示す lint コード。
- 別名: なし
- 適用ページ: `/reference/language-guide.html`, `/reference/language-spec.html`
- 一致方式: `token`
