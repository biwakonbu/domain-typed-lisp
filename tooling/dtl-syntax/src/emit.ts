import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";

import { fileURLToPath } from "node:url";

import { generateHighlightJsScript } from "./generate-highlightjs";
import { generateTextMateGrammar } from "./generate-textmate";
import { DTL_SYNTAX_SPEC } from "./syntax-spec";

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(__dirname, "../../..");

interface OutputFile {
  readonly path: string;
  readonly body: string;
}

function createOutputs(): OutputFile[] {
  const textMateGrammar = `${JSON.stringify(generateTextMateGrammar(DTL_SYNTAX_SPEC), null, 2)}\n`;
  const highlightScript = generateHighlightJsScript(DTL_SYNTAX_SPEC);

  return [
    {
      path: resolve(repoRoot, "editors/vscode-dtl/syntaxes/dtl.tmLanguage.json"),
      body: textMateGrammar
    },
    {
      path: resolve(repoRoot, "docs-site/theme/dtl-highlight.js"),
      body: highlightScript
    }
  ];
}

function hasDiff(path: string, expected: string): boolean {
  try {
    return readFileSync(path, "utf8") !== expected;
  } catch {
    return true;
  }
}

function writeFile(path: string, body: string): void {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, body, "utf8");
}

function main(): void {
  const checkMode = process.argv.includes("--check");
  const outputs = createOutputs();
  const dirtyFiles: string[] = [];

  for (const output of outputs) {
    if (hasDiff(output.path, output.body)) {
      dirtyFiles.push(output.path);
      if (!checkMode) {
        writeFile(output.path, output.body);
      }
    }
  }

  if (checkMode && dirtyFiles.length > 0) {
    console.error("generated files are out of date:");
    for (const file of dirtyFiles) {
      console.error(`- ${file}`);
    }
    process.exit(1);
  }

  if (!checkMode) {
    for (const output of outputs) {
      console.log(`generated: ${output.path}`);
    }
  }
}

main();
