#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import re
import sys
from collections import defaultdict
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parent.parent
TERMS_FILE = REPO_ROOT / "docs-site" / "src" / "reference" / "glossary-terms.json"
GLOSSARY_MD = REPO_ROOT / "docs-site" / "src" / "reference" / "glossary.md"
TERMS_JS = REPO_ROOT / "docs-site" / "theme" / "dtl-terms.js"

REQUIRED_TERM_FIELDS = {
    "id",
    "label",
    "aliases",
    "short_tip",
    "definition",
    "category",
    "match_mode",
    "enabled_pages",
}

ID_PATTERN = re.compile(r"^[a-z0-9][a-z0-9-]*$")
SUPPORTED_MATCH_MODES = {"token", "exact"}

CATEGORY_TITLES = {
    "dsl-keyword": "DSL キーワード",
    "cli-subcommand": "CLI サブコマンド",
    "type-system": "型システム",
    "semantics": "意味論・検証概念",
    "diagnostics": "診断コード",
}

CATEGORY_ORDER = [
    "dsl-keyword",
    "cli-subcommand",
    "type-system",
    "semantics",
    "diagnostics",
]


def load_terms_file(path: Path) -> dict[str, Any]:
    if not path.exists():
        raise ValueError(f"用語台帳が見つかりません: {path}")

    try:
        raw = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        raise ValueError(f"JSON の構文が不正です: {path}: {exc}") from exc

    if not isinstance(raw, dict):
        raise ValueError("用語台帳のトップレベルは object である必要があります")

    for field in ("schema_version", "target_pages", "terms"):
        if field not in raw:
            raise ValueError(f"用語台帳に必須フィールドがありません: {field}")

    target_pages = raw["target_pages"]
    if not isinstance(target_pages, list) or not target_pages:
        raise ValueError("target_pages は 1 件以上の配列である必要があります")

    for page in target_pages:
        if not isinstance(page, str) or not page.startswith("/"):
            raise ValueError(f"target_pages の値が不正です: {page!r}")

    terms = raw["terms"]
    if not isinstance(terms, list) or not terms:
        raise ValueError("terms は 1 件以上の配列である必要があります")

    seen_ids: set[str] = set()
    for index, term in enumerate(terms):
        if not isinstance(term, dict):
            raise ValueError(f"terms[{index}] は object である必要があります")

        missing = REQUIRED_TERM_FIELDS - term.keys()
        if missing:
            missing_text = ", ".join(sorted(missing))
            raise ValueError(f"terms[{index}] に必須フィールドがありません: {missing_text}")

        unknown = set(term.keys()) - REQUIRED_TERM_FIELDS
        if unknown:
            unknown_text = ", ".join(sorted(unknown))
            raise ValueError(f"terms[{index}] に未知フィールドがあります: {unknown_text}")

        term_id = term["id"]
        if not isinstance(term_id, str) or not ID_PATTERN.match(term_id):
            raise ValueError(f"terms[{index}].id が不正です: {term_id!r}")

        if term_id in seen_ids:
            raise ValueError(f"terms[{index}].id が重複しています: {term_id}")
        seen_ids.add(term_id)

        for text_field in ("label", "short_tip", "definition", "category", "match_mode"):
            value = term[text_field]
            if not isinstance(value, str) or not value.strip():
                raise ValueError(f"terms[{index}].{text_field} は非空文字列である必要があります")

        aliases = term["aliases"]
        if not isinstance(aliases, list):
            raise ValueError(f"terms[{index}].aliases は配列である必要があります")
        for alias in aliases:
            if not isinstance(alias, str) or not alias.strip():
                raise ValueError(f"terms[{index}].aliases に不正値があります: {alias!r}")

        match_mode = term["match_mode"]
        if match_mode not in SUPPORTED_MATCH_MODES:
            raise ValueError(
                f"terms[{index}].match_mode は {sorted(SUPPORTED_MATCH_MODES)} のいずれかである必要があります"
            )

        enabled_pages = term["enabled_pages"]
        if not isinstance(enabled_pages, list) or not enabled_pages:
            raise ValueError(f"terms[{index}].enabled_pages は 1 件以上の配列である必要があります")

        for page in enabled_pages:
            if page not in target_pages:
                raise ValueError(
                    f"terms[{index}].enabled_pages に target_pages 外の値があります: {page!r}"
                )

    return raw


def category_sort_key(category: str) -> tuple[int, str]:
    if category in CATEGORY_ORDER:
        return (CATEGORY_ORDER.index(category), category)
    return (len(CATEGORY_ORDER), category)


def render_glossary_markdown(payload: dict[str, Any]) -> str:
    terms: list[dict[str, Any]] = payload["terms"]
    grouped: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for term in terms:
        grouped[term["category"]].append(term)

    lines: list[str] = [
        "# 用語集",
        "",
        "<!-- このファイルは scripts/generate-glossary-assets.py と docs-site/src/reference/glossary-terms.json から自動生成されます。 -->",
        "",
        "専門語の短義と定義を、本文中のツールチップと同一ソースで管理しています。",
        "",
        "## 対象ページ",
        "",
    ]

    for page in payload["target_pages"]:
        lines.append(f"- `{page}`")

    lines.extend(["", "## 一致方式", "", "- `token`: トークン境界で一致（前後が英数字/`_`/`-` でない場合のみ）", "- `exact`: 完全一致", ""])

    for category in sorted(grouped.keys(), key=category_sort_key):
        title = CATEGORY_TITLES.get(category, category)
        lines.append(f"## {title}")
        lines.append("")

        category_terms = sorted(grouped[category], key=lambda item: item["label"].lower())
        for term in category_terms:
            lines.append(f'<a id="term-{term["id"]}"></a>')
            lines.append(f'### `{term["label"]}`')
            lines.append("")
            lines.append(f'- 短義: {term["short_tip"]}')
            lines.append(f'- 定義: {term["definition"]}')

            aliases = term["aliases"]
            if aliases:
                alias_text = ", ".join(f"`{alias}`" for alias in aliases)
            else:
                alias_text = "なし"
            lines.append(f"- 別名: {alias_text}")

            pages_text = ", ".join(f"`{page}`" for page in term["enabled_pages"])
            lines.append(f"- 適用ページ: {pages_text}")
            lines.append(f'- 一致方式: `{term["match_mode"]}`')
            lines.append("")

    return "\n".join(lines).rstrip() + "\n"


def render_terms_script(payload: dict[str, Any]) -> str:
    payload_json = json.dumps(
        {
            "target_pages": payload["target_pages"],
            "terms": payload["terms"],
        },
        ensure_ascii=False,
        indent=2,
    )

    return (
        "(() => {\n"
        "  const payload = "
        + payload_json
        + ";\n\n"
        "  const tokenCharPattern = /[A-Za-z0-9_-]/u;\n\n"
        "  function resolveActivePage(pathname) {\n"
        "    for (const page of payload.target_pages) {\n"
        "      if (pathname.endsWith(page)) {\n"
        "        return page;\n"
        "      }\n"
        "    }\n"
        "    return null;\n"
        "  }\n\n"
        "  function isTokenChar(char) {\n"
        "    return char.length > 0 && tokenCharPattern.test(char);\n"
        "  }\n\n"
        "  function hasTokenBoundary(text, start, end) {\n"
        "    const prev = start > 0 ? text[start - 1] : \"\";\n"
        "    const next = end < text.length ? text[end] : \"\";\n"
        "    return (!prev || !isTokenChar(prev)) && (!next || !isTokenChar(next));\n"
        "  }\n\n"
        "  function buildVariants(activePage) {\n"
        "    const variants = [];\n"
        "    for (const term of payload.terms) {\n"
        "      if (!Array.isArray(term.enabled_pages) || !term.enabled_pages.includes(activePage)) {\n"
        "        continue;\n"
        "      }\n"
        "      const rawVariants = [term.label, ...(term.aliases || [])].filter(Boolean);\n"
        "      const uniqueVariants = [...new Set(rawVariants)];\n"
        "      for (const value of uniqueVariants) {\n"
        "        variants.push({\n"
        "          id: term.id,\n"
        "          shortTip: term.short_tip,\n"
        "          value,\n"
        "          matchMode: term.match_mode\n"
        "        });\n"
        "      }\n"
        "    }\n"
        "\n"
        "    variants.sort((left, right) => {\n"
        "      if (right.value.length !== left.value.length) {\n"
        "        return right.value.length - left.value.length;\n"
        "      }\n"
        "      return left.value.localeCompare(right.value);\n"
        "    });\n"
        "\n"
        "    return variants;\n"
        "  }\n\n"
        "  function findNextMatch(text, cursor, variants) {\n"
        "    let best = null;\n"
        "\n"
        "    for (const variant of variants) {\n"
        "      let index = text.indexOf(variant.value, cursor);\n"
        "      while (index !== -1) {\n"
        "        const end = index + variant.value.length;\n"
        "        const boundaryOk =\n"
        "          variant.matchMode !== \"token\" || hasTokenBoundary(text, index, end);\n"
        "        if (boundaryOk) {\n"
        "          if (\n"
        "            best === null ||\n"
        "            index < best.index ||\n"
        "            (index === best.index && variant.value.length > best.variant.value.length)\n"
        "          ) {\n"
        "            best = { index, end, variant };\n"
        "          }\n"
        "          break;\n"
        "        }\n"
        "        index = text.indexOf(variant.value, index + 1);\n"
        "      }\n"
        "    }\n"
        "\n"
        "    return best;\n"
        "  }\n\n"
        "  function resolvePathToRoot() {\n"
        "    if (typeof path_to_root === \"string\") {\n"
        "      return path_to_root;\n"
        "    }\n"
        "    if (typeof window.path_to_root === \"string\") {\n"
        "      return window.path_to_root;\n"
        "    }\n"
        "    return \"\";\n"
        "  }\n\n"
        "  function buildGlossaryHref(termId) {\n"
        "    const root = resolvePathToRoot();\n"
        "    return `${root}reference/glossary.html#term-${termId}`;\n"
        "  }\n\n"
        "  function replaceTextNode(textNode, variants) {\n"
        "    const text = textNode.nodeValue ?? \"\";\n"
        "    if (text.trim().length === 0) {\n"
        "      return;\n"
        "    }\n"
        "\n"
        "    let cursor = 0;\n"
        "    let match = findNextMatch(text, cursor, variants);\n"
        "    if (match === null) {\n"
        "      return;\n"
        "    }\n"
        "\n"
        "    const fragment = document.createDocumentFragment();\n"
        "    while (match !== null) {\n"
        "      if (match.index > cursor) {\n"
        "        fragment.appendChild(document.createTextNode(text.slice(cursor, match.index)));\n"
        "      }\n"
        "\n"
        "      const value = text.slice(match.index, match.end);\n"
        "      const anchor = document.createElement(\"a\");\n"
        "      anchor.className = \"dtl-term\";\n"
        "      anchor.href = buildGlossaryHref(match.variant.id);\n"
        "      anchor.setAttribute(\"data-term-id\", match.variant.id);\n"
        "      anchor.setAttribute(\"data-tip\", match.variant.shortTip);\n"
        "      anchor.setAttribute(\"title\", match.variant.shortTip);\n"
        "      anchor.textContent = value;\n"
        "      fragment.appendChild(anchor);\n"
        "\n"
        "      cursor = match.end;\n"
        "      match = findNextMatch(text, cursor, variants);\n"
        "    }\n"
        "\n"
        "    if (cursor < text.length) {\n"
        "      fragment.appendChild(document.createTextNode(text.slice(cursor)));\n"
        "    }\n"
        "\n"
        "    const parent = textNode.parentNode;\n"
        "    if (parent) {\n"
        "      parent.replaceChild(fragment, textNode);\n"
        "    }\n"
        "  }\n\n"
        "  function isEligibleTextNode(node, root) {\n"
        "    const parent = node.parentElement;\n"
        "    if (!parent || !root.contains(parent)) {\n"
        "      return false;\n"
        "    }\n"
        "\n"
        "    if (!parent.closest(\"p, li, td, code\")) {\n"
        "      return false;\n"
        "    }\n"
        "\n"
        "    if (parent.closest(\"a, pre, h1, h2, h3, h4, h5, h6, script, style, .dtl-term\")) {\n"
        "      return false;\n"
        "    }\n"
        "\n"
        "    return true;\n"
        "  }\n\n"
        "  function annotateTerms() {\n"
        "    const pathname = typeof window.location?.pathname === \"string\" ? window.location.pathname : \"\";\n"
        "    const activePage = resolveActivePage(pathname);\n"
        "    if (!activePage) {\n"
        "      return;\n"
        "    }\n"
        "\n"
        "    const variants = buildVariants(activePage);\n"
        "    if (variants.length === 0) {\n"
        "      return;\n"
        "    }\n"
        "\n"
        "    const root = document.querySelector(\"#mdbook-content main\");\n"
        "    if (!root) {\n"
        "      return;\n"
        "    }\n"
        "\n"
        "    const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);\n"
        "    const textNodes = [];\n"
        "    while (walker.nextNode()) {\n"
        "      const current = walker.currentNode;\n"
        "      if (isEligibleTextNode(current, root)) {\n"
        "        textNodes.push(current);\n"
        "      }\n"
        "    }\n"
        "\n"
        "    for (const node of textNodes) {\n"
        "      replaceTextNode(node, variants);\n"
        "    }\n"
        "  }\n\n"
        "  if (document.readyState === \"loading\") {\n"
        "    document.addEventListener(\"DOMContentLoaded\", annotateTerms);\n"
        "  } else {\n"
        "    annotateTerms();\n"
        "  }\n"
        "})();\n"
    )


def diff_outputs(outputs: dict[Path, str]) -> list[Path]:
    dirty: list[Path] = []
    for path, expected in outputs.items():
        try:
            current = path.read_text(encoding="utf-8")
        except FileNotFoundError:
            current = None

        if current != expected:
            dirty.append(path)

    return dirty


def write_outputs(outputs: dict[Path, str]) -> None:
    for path, body in outputs.items():
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(body, encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(description="glossary.md と dtl-terms.js を生成します")
    parser.add_argument("--check", action="store_true", help="差分がある場合に非0終了")
    args = parser.parse_args()

    try:
        payload = load_terms_file(TERMS_FILE)
    except ValueError as exc:
        print(str(exc), file=sys.stderr)
        return 1

    outputs = {
        GLOSSARY_MD: render_glossary_markdown(payload),
        TERMS_JS: render_terms_script(payload),
    }

    dirty = diff_outputs(outputs)

    if args.check:
        if dirty:
            print("generated glossary assets are out of date:", file=sys.stderr)
            for path in dirty:
                print(f"- {path}", file=sys.stderr)
            return 1
        return 0

    write_outputs(outputs)
    for path in outputs:
        print(f"generated: {path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
