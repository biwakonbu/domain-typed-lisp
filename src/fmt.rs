use crate::ast::{
    AssertDecl, DataDecl, Defn, Expr, Fact, ImportDecl, Pattern, Program, RelationDecl, Rule,
    SortDecl, UniverseDecl,
};
use crate::diagnostics::Diagnostic;
use crate::parser::parse_program;
use crate::types::{Atom, Formula, LogicTerm, Type};
use std::iter::Peekable;
use std::str::CharIndices;

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
        self.sorts.sort_by(|a, b| a.name.cmp(&b.name));
        self.data_decls.sort_by(|a, b| a.name.cmp(&b.name));
        self.relations.sort_by(|a, b| a.name.cmp(&b.name));
        self.universes.sort_by(|a, b| a.ty_name.cmp(&b.ty_name));
    }
}

fn render_with_context_blocks(program: Program, src: &str, out: &mut String) {
    let Program {
        imports,
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
        if let Some(block_idx) = contexts.get(idx).copied().flatten() {
            if let Some((_, forms)) = blocks.get_mut(block_idx) {
                forms.imports.push(item);
                continue;
            }
        }
        prelude.imports.push(item);
    }
}

fn assign_sorts(
    items: Vec<SortDecl>,
    contexts: &[Option<usize>],
    prelude: &mut ContextForms,
    blocks: &mut [(String, ContextForms)],
) {
    for (idx, item) in items.into_iter().enumerate() {
        if let Some(block_idx) = contexts.get(idx).copied().flatten() {
            if let Some((_, forms)) = blocks.get_mut(block_idx) {
                forms.sorts.push(item);
                continue;
            }
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
        if let Some(block_idx) = contexts.get(idx).copied().flatten() {
            if let Some((_, forms)) = blocks.get_mut(block_idx) {
                forms.data_decls.push(item);
                continue;
            }
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
        if let Some(block_idx) = contexts.get(idx).copied().flatten() {
            if let Some((_, forms)) = blocks.get_mut(block_idx) {
                forms.relations.push(item);
                continue;
            }
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
        if let Some(block_idx) = contexts.get(idx).copied().flatten() {
            if let Some((_, forms)) = blocks.get_mut(block_idx) {
                forms.facts.push(item);
                continue;
            }
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
        if let Some(block_idx) = contexts.get(idx).copied().flatten() {
            if let Some((_, forms)) = blocks.get_mut(block_idx) {
                forms.rules.push(item);
                continue;
            }
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
        if let Some(block_idx) = contexts.get(idx).copied().flatten() {
            if let Some((_, forms)) = blocks.get_mut(block_idx) {
                forms.asserts.push(item);
                continue;
            }
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
        if let Some(block_idx) = contexts.get(idx).copied().flatten() {
            if let Some((_, forms)) = blocks.get_mut(block_idx) {
                forms.universes.push(item);
                continue;
            }
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
        if let Some(block_idx) = contexts.get(idx).copied().flatten() {
            if let Some((_, forms)) = blocks.get_mut(block_idx) {
                forms.defns.push(item);
                continue;
            }
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
