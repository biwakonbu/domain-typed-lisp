import { describe, expect, it } from "bun:test";

import { generateHighlightJsScript } from "../src/generate-highlightjs";
import { generateTextMateGrammar } from "../src/generate-textmate";
import { DTL_SYNTAX_SPEC } from "../src/syntax-spec";

describe("dtl syntax generator", () => {
  it("generates deterministic textmate grammar", () => {
    const once = JSON.stringify(generateTextMateGrammar(DTL_SYNTAX_SPEC));
    const twice = JSON.stringify(generateTextMateGrammar(DTL_SYNTAX_SPEC));

    expect(once).toBe(twice);
    expect(once).toContain("source.dtl");
    expect(once).toContain("keyword.control.dtl");
    expect(once).toContain("entity.other.attribute-name.tag.dtl");
    expect(once).toContain("インポート");
    expect(once).toContain(":引数");
  });

  it("generates highlight.js runtime with dtl+lisp compatibility", () => {
    const script = generateHighlightJsScript(DTL_SYNTAX_SPEC);

    expect(script).toContain("registerLanguage(\"dtl\"");
    expect(script).toContain("pre code.language-dtl, pre code.language-lisp");
    expect(script).toContain("language-dtl");
    expect(script).toContain("surfaceTagPattern");
  });
});
