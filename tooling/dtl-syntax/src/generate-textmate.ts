import { type DtlSyntaxSpec, uniqueTokens } from "./syntax-spec";

const TEXTMATE_SCHEMA_URL =
  "https://raw.githubusercontent.com/martinring/tmlanguage/master/tmlanguage.json";

function escapeRegexLiteral(value: string): string {
  return value.replace(/[.*+?^${}()|[\\]\\]/g, "\\$&");
}

function delimitedTokenPattern(tokens: readonly string[]): string {
  const body = tokens.map(escapeRegexLiteral).join("|");
  return `(?<![^\\s()])(?:${body})(?![^\\s()])`;
}

export function generateTextMateGrammar(spec: DtlSyntaxSpec): Record<string, unknown> {
  const keywordTokens = uniqueTokens(
    spec.coreTopLevelKeywords,
    spec.surfaceTopLevelKeywords,
    spec.specialFormKeywords
  );

  return {
    $schema: TEXTMATE_SCHEMA_URL,
    name: "DTL",
    scopeName: "source.dtl",
    patterns: [
      { include: "#comment" },
      { include: "#string" },
      { include: "#number" },
      { include: "#logicVar" },
      { include: "#surfaceTag" },
      { include: "#typeKeyword" },
      { include: "#booleanLiteral" },
      { include: "#operator" },
      { include: "#wildcard" },
      { include: "#keyword" }
    ],
    repository: {
      comment: {
        patterns: [{ name: "comment.line.semicolon.dtl", match: ";.*$" }]
      },
      string: {
        patterns: [{ name: "string.quoted.double.dtl", match: '"[^"\\n]*"' }]
      },
      number: {
        patterns: [
          {
            name: "constant.numeric.integer.dtl",
            match: "(?<![^\\s()])[-+]?\\d+(?![^\\s()])"
          }
        ]
      },
      logicVar: {
        patterns: [
          {
            name: "variable.other.logic.dtl",
            match: "(?<![^\\s()])\\?[^\\s()\";]+(?![^\\s()])"
          }
        ]
      },
      surfaceTag: {
        patterns: [
          {
            name: "entity.other.attribute-name.tag.dtl",
            match: delimitedTokenPattern(spec.surfaceTags)
          }
        ]
      },
      typeKeyword: {
        patterns: [
          {
            name: "storage.type.dtl",
            match: delimitedTokenPattern(spec.typeKeywords)
          }
        ]
      },
      booleanLiteral: {
        patterns: [
          {
            name: "constant.language.boolean.dtl",
            match: delimitedTokenPattern(spec.booleanLiterals)
          }
        ]
      },
      operator: {
        patterns: [
          {
            name: "keyword.operator.function-type.dtl",
            match: "(?<![^\\s()])->(?![^\\s()])"
          }
        ]
      },
      wildcard: {
        patterns: [
          {
            name: "constant.language.wildcard.dtl",
            match: "(?<![^\\s()])_(?![^\\s()])"
          }
        ]
      },
      keyword: {
        patterns: [
          {
            name: "keyword.control.dtl",
            match: delimitedTokenPattern(keywordTokens)
          }
        ]
      }
    }
  };
}
