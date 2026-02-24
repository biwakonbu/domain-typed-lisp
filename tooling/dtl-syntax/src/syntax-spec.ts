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
  surfaceTopLevelKeywords: [
    "インポート",
    "型",
    "データ",
    "関係",
    "事実",
    "規則",
    "検証",
    "宇宙",
    "関数"
  ],
  specialFormKeywords: ["and", "not", "let", "if", "match"],
  typeKeywords: ["Bool", "Int", "Symbol", "Refine", "Adt"],
  booleanLiterals: ["true", "false"],
  surfaceTags: [
    ":コンストラクタ",
    ":constructors",
    ":ctors",
    ":引数",
    ":args",
    ":項",
    ":terms",
    ":頭",
    ":head",
    ":本体",
    ":body",
    ":params",
    ":式",
    ":formula",
    ":値",
    ":values",
    ":戻り",
    ":ret"
  ]
};

export function uniqueTokens(...groups: ReadonlyArray<readonly string[]>): string[] {
  return [...new Set(groups.flat())];
}
