export interface DtlSyntaxSpec {
  readonly coreTopLevelKeywords: readonly string[];
  readonly surfaceTopLevelKeywords: readonly string[];
  readonly specialFormKeywords: readonly string[];
  readonly typeKeywords: readonly string[];
  readonly booleanLiterals: readonly string[];
  readonly surfaceTags: readonly string[];
}

export const DTL_SYNTAX_SPEC: DtlSyntaxSpec = {
  coreTopLevelKeywords: [
    "import",
    "sort",
    "data",
    "relation",
    "fact",
    "rule",
    "assert",
    "universe",
    "defn"
  ],
  surfaceTopLevelKeywords: ["project", "module", "reference", "contract", "quality-gate"],
  specialFormKeywords: ["and", "not", "let", "if", "match"],
  typeKeywords: ["Bool", "Int", "Symbol", "Refine", "Adt"],
  booleanLiterals: ["true", "false"],
  surfaceTags: [
    ":alias",
    ":canonical",
    ":constructors",
    ":args",
    ":terms",
    ":head",
    ":body",
    ":params",
    ":formula",
    ":values",
    ":ret",
    ":name",
    ":summary",
    ":path",
    ":category",
    ":from",
    ":to",
    ":source",
    ":command",
    ":required"
  ]
};

export function uniqueTokens(...groups: ReadonlyArray<readonly string[]>): string[] {
  return [...new Set(groups.flat())];
}
