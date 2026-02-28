use crate::ast::{
    AliasDecl, AssertDecl, DataDecl, Defn, Expr, Fact, ImportDecl, Pattern, Program, RelationDecl,
    Rule, SortDecl, UniverseDecl,
};
use crate::diagnostics::Diagnostic;
use crate::parser::parse_program;
use crate::types::{Atom, Formula, LogicTerm, Type};
use std::iter::Peekable;
use std::str::CharIndices;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy)]
pub struct FormatOptions {
    pub preserve_context: bool,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            preserve_context: true,
        }
    }
}

pub fn format_source(src: &str, options: FormatOptions) -> Result<String, Vec<Diagnostic>> {
    let program = parse_program(src)?;
    if contains_selfdoc_form(src) {
        return Ok(src.trim_end().to_string() + "\n");
    }

    let mut out = String::new();
    out.push_str("; syntax: surface\n");

    if options.preserve_context {
        render_with_context_blocks(program, src, &mut out);
    } else {
        let mut forms = ContextForms::from_program(program);
        forms.sort_for_render();
        render_forms(&forms, &mut out);
    }

    Ok(out.trim_end().to_string() + "\n")
}

#[derive(Debug, Clone, Copy)]
enum TopLevelKind {
    Import,
    Alias,
    Sort,
    Data,
    Relation,
    Fact,
    Rule,
    Assert,
    Universe,
    Defn,
}

#[derive(Debug, Default)]
struct ContextAssignments {
    block_names: Vec<String>,
    imports: Vec<Option<usize>>,
    aliases: Vec<Option<usize>>,
    sorts: Vec<Option<usize>>,
    data_decls: Vec<Option<usize>>,
    relations: Vec<Option<usize>>,
    facts: Vec<Option<usize>>,
    rules: Vec<Option<usize>>,
    asserts: Vec<Option<usize>>,
    universes: Vec<Option<usize>>,
    defns: Vec<Option<usize>>,
}

impl ContextAssignments {
    fn push(&mut self, kind: TopLevelKind, block_idx: Option<usize>) {
        match kind {
            TopLevelKind::Import => self.imports.push(block_idx),
            TopLevelKind::Alias => self.aliases.push(block_idx),
            TopLevelKind::Sort => self.sorts.push(block_idx),
            TopLevelKind::Data => self.data_decls.push(block_idx),
            TopLevelKind::Relation => self.relations.push(block_idx),
            TopLevelKind::Fact => self.facts.push(block_idx),
            TopLevelKind::Rule => self.rules.push(block_idx),
            TopLevelKind::Assert => self.asserts.push(block_idx),
            TopLevelKind::Universe => self.universes.push(block_idx),
            TopLevelKind::Defn => self.defns.push(block_idx),
        }
    }
}

#[derive(Debug, Default)]
struct ContextForms {
    imports: Vec<ImportDecl>,
    aliases: Vec<AliasDecl>,
    sorts: Vec<SortDecl>,
    data_decls: Vec<DataDecl>,
    relations: Vec<RelationDecl>,
    facts: Vec<Fact>,
    rules: Vec<Rule>,
    asserts: Vec<AssertDecl>,
    universes: Vec<UniverseDecl>,
    defns: Vec<Defn>,
}

impl ContextForms {
    fn from_program(program: Program) -> Self {
        Self {
            imports: program.imports,
            aliases: program.aliases,
            sorts: program.sorts,
            data_decls: program.data_decls,
            relations: program.relations,
            facts: program.facts,
            rules: program.rules,
            asserts: program.asserts,
            universes: program.universes,
            defns: program.defns,
        }
    }

    fn is_empty(&self) -> bool {
        self.imports.is_empty()
            && self.aliases.is_empty()
            && self.sorts.is_empty()
            && self.data_decls.is_empty()
            && self.relations.is_empty()
            && self.facts.is_empty()
            && self.rules.is_empty()
            && self.asserts.is_empty()
            && self.universes.is_empty()
            && self.defns.is_empty()
    }

    fn sort_for_render(&mut self) {
        self.imports.sort_by(|a, b| a.path.cmp(&b.path));
        self.aliases
            .sort_by(|a, b| a.alias.cmp(&b.alias).then(a.canonical.cmp(&b.canonical)));
        self.sorts.sort_by(|a, b| a.name.cmp(&b.name));
        self.data_decls.sort_by(|a, b| a.name.cmp(&b.name));
        self.relations.sort_by(|a, b| a.name.cmp(&b.name));
        self.universes.sort_by(|a, b| a.ty_name.cmp(&b.ty_name));
    }
}

fn render_with_context_blocks(program: Program, src: &str, out: &mut String) {
    let Program {
        imports,
        aliases,
        sorts,
        data_decls,
        relations,
        facts,
        rules,
        asserts,
        universes,
        defns,
    } = program;

    let assignments = collect_context_assignments(src);

    let mut prelude = ContextForms::default();
    let mut blocks = assignments
        .block_names
        .iter()
        .map(|name| (name.clone(), ContextForms::default()))
        .collect::<Vec<_>>();

    assign_imports(imports, &assignments.imports, &mut prelude, &mut blocks);
    assign_aliases(aliases, &assignments.aliases, &mut prelude, &mut blocks);
    assign_sorts(sorts, &assignments.sorts, &mut prelude, &mut blocks);
    assign_data_decls(
        data_decls,
        &assignments.data_decls,
        &mut prelude,
        &mut blocks,
    );
    assign_relations(relations, &assignments.relations, &mut prelude, &mut blocks);
    assign_facts(facts, &assignments.facts, &mut prelude, &mut blocks);
    assign_rules(rules, &assignments.rules, &mut prelude, &mut blocks);
    assign_asserts(asserts, &assignments.asserts, &mut prelude, &mut blocks);
    assign_universes(universes, &assignments.universes, &mut prelude, &mut blocks);
    assign_defns(defns, &assignments.defns, &mut prelude, &mut blocks);

    prelude.sort_for_render();
    for (_, forms) in &mut blocks {
        forms.sort_for_render();
    }

    let mut emitted = false;
    if blocks.is_empty() || !prelude.is_empty() {
        out.push_str("; @context: default\n\n");
        render_forms(&prelude, out);
        emitted = true;
    }

    for (idx, (name, forms)) in blocks.iter().enumerate() {
        if emitted || idx > 0 {
            out.push('\n');
        }
        out.push_str(&format!("; @context: {name}\n\n"));
        render_forms(forms, out);
        emitted = true;
    }
}

fn canonical_top_level_kind(head: &str) -> Option<TopLevelKind> {
    match head {
        "import" | "インポート" => Some(TopLevelKind::Import),
        "alias" | "同義語" => Some(TopLevelKind::Alias),
        "sort" | "型" => Some(TopLevelKind::Sort),
        "data" | "データ" => Some(TopLevelKind::Data),
        "relation" | "関係" => Some(TopLevelKind::Relation),
        "fact" | "事実" => Some(TopLevelKind::Fact),
        "rule" | "規則" => Some(TopLevelKind::Rule),
        "assert" | "検証" => Some(TopLevelKind::Assert),
        "universe" | "宇宙" => Some(TopLevelKind::Universe),
        "defn" | "関数" => Some(TopLevelKind::Defn),
        _ => None,
    }
}

fn contains_selfdoc_form(src: &str) -> bool {
    static SELF_DOC_FORM_RE: OnceLock<regex::Regex> = OnceLock::new();
    let pattern = SELF_DOC_FORM_RE.get_or_init(|| {
        regex::Regex::new(
            r"\(\s*(?:project|module|reference|contract|quality-gate|プロジェクト|モジュール|参照|契約|品質ゲート)\s*:",
        )
        .expect("valid selfdoc regex")
    });
    pattern.is_match(src)
}

fn collect_context_assignments(src: &str) -> ContextAssignments {
    let mut out = ContextAssignments::default();
    let mut current_block = None;
    let mut it = src.char_indices().peekable();

    while let Some((_, ch)) = it.next() {
        if ch.is_whitespace() {
            continue;
        }
        if ch == ';' {
            let comment = consume_comment(&mut it);
            if let Some(ctx) = parse_context_marker(&comment) {
                out.block_names.push(ctx.to_string());
                current_block = Some(out.block_names.len() - 1);
            }
            continue;
        }
        if ch != '(' {
            continue;
        }

        let mut depth = 1usize;
        let head = consume_head_atom(&mut it, &mut depth);
        if let Some(kind) = canonical_top_level_kind(head.as_str()) {
            out.push(kind, current_block);
        }
        consume_to_form_end(&mut it, &mut depth);
    }

    out
}

fn consume_comment(it: &mut Peekable<CharIndices<'_>>) -> String {
    let mut out = String::new();
    while let Some((_, ch)) = it.peek().copied() {
        if ch == '\n' {
            break;
        }
        out.push(ch);
        it.next();
    }
    out
}

fn parse_context_marker(comment: &str) -> Option<&str> {
    let body = comment.trim();
    let ctx = body.strip_prefix("@context:")?;
    let name = ctx.trim();
    (!name.is_empty()).then_some(name)
}

fn consume_head_atom(it: &mut Peekable<CharIndices<'_>>, depth: &mut usize) -> String {
    loop {
        let Some((_, ch)) = it.peek().copied() else {
            return String::new();
        };
        if ch.is_whitespace() {
            it.next();
            continue;
        }
        if ch == ';' {
            it.next();
            consume_comment(it);
            continue;
        }
        if ch == '(' {
            *depth += 1;
            it.next();
            return String::new();
        }
        if ch == ')' {
            *depth = depth.saturating_sub(1);
            it.next();
            return String::new();
        }

        let mut atom = String::new();
        while let Some((_, c)) = it.peek().copied() {
            if c.is_whitespace() || c == '(' || c == ')' || c == ';' {
                break;
            }
            atom.push(c);
            it.next();
        }
        return atom;
    }
}

fn consume_to_form_end(it: &mut Peekable<CharIndices<'_>>, depth: &mut usize) {
    while *depth > 0 {
        let Some((_, ch)) = it.next() else {
            break;
        };
        match ch {
            ';' => {
                consume_comment(it);
            }
            '(' => *depth += 1,
            ')' => *depth = depth.saturating_sub(1),
            _ => {}
        }
    }
}

fn assign_imports(
    items: Vec<ImportDecl>,
    contexts: &[Option<usize>],
    prelude: &mut ContextForms,
    blocks: &mut [(String, ContextForms)],
) {
    for (idx, item) in items.into_iter().enumerate() {
        if let Some(block_idx) = contexts.get(idx).copied().flatten()
            && let Some((_, forms)) = blocks.get_mut(block_idx)
        {
            forms.imports.push(item);
            continue;
        }
        prelude.imports.push(item);
    }
}

fn assign_aliases(
    items: Vec<AliasDecl>,
    contexts: &[Option<usize>],
    prelude: &mut ContextForms,
    blocks: &mut [(String, ContextForms)],
) {
    for (idx, item) in items.into_iter().enumerate() {
        if let Some(block_idx) = contexts.get(idx).copied().flatten()
            && let Some((_, forms)) = blocks.get_mut(block_idx)
        {
            forms.aliases.push(item);
            continue;
        }
        prelude.aliases.push(item);
    }
}

fn assign_sorts(
    items: Vec<SortDecl>,
    contexts: &[Option<usize>],
    prelude: &mut ContextForms,
    blocks: &mut [(String, ContextForms)],
) {
    for (idx, item) in items.into_iter().enumerate() {
        if let Some(block_idx) = contexts.get(idx).copied().flatten()
            && let Some((_, forms)) = blocks.get_mut(block_idx)
        {
            forms.sorts.push(item);
            continue;
        }
        prelude.sorts.push(item);
    }
}

fn assign_data_decls(
    items: Vec<DataDecl>,
    contexts: &[Option<usize>],
    prelude: &mut ContextForms,
    blocks: &mut [(String, ContextForms)],
) {
    for (idx, item) in items.into_iter().enumerate() {
        if let Some(block_idx) = contexts.get(idx).copied().flatten()
            && let Some((_, forms)) = blocks.get_mut(block_idx)
        {
            forms.data_decls.push(item);
            continue;
        }
        prelude.data_decls.push(item);
    }
}

fn assign_relations(
    items: Vec<RelationDecl>,
    contexts: &[Option<usize>],
    prelude: &mut ContextForms,
    blocks: &mut [(String, ContextForms)],
) {
    for (idx, item) in items.into_iter().enumerate() {
        if let Some(block_idx) = contexts.get(idx).copied().flatten()
            && let Some((_, forms)) = blocks.get_mut(block_idx)
        {
            forms.relations.push(item);
            continue;
        }
        prelude.relations.push(item);
    }
}

fn assign_facts(
    items: Vec<Fact>,
    contexts: &[Option<usize>],
    prelude: &mut ContextForms,
    blocks: &mut [(String, ContextForms)],
) {
    for (idx, item) in items.into_iter().enumerate() {
        if let Some(block_idx) = contexts.get(idx).copied().flatten()
            && let Some((_, forms)) = blocks.get_mut(block_idx)
        {
            forms.facts.push(item);
            continue;
        }
        prelude.facts.push(item);
    }
}

fn assign_rules(
    items: Vec<Rule>,
    contexts: &[Option<usize>],
    prelude: &mut ContextForms,
    blocks: &mut [(String, ContextForms)],
) {
    for (idx, item) in items.into_iter().enumerate() {
        if let Some(block_idx) = contexts.get(idx).copied().flatten()
            && let Some((_, forms)) = blocks.get_mut(block_idx)
        {
            forms.rules.push(item);
            continue;
        }
        prelude.rules.push(item);
    }
}

fn assign_asserts(
    items: Vec<AssertDecl>,
    contexts: &[Option<usize>],
    prelude: &mut ContextForms,
    blocks: &mut [(String, ContextForms)],
) {
    for (idx, item) in items.into_iter().enumerate() {
        if let Some(block_idx) = contexts.get(idx).copied().flatten()
            && let Some((_, forms)) = blocks.get_mut(block_idx)
        {
            forms.asserts.push(item);
            continue;
        }
        prelude.asserts.push(item);
    }
}

fn assign_universes(
    items: Vec<UniverseDecl>,
    contexts: &[Option<usize>],
    prelude: &mut ContextForms,
    blocks: &mut [(String, ContextForms)],
) {
    for (idx, item) in items.into_iter().enumerate() {
        if let Some(block_idx) = contexts.get(idx).copied().flatten()
            && let Some((_, forms)) = blocks.get_mut(block_idx)
        {
            forms.universes.push(item);
            continue;
        }
        prelude.universes.push(item);
    }
}

fn assign_defns(
    items: Vec<Defn>,
    contexts: &[Option<usize>],
    prelude: &mut ContextForms,
    blocks: &mut [(String, ContextForms)],
) {
    for (idx, item) in items.into_iter().enumerate() {
        if let Some(block_idx) = contexts.get(idx).copied().flatten()
            && let Some((_, forms)) = blocks.get_mut(block_idx)
        {
            forms.defns.push(item);
            continue;
        }
        prelude.defns.push(item);
    }
}

fn render_forms(forms: &ContextForms, out: &mut String) {
    for import in &forms.imports {
        out.push_str(&format!("(インポート \"{}\")\n", import.path));
    }
    if !forms.imports.is_empty() {
        out.push('\n');
    }

    for alias in &forms.aliases {
        out.push_str(&format!(
            "(同義語 :別名 {} :正規 {})\n",
            alias.alias, alias.canonical
        ));
    }
    if !forms.aliases.is_empty() {
        out.push('\n');
    }

    for sort in &forms.sorts {
        out.push_str(&format!("(型 {})\n", sort.name));
    }
    if !forms.sorts.is_empty() {
        out.push('\n');
    }

    for data in &forms.data_decls {
        let ctors = data
            .constructors
            .iter()
            .map(|ctor| {
                let fields = ctor
                    .fields
                    .iter()
                    .map(render_type)
                    .collect::<Vec<_>>()
                    .join(" ");
                if fields.is_empty() {
                    format!("({})", ctor.name)
                } else {
                    format!("({} {})", ctor.name, fields)
                }
            })
            .collect::<Vec<_>>()
            .join(" ");
        out.push_str(&format!(
            "(データ {} :コンストラクタ ({}))\n",
            data.name, ctors
        ));
    }
    if !forms.data_decls.is_empty() {
        out.push('\n');
    }

    for relation in &forms.relations {
        out.push_str(&format!(
            "(関係 {} :引数 ({}))\n",
            relation.name,
            relation.arg_sorts.join(" ")
        ));
    }
    if !forms.relations.is_empty() {
        out.push('\n');
    }

    for fact in &forms.facts {
        let terms = fact
            .terms
            .iter()
            .map(render_logic_term)
            .collect::<Vec<_>>()
            .join(" ");
        out.push_str(&format!("(事実 {} :項 ({}))\n", fact.name, terms));
    }
    if !forms.facts.is_empty() {
        out.push('\n');
    }

    for rule in &forms.rules {
        out.push_str(&format!(
            "(規則 :頭 {} :本体 {})\n",
            render_atom_rule(&rule.head),
            render_formula_rule(&rule.body)
        ));
    }
    if !forms.rules.is_empty() {
        out.push('\n');
    }

    for assertion in &forms.asserts {
        let params = assertion
            .params
            .iter()
            .map(|p| format!("({} {})", p.name, render_type(&p.ty)))
            .collect::<Vec<_>>()
            .join(" ");
        out.push_str(&format!(
            "(検証 {} :引数 ({}) :式 {})\n",
            assertion.name,
            params,
            render_formula_refine(&assertion.formula)
        ));
    }
    if !forms.asserts.is_empty() {
        out.push('\n');
    }

    for universe in &forms.universes {
        let values = universe
            .values
            .iter()
            .map(render_logic_term)
            .collect::<Vec<_>>()
            .join(" ");
        out.push_str(&format!("(宇宙 {} :値 ({}))\n", universe.ty_name, values));
    }
    if !forms.universes.is_empty() {
        out.push('\n');
    }

    for defn in &forms.defns {
        let params = defn
            .params
            .iter()
            .map(|p| format!("({} {})", p.name, render_type(&p.ty)))
            .collect::<Vec<_>>()
            .join(" ");
        out.push_str(&format!(
            "(関数 {}\n  :引数 ({})\n  :戻り {}\n  :本体 {})\n",
            defn.name,
            params,
            render_type(&defn.ret_type),
            render_expr(&defn.body)
        ));
    }
}

fn render_type(ty: &Type) -> String {
    match ty {
        Type::Bool => "Bool".to_string(),
        Type::Int => "Int".to_string(),
        Type::Symbol => "Symbol".to_string(),
        Type::Domain(name) => name.clone(),
        Type::Adt(name) => format!("(Adt {name})"),
        Type::Fun(args, ret) => format!(
            "(-> ({}) {})",
            args.iter().map(render_type).collect::<Vec<_>>().join(" "),
            render_type(ret)
        ),
        Type::Refine { var, base, formula } => format!(
            "(Refine {} {} {})",
            var,
            render_type(base),
            render_formula_refine(formula)
        ),
    }
}

fn render_formula_rule(formula: &Formula) -> String {
    match formula {
        Formula::True => "true".to_string(),
        Formula::Atom(atom) => render_atom_rule(atom),
        Formula::And(items) => format!(
            "(and {})",
            items
                .iter()
                .map(render_formula_rule)
                .collect::<Vec<_>>()
                .join(" ")
        ),
        Formula::Not(inner) => format!("(not {})", render_formula_rule(inner)),
    }
}

fn render_formula_refine(formula: &Formula) -> String {
    match formula {
        Formula::True => "true".to_string(),
        Formula::Atom(atom) => render_atom_refine(atom),
        Formula::And(items) => format!(
            "(and {})",
            items
                .iter()
                .map(render_formula_refine)
                .collect::<Vec<_>>()
                .join(" ")
        ),
        Formula::Not(inner) => format!("(not {})", render_formula_refine(inner)),
    }
}

fn render_atom_rule(atom: &Atom) -> String {
    if atom.terms.is_empty() {
        format!("({})", atom.pred)
    } else {
        format!(
            "({} {})",
            atom.pred,
            atom.terms
                .iter()
                .map(render_logic_term_rule)
                .collect::<Vec<_>>()
                .join(" ")
        )
    }
}

fn render_atom_refine(atom: &Atom) -> String {
    if atom.terms.is_empty() {
        format!("({})", atom.pred)
    } else {
        format!(
            "({} {})",
            atom.pred,
            atom.terms
                .iter()
                .map(render_logic_term_refine)
                .collect::<Vec<_>>()
                .join(" ")
        )
    }
}

fn render_logic_term_rule(term: &LogicTerm) -> String {
    match term {
        LogicTerm::Var(v) => format!("?{v}"),
        LogicTerm::Symbol(s) => s.clone(),
        LogicTerm::Int(i) => i.to_string(),
        LogicTerm::Bool(b) => b.to_string(),
        LogicTerm::Ctor { name, args } => {
            if args.is_empty() {
                format!("({name})")
            } else {
                format!(
                    "({} {})",
                    name,
                    args.iter()
                        .map(render_logic_term_rule)
                        .collect::<Vec<_>>()
                        .join(" ")
                )
            }
        }
    }
}

fn render_logic_term_refine(term: &LogicTerm) -> String {
    match term {
        LogicTerm::Var(v) => v.clone(),
        LogicTerm::Symbol(s) => s.clone(),
        LogicTerm::Int(i) => i.to_string(),
        LogicTerm::Bool(b) => b.to_string(),
        LogicTerm::Ctor { name, args } => {
            if args.is_empty() {
                format!("({name})")
            } else {
                format!(
                    "({} {})",
                    name,
                    args.iter()
                        .map(render_logic_term_refine)
                        .collect::<Vec<_>>()
                        .join(" ")
                )
            }
        }
    }
}

fn render_logic_term(term: &LogicTerm) -> String {
    match term {
        LogicTerm::Var(v) => v.clone(),
        LogicTerm::Symbol(s) => s.clone(),
        LogicTerm::Int(i) => i.to_string(),
        LogicTerm::Bool(b) => b.to_string(),
        LogicTerm::Ctor { name, args } => {
            if args.is_empty() {
                format!("({name})")
            } else {
                format!(
                    "({} {})",
                    name,
                    args.iter()
                        .map(render_logic_term)
                        .collect::<Vec<_>>()
                        .join(" ")
                )
            }
        }
    }
}

fn render_expr(expr: &Expr) -> String {
    match expr {
        Expr::Var { name, .. } => name.clone(),
        Expr::Symbol { value, .. } => value.clone(),
        Expr::Int { value, .. } => value.to_string(),
        Expr::Bool { value, .. } => value.to_string(),
        Expr::Call { name, args, .. } => {
            if args.is_empty() {
                format!("({name})")
            } else {
                format!(
                    "({} {})",
                    name,
                    args.iter().map(render_expr).collect::<Vec<_>>().join(" ")
                )
            }
        }
        Expr::Let { bindings, body, .. } => format!(
            "(let ({}) {})",
            bindings
                .iter()
                .map(|(name, expr, _)| format!("({} {})", name, render_expr(expr)))
                .collect::<Vec<_>>()
                .join(" "),
            render_expr(body)
        ),
        Expr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => format!(
            "(if {} {} {})",
            render_expr(cond),
            render_expr(then_branch),
            render_expr(else_branch)
        ),
        Expr::Match {
            scrutinee, arms, ..
        } => format!(
            "(match {} {})",
            render_expr(scrutinee),
            arms.iter()
                .map(|arm| format!(
                    "({} {})",
                    render_pattern(&arm.pattern),
                    render_expr(&arm.body)
                ))
                .collect::<Vec<_>>()
                .join(" ")
        ),
    }
}

fn render_pattern(pattern: &Pattern) -> String {
    match pattern {
        Pattern::Wildcard { .. } => "_".to_string(),
        Pattern::Var { name, .. } => name.clone(),
        Pattern::Symbol { value, .. } => value.clone(),
        Pattern::Int { value, .. } => value.to_string(),
        Pattern::Bool { value, .. } => value.to_string(),
        Pattern::Ctor { name, args, .. } => {
            if args.is_empty() {
                format!("({name})")
            } else {
                format!(
                    "({} {})",
                    name,
                    args.iter()
                        .map(render_pattern)
                        .collect::<Vec<_>>()
                        .join(" ")
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{ConstructorDecl, MatchArm, Param};
    use crate::diagnostics::Span;

    fn span() -> Span {
        Span {
            start: 0,
            end: 0,
            line: 1,
            column: 1,
            file_id: None,
        }
    }

    #[test]
    fn canonical_top_level_kind_supports_aliases() {
        assert!(matches!(
            canonical_top_level_kind("import"),
            Some(TopLevelKind::Import)
        ));
        assert!(matches!(
            canonical_top_level_kind("インポート"),
            Some(TopLevelKind::Import)
        ));
        assert!(matches!(
            canonical_top_level_kind("alias"),
            Some(TopLevelKind::Alias)
        ));
        assert!(matches!(
            canonical_top_level_kind("同義語"),
            Some(TopLevelKind::Alias)
        ));
        assert!(matches!(
            canonical_top_level_kind("sort"),
            Some(TopLevelKind::Sort)
        ));
        assert!(matches!(
            canonical_top_level_kind("型"),
            Some(TopLevelKind::Sort)
        ));
        assert!(matches!(
            canonical_top_level_kind("data"),
            Some(TopLevelKind::Data)
        ));
        assert!(matches!(
            canonical_top_level_kind("データ"),
            Some(TopLevelKind::Data)
        ));
        assert!(matches!(
            canonical_top_level_kind("relation"),
            Some(TopLevelKind::Relation)
        ));
        assert!(matches!(
            canonical_top_level_kind("関係"),
            Some(TopLevelKind::Relation)
        ));
        assert!(matches!(
            canonical_top_level_kind("fact"),
            Some(TopLevelKind::Fact)
        ));
        assert!(matches!(
            canonical_top_level_kind("事実"),
            Some(TopLevelKind::Fact)
        ));
        assert!(matches!(
            canonical_top_level_kind("rule"),
            Some(TopLevelKind::Rule)
        ));
        assert!(matches!(
            canonical_top_level_kind("規則"),
            Some(TopLevelKind::Rule)
        ));
        assert!(matches!(
            canonical_top_level_kind("assert"),
            Some(TopLevelKind::Assert)
        ));
        assert!(matches!(
            canonical_top_level_kind("検証"),
            Some(TopLevelKind::Assert)
        ));
        assert!(matches!(
            canonical_top_level_kind("universe"),
            Some(TopLevelKind::Universe)
        ));
        assert!(matches!(
            canonical_top_level_kind("宇宙"),
            Some(TopLevelKind::Universe)
        ));
        assert!(matches!(
            canonical_top_level_kind("defn"),
            Some(TopLevelKind::Defn)
        ));
        assert!(matches!(
            canonical_top_level_kind("関数"),
            Some(TopLevelKind::Defn)
        ));
        assert!(canonical_top_level_kind("unknown").is_none());
    }

    #[test]
    fn parse_and_collect_context_assignments_are_stable() {
        assert_eq!(parse_context_marker(" @context: ops "), Some("ops"));
        assert_eq!(parse_context_marker("@context: "), None);
        assert_eq!(parse_context_marker("noop"), None);

        let src = r#"
            ; @context: prelude
            (import "a.dtl")
            (sort Subject)
            (data User (alice) (bob))
            ; @context: app
            (relation allowed (Subject))
            (fact allowed alice)
            (rule (allowed ?x) (and (allowed ?x) true))
            (assert ok ((u Subject)) (and (allowed u) (not false)))
            (universe Subject (alice bob))
            (defn can ((u Subject)) Bool (if true (allowed u) false))
        "#;
        let assignments = collect_context_assignments(src);

        assert_eq!(assignments.block_names, vec!["prelude", "app"]);
        assert_eq!(assignments.imports, vec![Some(0)]);
        assert_eq!(assignments.aliases, Vec::<Option<usize>>::new());
        assert_eq!(assignments.sorts, vec![Some(0)]);
        assert_eq!(assignments.data_decls, vec![Some(0)]);
        assert_eq!(assignments.relations, vec![Some(1)]);
        assert_eq!(assignments.facts, vec![Some(1)]);
        assert_eq!(assignments.rules, vec![Some(1)]);
        assert_eq!(assignments.asserts, vec![Some(1)]);
        assert_eq!(assignments.universes, vec![Some(1)]);
        assert_eq!(assignments.defns, vec![Some(1)]);
    }

    #[test]
    fn consume_head_atom_and_form_end_cover_edge_cases() {
        let mut end_it = "".char_indices().peekable();
        let mut depth = 1usize;
        assert_eq!(consume_head_atom(&mut end_it, &mut depth), "");
        assert_eq!(depth, 1);

        let mut ws_comment_it = "  ; note\nrule ?x)".char_indices().peekable();
        depth = 1;
        assert_eq!(consume_head_atom(&mut ws_comment_it, &mut depth), "rule");
        assert_eq!(depth, 1);

        let mut open_it = "(nested".char_indices().peekable();
        depth = 1;
        assert_eq!(consume_head_atom(&mut open_it, &mut depth), "");
        assert_eq!(depth, 2);

        let mut close_it = ")".char_indices().peekable();
        depth = 1;
        assert_eq!(consume_head_atom(&mut close_it, &mut depth), "");
        assert_eq!(depth, 0);

        let mut consume_it = " foo ; cmt\n(bar)) trailing".char_indices().peekable();
        depth = 1;
        consume_to_form_end(&mut consume_it, &mut depth);
        assert_eq!(depth, 0);
    }

    #[test]
    fn render_helpers_cover_all_variants() {
        let atom_rule = Atom {
            pred: "p".to_string(),
            terms: vec![
                LogicTerm::Var("x".to_string()),
                LogicTerm::Symbol("alice".to_string()),
                LogicTerm::Int(42),
                LogicTerm::Bool(true),
                LogicTerm::Ctor {
                    name: "cons".to_string(),
                    args: vec![LogicTerm::Ctor {
                        name: "nil".to_string(),
                        args: vec![],
                    }],
                },
            ],
        };

        let formula = Formula::And(vec![
            Formula::True,
            Formula::Atom(atom_rule.clone()),
            Formula::Not(Box::new(Formula::Atom(Atom {
                pred: "q".to_string(),
                terms: vec![],
            }))),
        ]);

        let ty = Type::Refine {
            var: "x".to_string(),
            base: Box::new(Type::Fun(
                vec![
                    Type::Bool,
                    Type::Int,
                    Type::Symbol,
                    Type::Domain("User".to_string()),
                    Type::Adt("Tree".to_string()),
                ],
                Box::new(Type::Bool),
            )),
            formula: formula.clone(),
        };

        let rendered_type = render_type(&ty);
        assert!(rendered_type.contains("(Refine x"));
        assert!(rendered_type.contains("(-> (Bool Int Symbol User (Adt Tree)) Bool)"));

        let rendered_formula_rule = render_formula_rule(&formula);
        assert!(rendered_formula_rule.contains("(and"));
        assert!(rendered_formula_rule.contains("(not (q))"));
        assert!(rendered_formula_rule.contains("?x"));

        let rendered_formula_refine = render_formula_refine(&formula);
        assert!(rendered_formula_refine.contains("(and"));
        assert!(rendered_formula_refine.contains("(not (q))"));
        assert!(rendered_formula_refine.contains("x"));

        assert_eq!(
            render_atom_rule(&Atom {
                pred: "z".to_string(),
                terms: vec![],
            }),
            "(z)"
        );
        assert_eq!(
            render_atom_refine(&Atom {
                pred: "z".to_string(),
                terms: vec![],
            }),
            "(z)"
        );

        let term_ctor_no_args = LogicTerm::Ctor {
            name: "nil".to_string(),
            args: vec![],
        };
        assert_eq!(render_logic_term_rule(&term_ctor_no_args), "(nil)");
        assert_eq!(render_logic_term_refine(&term_ctor_no_args), "(nil)");
        assert_eq!(render_logic_term(&term_ctor_no_args), "(nil)");
    }

    #[test]
    fn render_expr_and_pattern_cover_all_variants() {
        let call0 = Expr::Call {
            name: "f0".to_string(),
            args: vec![],
            span: span(),
        };
        let call1 = Expr::Call {
            name: "f1".to_string(),
            args: vec![Expr::Var {
                name: "x".to_string(),
                span: span(),
            }],
            span: span(),
        };

        assert_eq!(
            render_expr(&Expr::Var {
                name: "v".to_string(),
                span: span(),
            }),
            "v"
        );
        assert_eq!(
            render_expr(&Expr::Symbol {
                value: "sym".to_string(),
                span: span(),
            }),
            "sym"
        );
        assert_eq!(
            render_expr(&Expr::Int {
                value: 7,
                span: span(),
            }),
            "7"
        );
        assert_eq!(
            render_expr(&Expr::Bool {
                value: false,
                span: span(),
            }),
            "false"
        );
        assert_eq!(render_expr(&call0), "(f0)");
        assert_eq!(render_expr(&call1), "(f1 x)");

        let let_expr = Expr::Let {
            bindings: vec![(
                "a".to_string(),
                Expr::Int {
                    value: 1,
                    span: span(),
                },
                span(),
            )],
            body: Box::new(Expr::Var {
                name: "a".to_string(),
                span: span(),
            }),
            span: span(),
        };
        assert_eq!(render_expr(&let_expr), "(let ((a 1)) a)");

        let if_expr = Expr::If {
            cond: Box::new(Expr::Bool {
                value: true,
                span: span(),
            }),
            then_branch: Box::new(Expr::Int {
                value: 1,
                span: span(),
            }),
            else_branch: Box::new(Expr::Int {
                value: 0,
                span: span(),
            }),
            span: span(),
        };
        assert_eq!(render_expr(&if_expr), "(if true 1 0)");

        let patterns = [
            Pattern::Wildcard { span: span() },
            Pattern::Var {
                name: "v".to_string(),
                span: span(),
            },
            Pattern::Symbol {
                value: "alice".to_string(),
                span: span(),
            },
            Pattern::Int {
                value: 3,
                span: span(),
            },
            Pattern::Bool {
                value: true,
                span: span(),
            },
            Pattern::Ctor {
                name: "node".to_string(),
                args: vec![
                    Pattern::Ctor {
                        name: "leaf".to_string(),
                        args: vec![],
                        span: span(),
                    },
                    Pattern::Var {
                        name: "tail".to_string(),
                        span: span(),
                    },
                ],
                span: span(),
            },
        ];
        assert_eq!(render_pattern(&patterns[0]), "_");
        assert_eq!(render_pattern(&patterns[1]), "v");
        assert_eq!(render_pattern(&patterns[2]), "alice");
        assert_eq!(render_pattern(&patterns[3]), "3");
        assert_eq!(render_pattern(&patterns[4]), "true");
        assert_eq!(render_pattern(&patterns[5]), "(node (leaf) tail)");

        let match_expr = Expr::Match {
            scrutinee: Box::new(Expr::Var {
                name: "xs".to_string(),
                span: span(),
            }),
            arms: vec![
                MatchArm {
                    pattern: Pattern::Ctor {
                        name: "leaf".to_string(),
                        args: vec![],
                        span: span(),
                    },
                    body: Expr::Int {
                        value: 0,
                        span: span(),
                    },
                    span: span(),
                },
                MatchArm {
                    pattern: Pattern::Ctor {
                        name: "node".to_string(),
                        args: vec![Pattern::Var {
                            name: "n".to_string(),
                            span: span(),
                        }],
                        span: span(),
                    },
                    body: Expr::Var {
                        name: "n".to_string(),
                        span: span(),
                    },
                    span: span(),
                },
            ],
            span: span(),
        };
        assert_eq!(
            render_expr(&match_expr),
            "(match xs ((leaf) 0) ((node n) n))"
        );
    }

    #[test]
    fn render_forms_and_context_blocks_work_for_mixed_content() {
        let mut forms = ContextForms {
            imports: vec![ImportDecl {
                path: "zeta.dtl".to_string(),
                span: span(),
            }],
            aliases: vec![AliasDecl {
                alias: "閲覧".to_string(),
                canonical: "read".to_string(),
                span: span(),
            }],
            sorts: vec![SortDecl {
                name: "Subject".to_string(),
                span: span(),
            }],
            data_decls: vec![DataDecl {
                name: "Node".to_string(),
                constructors: vec![
                    ConstructorDecl {
                        name: "leaf".to_string(),
                        fields: vec![],
                        span: span(),
                    },
                    ConstructorDecl {
                        name: "cons".to_string(),
                        fields: vec![Type::Int],
                        span: span(),
                    },
                ],
                span: span(),
            }],
            relations: vec![RelationDecl {
                name: "allowed".to_string(),
                arg_sorts: vec!["Subject".to_string()],
                span: span(),
            }],
            facts: vec![Fact {
                name: "allowed".to_string(),
                terms: vec![LogicTerm::Symbol("alice".to_string())],
                span: span(),
            }],
            rules: vec![Rule {
                head: Atom {
                    pred: "allowed".to_string(),
                    terms: vec![LogicTerm::Var("x".to_string())],
                },
                body: Formula::Atom(Atom {
                    pred: "allowed".to_string(),
                    terms: vec![LogicTerm::Var("x".to_string())],
                }),
                span: span(),
            }],
            asserts: vec![AssertDecl {
                name: "ok".to_string(),
                params: vec![Param {
                    name: "u".to_string(),
                    ty: Type::Domain("Subject".to_string()),
                    span: span(),
                }],
                formula: Formula::Atom(Atom {
                    pred: "allowed".to_string(),
                    terms: vec![LogicTerm::Var("u".to_string())],
                }),
                span: span(),
            }],
            universes: vec![UniverseDecl {
                ty_name: "Subject".to_string(),
                values: vec![LogicTerm::Symbol("alice".to_string())],
                span: span(),
            }],
            defns: vec![Defn {
                name: "id".to_string(),
                params: vec![Param {
                    name: "x".to_string(),
                    ty: Type::Int,
                    span: span(),
                }],
                ret_type: Type::Int,
                body: Expr::Var {
                    name: "x".to_string(),
                    span: span(),
                },
                span: span(),
            }],
        };
        forms.sort_for_render();

        let mut rendered = String::new();
        render_forms(&forms, &mut rendered);
        assert!(rendered.contains("(インポート \"zeta.dtl\")"));
        assert!(rendered.contains("(同義語 :別名 閲覧 :正規 read)"));
        assert!(rendered.contains("(型 Subject)"));
        assert!(rendered.contains("(データ Node :コンストラクタ ((leaf) (cons Int)))"));
        assert!(rendered.contains("(関係 allowed :引数 (Subject))"));
        assert!(rendered.contains("(事実 allowed :項 (alice))"));
        assert!(rendered.contains("(規則 :頭 (allowed ?x) :本体 (allowed ?x))"));
        assert!(rendered.contains("(検証 ok :引数 ((u Subject)) :式 (allowed u))"));
        assert!(rendered.contains("(宇宙 Subject :値 (alice))"));
        assert!(rendered.contains("(関数 id"));

        let program = Program {
            imports: forms.imports.clone(),
            aliases: forms.aliases.clone(),
            sorts: forms.sorts.clone(),
            data_decls: forms.data_decls.clone(),
            relations: forms.relations.clone(),
            facts: forms.facts.clone(),
            rules: forms.rules.clone(),
            asserts: forms.asserts.clone(),
            universes: forms.universes.clone(),
            defns: forms.defns.clone(),
        };
        let src = r#"
            ; @context: pre
            (import "zeta.dtl")
            (sort Subject)
            ; @context: app
            (relation allowed (Subject))
            (fact allowed alice)
            (rule (allowed ?x) (allowed ?x))
            (assert ok ((u Subject)) (allowed u))
            (universe Subject (alice))
            (defn id ((x Int)) Int x)
            (data Node (leaf) (cons Int))
        "#;
        let mut out = String::new();
        render_with_context_blocks(program, src, &mut out);
        assert!(out.contains("; @context: pre"));
        assert!(out.contains("; @context: app"));
    }

    #[test]
    fn format_source_supports_no_context_mode_and_parse_errors() {
        let src = r#"
            (relation z (Subject))
            (sort Subject)
            (data Choice (a) (b))
        "#;
        let rendered = format_source(
            src,
            FormatOptions {
                preserve_context: false,
            },
        )
        .expect("format");

        let sort_pos = rendered.find("(型 Subject)").expect("sort");
        let relation_pos = rendered.find("(関係 z :引数 (Subject))").expect("relation");
        assert!(sort_pos < relation_pos);

        let err = format_source("(", FormatOptions::default()).expect_err("parse error");
        assert!(!err.is_empty());
    }
}
