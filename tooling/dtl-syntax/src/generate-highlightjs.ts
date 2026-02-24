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
  const escapeHtml = (value) =>
    value
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;");
  const supportsLookbehind = (() => {
    try {
      return /(?<!a)b/u.test("b");
    } catch {
      return false;
    }
  })();
  const tokenPattern = (tokens) => {
    const body = tokens.map(escapeRegex).join("|");
    if (supportsLookbehind) {
      return new RegExp("(?<![^\\\\s()])(?:" + body + ")(?![^\\\\s()])", "u");
    }
    return new RegExp("(?:" + body + ")", "u");
  };

  function dtlLanguage(hljs) {
    const keywordPattern = tokenPattern(payload.keywordTokens);
    const typePattern = tokenPattern(payload.typeKeywords);
    const literalPattern = tokenPattern(payload.booleanLiterals);
    const surfaceTagPattern = tokenPattern(payload.surfaceTags);
    const dtlNumberPattern = supportsLookbehind
      ? /(?<![^\\s()])[-+]?\\d+(?![^\\s()])/u
      : /[-+]?\\d+/u;
    const dtlVariablePattern = supportsLookbehind
      ? /(?<![^\\s()])\\?[^\\s()";]+(?![^\\s()])/u
      : /\\?[^\\s()";]+/u;
    const dtlOperatorPattern = supportsLookbehind
      ? /(?<![^\\s()])->(?![^\\s()])/u
      : /->/u;
    const dtlWildcardPattern = supportsLookbehind
      ? /(?<![^\\s()])_(?![^\\s()])/u
      : /_/u;

    return {
      name: "DTL",
      aliases: ["dtl"],
      unicodeRegex: true,
      contains: [
        hljs.COMMENT(";", "$"),
        {
          className: "string",
          begin: /"/u,
          end: /"/u,
          illegal: /\\n/u
        },
        {
          className: "number",
          begin: dtlNumberPattern
        },
        {
          className: "variable",
          begin: dtlVariablePattern
        },
        {
          className: "attr",
          begin: surfaceTagPattern
        },
        {
          className: "type",
          begin: typePattern
        },
        {
          className: "literal",
          begin: literalPattern
        },
        {
          className: "operator",
          begin: dtlOperatorPattern
        },
        {
          className: "symbol",
          begin: dtlWildcardPattern
        },
        {
          className: "keyword",
          begin: keywordPattern
        }
      ]
    };
  }

  const terminalTokenPattern =
    /"(?:[^"\\\\]|\\\\.)*"|'(?:[^'\\\\]|\\\\.)*'|\\$\\{[A-Za-z_][A-Za-z0-9_]*\\}|\\$[A-Za-z_][A-Za-z0-9_]*|--?[A-Za-z0-9][A-Za-z0-9-]*|(?:~\\/|\\.{1,2}\\/|\\/)[^\\s"']+|[+-]?\\d+|[^\\s]+|\\s+/gu;
  const promptPattern = /^(\\s*(?:\\$|#|>|❯)\\s+)/u;
  const commentLinePattern = /^\\s*#/u;
  const spacePattern = /^\\s+$/u;
  const stringPattern = /^(?:"(?:[^"\\\\]|\\\\.)*"|'(?:[^'\\\\]|\\\\.)*')$/u;
  const variablePattern = /^\\$(?:\\{[A-Za-z_][A-Za-z0-9_]*\\}|[A-Za-z_][A-Za-z0-9_]*)$/u;
  const assignmentPattern = /^[A-Za-z_][A-Za-z0-9_]*=.*/u;
  const flagPattern = /^--?[A-Za-z0-9][A-Za-z0-9-]*$/u;
  const pathPattern = /^(?:~\\/|\\.{1,2}\\/|\\/)[^\\s"']+$/u;
  const numberPattern = /^[+-]?\\d+$/u;
  const operatorPattern = /^(?:\\|\\||&&|[|<>])$/u;

  const wrapToken = (className, token) =>
    '<span class="hljs-' + className + '">' + escapeHtml(token) + "</span>";

  function classifyTerminalToken(token) {
    if (stringPattern.test(token)) {
      return "string";
    }
    if (variablePattern.test(token) || assignmentPattern.test(token)) {
      return "variable";
    }
    if (flagPattern.test(token)) {
      return "keyword";
    }
    if (pathPattern.test(token)) {
      return "attr";
    }
    if (numberPattern.test(token)) {
      return "number";
    }
    if (operatorPattern.test(token)) {
      return "operator";
    }
    return null;
  }

  function highlightTerminalLine(line) {
    if (line.length === 0) {
      return "";
    }

    if (commentLinePattern.test(line)) {
      return wrapToken("comment", line);
    }

    let rest = line;
    let result = "";
    const promptMatch = rest.match(promptPattern);
    if (promptMatch) {
      result += wrapToken("meta", promptMatch[1]);
      rest = rest.slice(promptMatch[1].length);
      if (rest.length === 0) {
        return result;
      }
    }

    if (commentLinePattern.test(rest)) {
      return result + wrapToken("comment", rest);
    }

    let hasCommand = false;
    const tokens = rest.match(terminalTokenPattern) ?? [];
    for (const token of tokens) {
      if (spacePattern.test(token)) {
        result += token;
        continue;
      }

      let className = null;
      if (!hasCommand) {
        if (assignmentPattern.test(token)) {
          className = "variable";
        } else {
          className = "built_in";
          hasCommand = true;
        }
      } else {
        className = classifyTerminalToken(token);
      }

      result += className ? wrapToken(className, token) : escapeHtml(token);
    }

    return result;
  }

  function highlightTerminalBlock(block) {
    const raw = block.textContent ?? "";
    const highlighted = raw
      .split(/\\r?\\n/u)
      .map(highlightTerminalLine)
      .join("\\n");

    block.innerHTML = highlighted;
    block.removeAttribute("data-highlighted");
    block.classList.add("hljs", "language-terminal");
  }

  function highlightDtlBlock(hljs, block) {
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

    const dtlBlocks = document.querySelectorAll("pre code.language-dtl, pre code.language-lisp");
    for (const block of dtlBlocks) {
      highlightDtlBlock(hljs, block);
    }

    const terminalBlocks = document.querySelectorAll(
      "pre code.language-bash, pre code.language-sh, pre code.language-zsh, pre code.language-shell, pre code.language-console, pre code.language-terminal, pre code.language-shellsession"
    );
    for (const block of terminalBlocks) {
      highlightTerminalBlock(block);
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
