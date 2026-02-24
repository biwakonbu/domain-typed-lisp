import { type DtlSyntaxSpec, uniqueTokens } from "./syntax-spec";

interface HighlightPayload {
  readonly keywordTokens: readonly string[];
  readonly typeKeywords: readonly string[];
  readonly booleanLiterals: readonly string[];
  readonly surfaceTags: readonly string[];
}

function buildPayload(spec: DtlSyntaxSpec): HighlightPayload {
  return {
    keywordTokens: uniqueTokens(
      spec.coreTopLevelKeywords,
      spec.surfaceTopLevelKeywords,
      spec.specialFormKeywords
    ),
    typeKeywords: spec.typeKeywords,
    booleanLiterals: spec.booleanLiterals,
    surfaceTags: spec.surfaceTags
  };
}

export function generateHighlightJsScript(spec: DtlSyntaxSpec): string {
  const payload = JSON.stringify(buildPayload(spec), null, 2);

  return `(() => {
  const payload = ${payload};

  const escapeRegex = (value) => value.replace(/[|\\\\{}()[\\]^$+*?.]/g, "\\\\$&");
  const tokenPattern = (tokens) =>
    new RegExp(
      \`(?<![^\\\\s()])(?:\${tokens.map(escapeRegex).join("|")})(?![^\\\\s()])\`,
      "u"
    );

  function dtlLanguage(hljs) {
    const keywordPattern = tokenPattern(payload.keywordTokens);
    const typePattern = tokenPattern(payload.typeKeywords);
    const literalPattern = tokenPattern(payload.booleanLiterals);
    const surfaceTagPattern = tokenPattern(payload.surfaceTags);

    return {
      name: "DTL",
      aliases: ["dtl"],
      unicodeRegex: true,
      contains: [
        hljs.COMMENT(";", "$"),
        {
          scope: "string",
          begin: /"/u,
          end: /"/u,
          illegal: /\\n/u
        },
        {
          scope: "number",
          begin: /(?<![^\\s()])[-+]?\\d+(?![^\\s()])/u
        },
        {
          scope: "variable",
          begin: /(?<![^\\s()])\\?[^\\s()\";]+(?![^\\s()])/u
        },
        {
          scope: "attr",
          begin: surfaceTagPattern
        },
        {
          scope: "type",
          begin: typePattern
        },
        {
          scope: "literal",
          begin: literalPattern
        },
        {
          scope: "operator",
          begin: /(?<![^\\s()])->(?![^\\s()])/u
        },
        {
          scope: "symbol",
          begin: /(?<![^\\s()])_(?![^\\s()])/u
        },
        {
          scope: "keyword",
          begin: keywordPattern
        }
      ]
    };
  }

  function highlightBlock(hljs, block) {
    const raw = block.textContent ?? "";
    block.textContent = raw;
    block.removeAttribute("data-highlighted");
    block.classList.remove("language-lisp");
    block.classList.add("language-dtl", "hljs");

    if (typeof hljs.highlightElement === "function") {
      hljs.highlightElement(block);
      return;
    }

    if (typeof hljs.highlightBlock === "function") {
      hljs.highlightBlock(block);
    }
  }

  function registerAndHighlight() {
    if (typeof window === "undefined" || !window.hljs) {
      return;
    }

    const hljs = window.hljs;
    if (!hljs.getLanguage("dtl")) {
      hljs.registerLanguage("dtl", dtlLanguage);
    }

    const blocks = document.querySelectorAll("pre code.language-dtl, pre code.language-lisp");
    for (const block of blocks) {
      highlightBlock(hljs, block);
    }
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", registerAndHighlight);
  } else {
    registerAndHighlight();
  }
})();
`;
}
