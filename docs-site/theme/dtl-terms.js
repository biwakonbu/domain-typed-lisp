(() => {
  const payload = {
  "target_pages": [
    "/reference/language-guide.html",
    "/reference/language-spec.html",
    "/reference/troubleshooting.html",
    "/tutorial/quickstart.html",
    "/tutorial/first-policy.html"
  ],
  "terms": [
    {
      "id": "sort",
      "label": "sort",
      "aliases": [],
      "short_tip": "開集合のドメイン軸を宣言する型フォーム。",
      "definition": "値集合を列挙せず、ドメイン軸の型名だけを定義するトップレベル宣言。",
      "category": "dsl-keyword",
      "match_mode": "token",
      "enabled_pages": [
        "/reference/language-guide.html",
        "/reference/language-spec.html",
        "/reference/troubleshooting.html",
        "/tutorial/first-policy.html"
      ]
    },
    {
      "id": "data",
      "label": "data",
      "aliases": [],
      "short_tip": "constructor 群で閉集合の語彙を定義する ADT 宣言。",
      "definition": "許可する値集合を constructor で固定し、語彙統制を行うトップレベル宣言。",
      "category": "dsl-keyword",
      "match_mode": "token",
      "enabled_pages": [
        "/reference/language-guide.html",
        "/reference/language-spec.html",
        "/reference/troubleshooting.html",
        "/tutorial/first-policy.html"
      ]
    },
    {
      "id": "relation",
      "label": "relation",
      "aliases": [],
      "short_tip": "述語シグネチャを宣言する論理知識の入口。",
      "definition": "事実や規則で利用する述語名と引数型を宣言するトップレベルフォーム。",
      "category": "dsl-keyword",
      "match_mode": "token",
      "enabled_pages": [
        "/reference/language-guide.html",
        "/reference/language-spec.html",
        "/reference/troubleshooting.html",
        "/tutorial/first-policy.html"
      ]
    },
    {
      "id": "fact",
      "label": "fact",
      "aliases": [],
      "short_tip": "relation に対する具体的事実を与える宣言。",
      "definition": "述語が成り立つ具体値の組を追加し、推論の基底を提供するトップレベルフォーム。",
      "category": "dsl-keyword",
      "match_mode": "token",
      "enabled_pages": [
        "/reference/language-guide.html",
        "/reference/language-spec.html",
        "/reference/troubleshooting.html",
        "/tutorial/first-policy.html"
      ]
    },
    {
      "id": "rule",
      "label": "rule",
      "aliases": [],
      "short_tip": "relation の導出条件を記述する推論規則。",
      "definition": "ヘッド述語がボディ条件から導かれることを定義する論理ルール。",
      "category": "dsl-keyword",
      "match_mode": "token",
      "enabled_pages": [
        "/reference/language-guide.html",
        "/reference/language-spec.html",
        "/reference/troubleshooting.html",
        "/tutorial/quickstart.html",
        "/tutorial/first-policy.html"
      ]
    },
    {
      "id": "assert",
      "label": "assert",
      "aliases": [],
      "short_tip": "常に成り立つべきグローバル制約を宣言。",
      "definition": "証明フェーズで義務化される論理条件を定義するトップレベルフォーム。",
      "category": "dsl-keyword",
      "match_mode": "token",
      "enabled_pages": [
        "/reference/language-guide.html",
        "/reference/language-spec.html",
        "/reference/troubleshooting.html",
        "/tutorial/quickstart.html",
        "/tutorial/first-policy.html"
      ]
    },
    {
      "id": "universe",
      "label": "universe",
      "aliases": [],
      "short_tip": "有限モデル検証の値境界を与える宣言。",
      "definition": "量化対象型ごとに有限値集合を与え、prove の全探索空間を定義するフォーム。",
      "category": "dsl-keyword",
      "match_mode": "token",
      "enabled_pages": [
        "/reference/language-guide.html",
        "/reference/language-spec.html",
        "/reference/troubleshooting.html",
        "/tutorial/first-policy.html"
      ]
    },
    {
      "id": "defn",
      "label": "defn",
      "aliases": [],
      "short_tip": "型付き純粋関数を定義するフォーム。",
      "definition": "引数型・戻り型を明示し、構造再帰制約下で純粋関数を定義するトップレベルフォーム。",
      "category": "dsl-keyword",
      "match_mode": "token",
      "enabled_pages": [
        "/reference/language-guide.html",
        "/reference/language-spec.html",
        "/reference/troubleshooting.html",
        "/tutorial/quickstart.html",
        "/tutorial/first-policy.html"
      ]
    },
    {
      "id": "match",
      "label": "match",
      "aliases": [],
      "short_tip": "ADT/Bool を分岐分解する式。",
      "definition": "constructor や真偽値ごとの分岐を記述し、網羅性・到達不能性の検査対象になる式。",
      "category": "dsl-keyword",
      "match_mode": "token",
      "enabled_pages": [
        "/reference/language-guide.html",
        "/reference/language-spec.html",
        "/reference/troubleshooting.html"
      ]
    },
    {
      "id": "check",
      "label": "check",
      "aliases": [],
      "short_tip": "構文・名前解決・型などの静的検査コマンド。",
      "definition": "プログラムを実行せず、構文/名前解決/型/全域性/match 検査を行う CLI サブコマンド。",
      "category": "cli-subcommand",
      "match_mode": "token",
      "enabled_pages": [
        "/reference/language-guide.html",
        "/reference/language-spec.html",
        "/reference/troubleshooting.html",
        "/tutorial/quickstart.html",
        "/tutorial/first-policy.html"
      ]
    },
    {
      "id": "prove",
      "label": "prove",
      "aliases": [],
      "short_tip": "有限モデル上で証明義務を検証するコマンド。",
      "definition": "assert と Refine 契約を universe に基づいて全探索検証する CLI サブコマンド。",
      "category": "cli-subcommand",
      "match_mode": "token",
      "enabled_pages": [
        "/reference/language-guide.html",
        "/reference/language-spec.html",
        "/reference/troubleshooting.html",
        "/tutorial/quickstart.html",
        "/tutorial/first-policy.html"
      ]
    },
    {
      "id": "doc",
      "label": "doc",
      "aliases": [],
      "short_tip": "証明成功時のみ仕様成果物を出力するコマンド。",
      "definition": "未証明義務がない場合に限り、spec/proof-trace/doc-index を生成する CLI サブコマンド。",
      "category": "cli-subcommand",
      "match_mode": "token",
      "enabled_pages": [
        "/reference/language-guide.html",
        "/reference/language-spec.html",
        "/reference/troubleshooting.html",
        "/tutorial/quickstart.html"
      ]
    },
    {
      "id": "lint",
      "label": "lint",
      "aliases": [],
      "short_tip": "重複候補と未使用宣言を検出するコマンド。",
      "definition": "L-DUP 系と L-UNUSED-DECL を warning として報告する CLI サブコマンド。",
      "category": "cli-subcommand",
      "match_mode": "token",
      "enabled_pages": [
        "/reference/language-guide.html",
        "/reference/language-spec.html",
        "/reference/troubleshooting.html",
        "/tutorial/quickstart.html",
        "/tutorial/first-policy.html"
      ]
    },
    {
      "id": "fmt",
      "label": "fmt",
      "aliases": [],
      "short_tip": "Surface 形式へ正規化整形するコマンド。",
      "definition": "AST 正規化に基づき idempotent な整形を行う CLI サブコマンド。",
      "category": "cli-subcommand",
      "match_mode": "token",
      "enabled_pages": [
        "/reference/language-guide.html",
        "/reference/troubleshooting.html",
        "/tutorial/quickstart.html"
      ]
    },
    {
      "id": "refine",
      "label": "Refine",
      "aliases": [],
      "short_tip": "論理式で値を制約する精密化型。",
      "definition": "base 型の値に対して論理述語を課す型コンストラクタで、証明義務生成に関与する。",
      "category": "type-system",
      "match_mode": "token",
      "enabled_pages": [
        "/reference/language-guide.html",
        "/reference/language-spec.html"
      ]
    },
    {
      "id": "strict-subterm",
      "label": "strict subterm",
      "aliases": [],
      "short_tip": "再帰呼び出しで元引数より真に小さい部分値。",
      "definition": "match 分解で得た子要素など、構造的に減少していることを示す ADT 部分項。",
      "category": "semantics",
      "match_mode": "exact",
      "enabled_pages": [
        "/reference/language-guide.html",
        "/reference/language-spec.html"
      ]
    },
    {
      "id": "tail-position",
      "label": "tail position",
      "aliases": [],
      "short_tip": "式評価の最終位置で結果をそのまま返す場所。",
      "definition": "追加計算を伴わず、呼び出し結果がそのまま関数結果になる式位置。",
      "category": "semantics",
      "match_mode": "exact",
      "enabled_pages": [
        "/reference/language-guide.html",
        "/reference/language-spec.html",
        "/reference/troubleshooting.html"
      ]
    },
    {
      "id": "semantic-dup",
      "label": "semantic-dup",
      "aliases": [],
      "short_tip": "有限モデル上の意味同値で重複候補を検出するモード。",
      "definition": "構文一致ではなく評価結果の同値性を使って重複候補を検出する lint オプション。",
      "category": "semantics",
      "match_mode": "token",
      "enabled_pages": [
        "/reference/language-guide.html",
        "/reference/language-spec.html",
        "/reference/troubleshooting.html",
        "/tutorial/quickstart.html",
        "/tutorial/first-policy.html"
      ]
    },
    {
      "id": "e-parse",
      "label": "E-PARSE",
      "aliases": [],
      "short_tip": "構文不正を示す診断コード。",
      "definition": "S 式の括弧不整合やフォーム構造崩れなど、構文解析段階の失敗を示すエラーコード。",
      "category": "diagnostics",
      "match_mode": "token",
      "enabled_pages": [
        "/reference/language-guide.html",
        "/reference/language-spec.html",
        "/reference/troubleshooting.html"
      ]
    },
    {
      "id": "e-syntax-auto",
      "label": "E-SYNTAX-AUTO",
      "aliases": [],
      "short_tip": "Core/Surface 自動判定衝突を示す診断コード。",
      "definition": "syntax:auto で同一ファイルに Core と Surface が混在したときに返るエラーコード。",
      "category": "diagnostics",
      "match_mode": "token",
      "enabled_pages": [
        "/reference/language-guide.html",
        "/reference/language-spec.html",
        "/reference/troubleshooting.html"
      ]
    },
    {
      "id": "e-resolve",
      "label": "E-RESOLVE",
      "aliases": [],
      "short_tip": "名前解決失敗を示す診断コード。",
      "definition": "未定義識別子、重複定義、unsafe rule などの名前解決違反を示すエラーコード。",
      "category": "diagnostics",
      "match_mode": "token",
      "enabled_pages": [
        "/reference/language-guide.html",
        "/reference/language-spec.html",
        "/reference/troubleshooting.html",
        "/tutorial/first-policy.html"
      ]
    },
    {
      "id": "e-type",
      "label": "E-TYPE",
      "aliases": [],
      "short_tip": "型不一致を示す診断コード。",
      "definition": "関数引数/戻り値、relation 引数、条件式などの型整合性違反を示すエラーコード。",
      "category": "diagnostics",
      "match_mode": "token",
      "enabled_pages": [
        "/reference/language-guide.html",
        "/reference/language-spec.html",
        "/reference/troubleshooting.html",
        "/tutorial/first-policy.html"
      ]
    },
    {
      "id": "e-total",
      "label": "E-TOTAL",
      "aliases": [],
      "short_tip": "停止性/全域性違反を示す診断コード。",
      "definition": "非 tail 再帰、非減少再帰、相互再帰など全域性条件違反を示すエラーコード。",
      "category": "diagnostics",
      "match_mode": "token",
      "enabled_pages": [
        "/reference/language-guide.html",
        "/reference/language-spec.html",
        "/reference/troubleshooting.html"
      ]
    },
    {
      "id": "e-match",
      "label": "E-MATCH",
      "aliases": [],
      "short_tip": "match 検査違反を示す診断コード。",
      "definition": "分岐非網羅、到達不能分岐、パターン型不一致を示すエラーコード。",
      "category": "diagnostics",
      "match_mode": "token",
      "enabled_pages": [
        "/reference/language-guide.html",
        "/reference/language-spec.html",
        "/reference/troubleshooting.html"
      ]
    },
    {
      "id": "e-prove",
      "label": "E-PROVE",
      "aliases": [],
      "short_tip": "証明失敗または universe 不備を示す診断コード。",
      "definition": "有限モデル検証での反例検出や必須 universe 欠落による失敗を示すエラーコード。",
      "category": "diagnostics",
      "match_mode": "token",
      "enabled_pages": [
        "/reference/language-spec.html",
        "/reference/troubleshooting.html"
      ]
    },
    {
      "id": "l-dup-exact",
      "label": "L-DUP-EXACT",
      "aliases": [],
      "short_tip": "構文正規化後に確定した重複警告。",
      "definition": "正規化済みフォームが同一であることを根拠に重複と判定された lint コード。",
      "category": "diagnostics",
      "match_mode": "token",
      "enabled_pages": [
        "/reference/language-guide.html",
        "/reference/language-spec.html"
      ]
    },
    {
      "id": "l-dup-maybe",
      "label": "L-DUP-MAYBE",
      "aliases": [],
      "short_tip": "意味同値にもとづく重複候補警告。",
      "definition": "有限モデルでの双方向検証結果から、重複の可能性を示す lint コード。",
      "category": "diagnostics",
      "match_mode": "token",
      "enabled_pages": [
        "/reference/language-guide.html",
        "/reference/language-spec.html",
        "/reference/troubleshooting.html",
        "/tutorial/quickstart.html"
      ]
    },
    {
      "id": "l-dup-skip-universe",
      "label": "L-DUP-SKIP-UNIVERSE",
      "aliases": [],
      "short_tip": "universe 不足で semantic-dup 判定を省略した警告。",
      "definition": "必要な universe が不足し、重複候補の意味検証を実施できなかったことを示す lint コード。",
      "category": "diagnostics",
      "match_mode": "token",
      "enabled_pages": [
        "/reference/language-guide.html",
        "/reference/language-spec.html",
        "/reference/troubleshooting.html",
        "/tutorial/first-policy.html"
      ]
    },
    {
      "id": "l-dup-skip-eval-depth",
      "label": "L-DUP-SKIP-EVAL-DEPTH",
      "aliases": [],
      "short_tip": "評価深さ上限到達で入力点を省略した警告。",
      "definition": "defn 比較時に再帰評価が深さ上限へ達し、判定対象の一部をスキップしたことを示す lint コード。",
      "category": "diagnostics",
      "match_mode": "token",
      "enabled_pages": [
        "/reference/language-guide.html",
        "/reference/language-spec.html",
        "/reference/troubleshooting.html"
      ]
    },
    {
      "id": "l-unused-decl",
      "label": "L-UNUSED-DECL",
      "aliases": [],
      "short_tip": "未参照宣言を示す警告コード。",
      "definition": "relation/sort/data などの宣言が参照されていないことを示す lint コード。",
      "category": "diagnostics",
      "match_mode": "token",
      "enabled_pages": [
        "/reference/language-guide.html",
        "/reference/language-spec.html"
      ]
    }
  ]
};

  const tokenCharPattern = /[A-Za-z0-9_-]/u;

  function resolveActivePage(pathname) {
    for (const page of payload.target_pages) {
      if (pathname.endsWith(page)) {
        return page;
      }
    }
    return null;
  }

  function isTokenChar(char) {
    return char.length > 0 && tokenCharPattern.test(char);
  }

  function hasTokenBoundary(text, start, end) {
    const prev = start > 0 ? text[start - 1] : "";
    const next = end < text.length ? text[end] : "";
    return (!prev || !isTokenChar(prev)) && (!next || !isTokenChar(next));
  }

  function buildVariants(activePage) {
    const variants = [];
    for (const term of payload.terms) {
      if (!Array.isArray(term.enabled_pages) || !term.enabled_pages.includes(activePage)) {
        continue;
      }
      const rawVariants = [term.label, ...(term.aliases || [])].filter(Boolean);
      const uniqueVariants = [...new Set(rawVariants)];
      for (const value of uniqueVariants) {
        variants.push({
          id: term.id,
          shortTip: term.short_tip,
          value,
          matchMode: term.match_mode
        });
      }
    }

    variants.sort((left, right) => {
      if (right.value.length !== left.value.length) {
        return right.value.length - left.value.length;
      }
      return left.value.localeCompare(right.value);
    });

    return variants;
  }

  function findNextMatch(text, cursor, variants) {
    let best = null;

    for (const variant of variants) {
      let index = text.indexOf(variant.value, cursor);
      while (index !== -1) {
        const end = index + variant.value.length;
        const boundaryOk =
          variant.matchMode !== "token" || hasTokenBoundary(text, index, end);
        if (boundaryOk) {
          if (
            best === null ||
            index < best.index ||
            (index === best.index && variant.value.length > best.variant.value.length)
          ) {
            best = { index, end, variant };
          }
          break;
        }
        index = text.indexOf(variant.value, index + 1);
      }
    }

    return best;
  }

  function buildGlossaryHref(termId) {
    const root = typeof window.path_to_root === "string" ? window.path_to_root : "";
    return `${root}reference/glossary.html#term-${termId}`;
  }

  function replaceTextNode(textNode, variants) {
    const text = textNode.nodeValue ?? "";
    if (text.trim().length === 0) {
      return;
    }

    let cursor = 0;
    let match = findNextMatch(text, cursor, variants);
    if (match === null) {
      return;
    }

    const fragment = document.createDocumentFragment();
    while (match !== null) {
      if (match.index > cursor) {
        fragment.appendChild(document.createTextNode(text.slice(cursor, match.index)));
      }

      const value = text.slice(match.index, match.end);
      const anchor = document.createElement("a");
      anchor.className = "dtl-term";
      anchor.href = buildGlossaryHref(match.variant.id);
      anchor.setAttribute("data-term-id", match.variant.id);
      anchor.setAttribute("data-tip", match.variant.shortTip);
      anchor.setAttribute("title", match.variant.shortTip);
      anchor.textContent = value;
      fragment.appendChild(anchor);

      cursor = match.end;
      match = findNextMatch(text, cursor, variants);
    }

    if (cursor < text.length) {
      fragment.appendChild(document.createTextNode(text.slice(cursor)));
    }

    const parent = textNode.parentNode;
    if (parent) {
      parent.replaceChild(fragment, textNode);
    }
  }

  function isEligibleTextNode(node, root) {
    const parent = node.parentElement;
    if (!parent || !root.contains(parent)) {
      return false;
    }

    if (!parent.closest("p, li, td, code")) {
      return false;
    }

    if (parent.closest("a, pre, h1, h2, h3, h4, h5, h6, script, style, .dtl-term")) {
      return false;
    }

    return true;
  }

  function annotateTerms() {
    const pathname = typeof window.location?.pathname === "string" ? window.location.pathname : "";
    const activePage = resolveActivePage(pathname);
    if (!activePage) {
      return;
    }

    const variants = buildVariants(activePage);
    if (variants.length === 0) {
      return;
    }

    const root = document.querySelector("#mdbook-content main");
    if (!root) {
      return;
    }

    const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
    const textNodes = [];
    while (walker.nextNode()) {
      const current = walker.currentNode;
      if (isEligibleTextNode(current, root)) {
        textNodes.push(current);
      }
    }

    for (const node of textNodes) {
      replaceTextNode(node, variants);
    }
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", annotateTerms);
  } else {
    annotateTerms();
  }
})();
