use crate::ast::{Expr, Pattern};
use crate::diagnostics::Diagnostic;
use crate::parser::parse_program;
use crate::types::{Atom, Formula, LogicTerm, Type};

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
    let mut program = parse_program(src)?;

    program.imports.sort_by(|a, b| a.path.cmp(&b.path));
    program.sorts.sort_by(|a, b| a.name.cmp(&b.name));
    program.data_decls.sort_by(|a, b| a.name.cmp(&b.name));
    program.relations.sort_by(|a, b| a.name.cmp(&b.name));
    program.universes.sort_by(|a, b| a.ty_name.cmp(&b.ty_name));

    let mut out = String::new();
    out.push_str("; syntax: surface\n");

    if options.preserve_context {
        let contexts = collect_context_markers(src);
        if !contexts.is_empty() {
            out.push_str(&format!("; @context: {}\n\n", contexts[0]));
        } else {
            out.push_str("; @context: default\n\n");
        }
    }

    for import in &program.imports {
        out.push_str(&format!("(インポート \"{}\")\n", import.path));
    }
    if !program.imports.is_empty() {
        out.push('\n');
    }

    for sort in &program.sorts {
        out.push_str(&format!("(型 {})\n", sort.name));
    }
    if !program.sorts.is_empty() {
        out.push('\n');
    }

    for data in &program.data_decls {
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
    if !program.data_decls.is_empty() {
        out.push('\n');
    }

    for relation in &program.relations {
        out.push_str(&format!(
            "(関係 {} :引数 ({}))\n",
            relation.name,
            relation.arg_sorts.join(" ")
        ));
    }
    if !program.relations.is_empty() {
        out.push('\n');
    }

    for fact in &program.facts {
        let terms = fact
            .terms
            .iter()
            .map(render_logic_term)
            .collect::<Vec<_>>()
            .join(" ");
        out.push_str(&format!("(事実 {} :項 ({}))\n", fact.name, terms));
    }
    if !program.facts.is_empty() {
        out.push('\n');
    }

    for rule in &program.rules {
        out.push_str(&format!(
            "(規則 :頭 {} :本体 {})\n",
            render_atom_rule(&rule.head),
            render_formula_rule(&rule.body)
        ));
    }
    if !program.rules.is_empty() {
        out.push('\n');
    }

    for assertion in &program.asserts {
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
    if !program.asserts.is_empty() {
        out.push('\n');
    }

    for universe in &program.universes {
        let values = universe
            .values
            .iter()
            .map(render_logic_term)
            .collect::<Vec<_>>()
            .join(" ");
        out.push_str(&format!("(宇宙 {} :値 ({}))\n", universe.ty_name, values));
    }
    if !program.universes.is_empty() {
        out.push('\n');
    }

    for defn in &program.defns {
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

    Ok(out.trim_end().to_string() + "\n")
}

fn collect_context_markers(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in src.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with(';') {
            continue;
        }
        let body = trimmed.trim_start_matches(';').trim();
        if let Some(ctx) = body.strip_prefix("@context:") {
            let value = ctx.trim();
            if !value.is_empty() {
                out.push(value.to_string());
            }
        }
    }
    out
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
