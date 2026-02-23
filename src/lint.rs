use std::collections::{HashMap, HashSet};

use crate::ast::{Defn, Expr, Pattern, Program};
use crate::diagnostics::Span;
use crate::name_resolve::resolve_program;
use crate::types::{Atom, Formula, LogicTerm, Type};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LintSeverity {
    Warning,
}

impl LintSeverity {
    pub fn as_str(self) -> &'static str {
        match self {
            LintSeverity::Warning => "warning",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LintDiagnostic {
    pub severity: LintSeverity,
    pub lint_code: &'static str,
    pub category: &'static str,
    pub message: String,
    pub source: Option<String>,
    pub span: Option<Span>,
    pub confidence: Option<f64>,
}

impl LintDiagnostic {
    fn warning(
        lint_code: &'static str,
        category: &'static str,
        message: impl Into<String>,
        span: Option<Span>,
        confidence: Option<f64>,
    ) -> Self {
        let source = span.as_ref().and_then(|s| s.file_id.clone());
        Self {
            severity: LintSeverity::Warning,
            lint_code,
            category,
            message: message.into(),
            source,
            span,
            confidence,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct LintOptions {
    pub semantic_dup: bool,
}

pub fn lint_program(program: &Program, options: LintOptions) -> Vec<LintDiagnostic> {
    let mut out = Vec::new();

    // 既存解決エラーがある場合は lint を進めてもノイズになるため打ち切る。
    if !resolve_program(program).is_empty() {
        return out;
    }

    out.extend(lint_exact_duplicates(program));
    out.extend(lint_unused_declarations(program));

    if options.semantic_dup {
        out.extend(lint_semantic_duplicates(program));
    }

    out
}

fn lint_exact_duplicates(program: &Program) -> Vec<LintDiagnostic> {
    let mut out = Vec::new();

    let mut seen_fact: HashMap<String, Span> = HashMap::new();
    for fact in &program.facts {
        let key = normalize_fact(fact);
        if let Some(prev) = seen_fact.get(&key) {
            out.push(LintDiagnostic::warning(
                "L-DUP-EXACT",
                "duplicate",
                format!(
                    "重複した fact です: {}（最初の定義: {}:{}）",
                    fact.name, prev.line, prev.column
                ),
                Some(fact.span.clone()),
                None,
            ));
        } else {
            seen_fact.insert(key, fact.span.clone());
        }
    }

    let mut seen_rule: HashMap<String, Span> = HashMap::new();
    for rule in &program.rules {
        let key = normalize_rule(rule);
        if let Some(prev) = seen_rule.get(&key) {
            out.push(LintDiagnostic::warning(
                "L-DUP-EXACT",
                "duplicate",
                format!(
                    "重複した rule です: {}（最初の定義: {}:{}）",
                    rule.head.pred, prev.line, prev.column
                ),
                Some(rule.span.clone()),
                None,
            ));
        } else {
            seen_rule.insert(key, rule.span.clone());
        }
    }

    let mut seen_assert: HashMap<String, (String, Span)> = HashMap::new();
    for assertion in &program.asserts {
        let key = normalize_assert(assertion);
        if let Some((prev_name, prev_span)) = seen_assert.get(&key) {
            out.push(LintDiagnostic::warning(
                "L-DUP-EXACT",
                "duplicate",
                format!(
                    "重複した assert です: {} と {}（最初の定義: {}:{}）",
                    prev_name, assertion.name, prev_span.line, prev_span.column
                ),
                Some(assertion.span.clone()),
                None,
            ));
        } else {
            seen_assert.insert(key, (assertion.name.clone(), assertion.span.clone()));
        }
    }

    let mut seen_defn: HashMap<String, (String, Span)> = HashMap::new();
    for defn in &program.defns {
        let key = normalize_defn(defn);
        if let Some((prev_name, prev_span)) = seen_defn.get(&key) {
            out.push(LintDiagnostic::warning(
                "L-DUP-EXACT",
                "duplicate",
                format!(
                    "重複した defn です: {} と {}（最初の定義: {}:{}）",
                    prev_name, defn.name, prev_span.line, prev_span.column
                ),
                Some(defn.span.clone()),
                None,
            ));
        } else {
            seen_defn.insert(key, (defn.name.clone(), defn.span.clone()));
        }
    }

    out
}

fn lint_semantic_duplicates(program: &Program) -> Vec<LintDiagnostic> {
    let mut out = Vec::new();

    if let Some(missing) = missing_universe_types(program) {
        out.push(LintDiagnostic::warning(
            "L-DUP-SKIP-UNIVERSE",
            "duplicate",
            format!(
                "semantic duplicate 判定をスキップしました: universe 不足 ({})",
                missing.join(", ")
            ),
            None,
            None,
        ));
        return out;
    }

    // 近似判定: 形状（述語名/関数名を無視したスケルトン）が一致する場合に maybe とする。
    let mut assert_buckets: HashMap<String, Vec<usize>> = HashMap::new();
    for (idx, assertion) in program.asserts.iter().enumerate() {
        let sig = assertion
            .params
            .iter()
            .map(|p| normalize_type(&p.ty))
            .collect::<Vec<_>>()
            .join(",");
        assert_buckets.entry(sig).or_default().push(idx);
    }
    for indices in assert_buckets.values() {
        for i in 0..indices.len() {
            for j in (i + 1)..indices.len() {
                let a = &program.asserts[indices[i]];
                let b = &program.asserts[indices[j]];
                if normalize_assert(a) == normalize_assert(b) {
                    continue;
                }
                if skeleton_formula(&a.formula) == skeleton_formula(&b.formula) {
                    out.push(LintDiagnostic::warning(
                        "L-DUP-MAYBE",
                        "duplicate",
                        format!(
                            "assert {} と {} は論理同値の可能性があります",
                            a.name, b.name
                        ),
                        Some(b.span.clone()),
                        Some(0.55),
                    ));
                }
            }
        }
    }

    let mut defn_buckets: HashMap<String, Vec<usize>> = HashMap::new();
    for (idx, defn) in program.defns.iter().enumerate() {
        let sig = format!(
            "{}->{}",
            defn.params
                .iter()
                .map(|p| normalize_type(&p.ty))
                .collect::<Vec<_>>()
                .join(","),
            normalize_type(&defn.ret_type)
        );
        defn_buckets.entry(sig).or_default().push(idx);
    }
    for indices in defn_buckets.values() {
        for i in 0..indices.len() {
            for j in (i + 1)..indices.len() {
                let a = &program.defns[indices[i]];
                let b = &program.defns[indices[j]];
                if normalize_defn(a) == normalize_defn(b) {
                    continue;
                }
                if skeleton_expr(&a.body) == skeleton_expr(&b.body) {
                    out.push(LintDiagnostic::warning(
                        "L-DUP-MAYBE",
                        "duplicate",
                        format!("defn {} と {} は等価実装の可能性があります", a.name, b.name),
                        Some(b.span.clone()),
                        Some(0.55),
                    ));
                }
            }
        }
    }

    let mut rule_buckets: HashMap<String, Vec<usize>> = HashMap::new();
    for (idx, rule) in program.rules.iter().enumerate() {
        let head_sig = format!("{}:{}", rule.head.pred, rule.head.terms.len());
        rule_buckets.entry(head_sig).or_default().push(idx);
    }
    for indices in rule_buckets.values() {
        for i in 0..indices.len() {
            for j in (i + 1)..indices.len() {
                let a = &program.rules[indices[i]];
                let b = &program.rules[indices[j]];
                if normalize_rule(a) == normalize_rule(b) {
                    continue;
                }
                if skeleton_formula(&a.body) == skeleton_formula(&b.body) {
                    out.push(LintDiagnostic::warning(
                        "L-DUP-MAYBE",
                        "duplicate",
                        format!("rule {} の定義が論理同値の可能性があります", a.head.pred),
                        Some(b.span.clone()),
                        Some(0.55),
                    ));
                }
            }
        }
    }

    out
}

fn missing_universe_types(program: &Program) -> Option<Vec<String>> {
    let declared = program
        .universes
        .iter()
        .map(|u| u.ty_name.clone())
        .collect::<HashSet<_>>();
    let mut required = HashSet::new();
    for assertion in &program.asserts {
        for p in &assertion.params {
            collect_type_keys(&p.ty, &mut required);
        }
    }
    for defn in &program.defns {
        for p in &defn.params {
            collect_type_keys(&p.ty, &mut required);
        }
    }

    let mut missing = required
        .into_iter()
        .filter(|k| !declared.contains(k))
        .collect::<Vec<_>>();
    missing.sort();
    if missing.is_empty() {
        None
    } else {
        Some(missing)
    }
}

fn collect_type_keys(ty: &Type, out: &mut HashSet<String>) {
    match ty {
        Type::Bool => {
            out.insert("Bool".to_string());
        }
        Type::Int => {
            out.insert("Int".to_string());
        }
        Type::Symbol => {
            out.insert("Symbol".to_string());
        }
        Type::Domain(name) | Type::Adt(name) => {
            out.insert(name.clone());
        }
        Type::Fun(args, ret) => {
            for arg in args {
                collect_type_keys(arg, out);
            }
            collect_type_keys(ret, out);
        }
        Type::Refine { base, .. } => collect_type_keys(base, out),
    }
}

fn lint_unused_declarations(program: &Program) -> Vec<LintDiagnostic> {
    let mut out = Vec::new();

    let relation_names = program
        .relations
        .iter()
        .map(|r| r.name.clone())
        .collect::<HashSet<_>>();
    let defn_names = program
        .defns
        .iter()
        .map(|d| d.name.clone())
        .collect::<HashSet<_>>();

    let mut used_relations = HashSet::new();
    let mut used_defns = HashSet::new();
    let mut used_types = HashSet::new();
    let mut used_universe = HashSet::new();

    for relation in &program.relations {
        for arg in &relation.arg_sorts {
            used_types.insert(arg.clone());
        }
    }
    for fact in &program.facts {
        used_relations.insert(fact.name.clone());
    }
    for rule in &program.rules {
        used_relations.insert(rule.head.pred.clone());
        collect_formula_relations(&rule.body, &mut used_relations);
    }
    for assertion in &program.asserts {
        for param in &assertion.params {
            collect_type_names(&param.ty, &mut used_types);
            collect_type_keys(&param.ty, &mut used_universe);
        }
        collect_formula_relations(&assertion.formula, &mut used_relations);
    }
    for defn in &program.defns {
        for param in &defn.params {
            collect_type_names(&param.ty, &mut used_types);
            collect_type_keys(&param.ty, &mut used_universe);
        }
        collect_type_names(&defn.ret_type, &mut used_types);
        collect_expr_calls(
            &defn.body,
            &relation_names,
            &defn_names,
            &mut used_relations,
            &mut used_defns,
        );
    }

    // defn 自身は証明義務対象なので、自己使用として扱う。
    for defn in &program.defns {
        used_defns.insert(defn.name.clone());
    }

    for relation in &program.relations {
        if !used_relations.contains(&relation.name) {
            out.push(LintDiagnostic::warning(
                "L-UNUSED-DECL",
                "unused",
                format!("未使用 relation: {}", relation.name),
                Some(relation.span.clone()),
                None,
            ));
        }
    }
    for defn in &program.defns {
        if !used_defns.contains(&defn.name) {
            out.push(LintDiagnostic::warning(
                "L-UNUSED-DECL",
                "unused",
                format!("未使用 defn: {}", defn.name),
                Some(defn.span.clone()),
                None,
            ));
        }
    }
    for sort in &program.sorts {
        if !used_types.contains(&sort.name) {
            out.push(LintDiagnostic::warning(
                "L-UNUSED-DECL",
                "unused",
                format!("未使用 sort: {}", sort.name),
                Some(sort.span.clone()),
                None,
            ));
        }
    }
    for data in &program.data_decls {
        if !used_types.contains(&data.name) {
            out.push(LintDiagnostic::warning(
                "L-UNUSED-DECL",
                "unused",
                format!("未使用 data: {}", data.name),
                Some(data.span.clone()),
                None,
            ));
        }
    }
    for universe in &program.universes {
        if !used_universe.contains(&universe.ty_name) {
            out.push(LintDiagnostic::warning(
                "L-UNUSED-DECL",
                "unused",
                format!("未使用 universe: {}", universe.ty_name),
                Some(universe.span.clone()),
                None,
            ));
        }
    }

    out
}

fn collect_formula_relations(formula: &Formula, out: &mut HashSet<String>) {
    match formula {
        Formula::True => {}
        Formula::Atom(atom) => {
            out.insert(atom.pred.clone());
        }
        Formula::And(items) => {
            for item in items {
                collect_formula_relations(item, out);
            }
        }
        Formula::Not(inner) => collect_formula_relations(inner, out),
    }
}

fn collect_type_names(ty: &Type, out: &mut HashSet<String>) {
    match ty {
        Type::Bool | Type::Int | Type::Symbol => {}
        Type::Domain(name) | Type::Adt(name) => {
            out.insert(name.clone());
        }
        Type::Fun(args, ret) => {
            for arg in args {
                collect_type_names(arg, out);
            }
            collect_type_names(ret, out);
        }
        Type::Refine { base, .. } => collect_type_names(base, out),
    }
}

fn collect_expr_calls(
    expr: &Expr,
    relation_names: &HashSet<String>,
    defn_names: &HashSet<String>,
    used_relations: &mut HashSet<String>,
    used_defns: &mut HashSet<String>,
) {
    match expr {
        Expr::Var { .. } | Expr::Symbol { .. } | Expr::Int { .. } | Expr::Bool { .. } => {}
        Expr::Call { name, args, .. } => {
            if relation_names.contains(name) {
                used_relations.insert(name.clone());
            }
            if defn_names.contains(name) {
                used_defns.insert(name.clone());
            }
            for arg in args {
                collect_expr_calls(arg, relation_names, defn_names, used_relations, used_defns);
            }
        }
        Expr::Let { bindings, body, .. } => {
            for (_, bexpr, _) in bindings {
                collect_expr_calls(
                    bexpr,
                    relation_names,
                    defn_names,
                    used_relations,
                    used_defns,
                );
            }
            collect_expr_calls(body, relation_names, defn_names, used_relations, used_defns);
        }
        Expr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            collect_expr_calls(cond, relation_names, defn_names, used_relations, used_defns);
            collect_expr_calls(
                then_branch,
                relation_names,
                defn_names,
                used_relations,
                used_defns,
            );
            collect_expr_calls(
                else_branch,
                relation_names,
                defn_names,
                used_relations,
                used_defns,
            );
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            collect_expr_calls(
                scrutinee,
                relation_names,
                defn_names,
                used_relations,
                used_defns,
            );
            for arm in arms {
                collect_expr_calls(
                    &arm.body,
                    relation_names,
                    defn_names,
                    used_relations,
                    used_defns,
                );
            }
        }
    }
}

#[derive(Default)]
struct AlphaState {
    map: HashMap<String, String>,
    next: usize,
}

impl AlphaState {
    fn name_for(&mut self, raw: &str, prefix: &str) -> String {
        if let Some(name) = self.map.get(raw) {
            return name.clone();
        }
        let current = self.next;
        self.next += 1;
        let name = format!("{prefix}{current}");
        self.map.insert(raw.to_string(), name.clone());
        name
    }
}

fn normalize_fact(fact: &crate::ast::Fact) -> String {
    format!(
        "{}({})",
        fact.name,
        fact.terms
            .iter()
            .map(|t| normalize_logic_term(t, &mut AlphaState::default()))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn normalize_rule(rule: &crate::ast::Rule) -> String {
    let mut alpha = AlphaState::default();
    let head = normalize_atom(&rule.head, &mut alpha);
    let body = normalize_formula(&rule.body, &mut alpha);
    format!("{head}<-{body}")
}

fn normalize_assert(assertion: &crate::ast::AssertDecl) -> String {
    let mut alpha = AlphaState::default();
    let params = assertion
        .params
        .iter()
        .map(|p| {
            let pname = alpha.name_for(&p.name, "p");
            format!("{pname}:{}", normalize_type(&p.ty))
        })
        .collect::<Vec<_>>()
        .join(",");
    let formula = normalize_formula(&assertion.formula, &mut alpha);
    format!("({params})=>{formula}")
}

fn normalize_defn(defn: &Defn) -> String {
    let mut alpha = AlphaState::default();
    let mut env = HashMap::new();
    let params = defn
        .params
        .iter()
        .map(|p| {
            let pname = alpha.name_for(&p.name, "p");
            env.insert(p.name.clone(), pname.clone());
            format!("{pname}:{}", normalize_type(&p.ty))
        })
        .collect::<Vec<_>>()
        .join(",");
    let ret = normalize_type(&defn.ret_type);
    let body = normalize_expr(&defn.body, &mut env, &mut alpha);
    format!("({params})->{ret}:{body}")
}

fn normalize_atom(atom: &Atom, alpha: &mut AlphaState) -> String {
    let terms = atom
        .terms
        .iter()
        .map(|t| normalize_logic_term(t, alpha))
        .collect::<Vec<_>>()
        .join(",");
    format!("{}({terms})", atom.pred)
}

fn normalize_logic_term(term: &LogicTerm, alpha: &mut AlphaState) -> String {
    match term {
        LogicTerm::Var(v) => alpha.name_for(v, "v"),
        LogicTerm::Symbol(s) => format!("'{}'", s),
        LogicTerm::Int(i) => i.to_string(),
        LogicTerm::Bool(b) => b.to_string(),
        LogicTerm::Ctor { name, args } => {
            let inner = args
                .iter()
                .map(|a| normalize_logic_term(a, alpha))
                .collect::<Vec<_>>()
                .join(",");
            format!("({name} {inner})")
        }
    }
}

fn normalize_formula(formula: &Formula, alpha: &mut AlphaState) -> String {
    match formula {
        Formula::True => "true".to_string(),
        Formula::Atom(atom) => normalize_atom(atom, alpha),
        Formula::Not(inner) => format!("(not {})", normalize_formula(inner, alpha)),
        Formula::And(items) => {
            let mut flattened = Vec::new();
            collect_and_terms(items, &mut flattened);
            let mut rendered = flattened
                .iter()
                .map(|f| normalize_formula(f, alpha))
                .collect::<Vec<_>>();
            rendered.sort();
            format!("(and {})", rendered.join(" "))
        }
    }
}

fn collect_and_terms<'a>(items: &'a [Formula], out: &mut Vec<&'a Formula>) {
    for item in items {
        match item {
            Formula::And(inner) => collect_and_terms(inner, out),
            _ => out.push(item),
        }
    }
}

fn normalize_type(ty: &Type) -> String {
    match ty {
        Type::Bool => "Bool".to_string(),
        Type::Int => "Int".to_string(),
        Type::Symbol => "Symbol".to_string(),
        Type::Domain(n) => format!("Domain({n})"),
        Type::Adt(n) => format!("Adt({n})"),
        Type::Fun(args, ret) => format!(
            "(-> ({}) {})",
            args.iter()
                .map(normalize_type)
                .collect::<Vec<_>>()
                .join(" "),
            normalize_type(ret)
        ),
        Type::Refine { var, base, formula } => format!(
            "(Refine {} {} {})",
            var,
            normalize_type(base),
            normalize_formula(formula, &mut AlphaState::default())
        ),
    }
}

fn normalize_expr(
    expr: &Expr,
    env: &mut HashMap<String, String>,
    alpha: &mut AlphaState,
) -> String {
    match expr {
        Expr::Var { name, .. } => env.get(name).cloned().unwrap_or_else(|| name.clone()),
        Expr::Symbol { value, .. } => format!("'{}'", value),
        Expr::Int { value, .. } => value.to_string(),
        Expr::Bool { value, .. } => value.to_string(),
        Expr::Call { name, args, .. } => format!(
            "({} {})",
            name,
            args.iter()
                .map(|a| normalize_expr(a, env, alpha))
                .collect::<Vec<_>>()
                .join(" ")
        ),
        Expr::Let { bindings, body, .. } => {
            let mut local = env.clone();
            let mut rendered = Vec::new();
            for (name, bexpr, _) in bindings {
                let b = normalize_expr(bexpr, &mut local, alpha);
                let renamed = alpha.name_for(name, "l");
                local.insert(name.clone(), renamed.clone());
                rendered.push(format!("({renamed} {b})"));
            }
            let body = normalize_expr(body, &mut local, alpha);
            format!("(let ({}) {body})", rendered.join(" "))
        }
        Expr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => format!(
            "(if {} {} {})",
            normalize_expr(cond, env, alpha),
            normalize_expr(then_branch, env, alpha),
            normalize_expr(else_branch, env, alpha)
        ),
        Expr::Match {
            scrutinee, arms, ..
        } => {
            let scr = normalize_expr(scrutinee, env, alpha);
            let rendered = arms
                .iter()
                .map(|arm| {
                    let mut arm_env = env.clone();
                    let pat = normalize_pattern(&arm.pattern, &mut arm_env, alpha);
                    let body = normalize_expr(&arm.body, &mut arm_env, alpha);
                    format!("({pat} {body})")
                })
                .collect::<Vec<_>>()
                .join(" ");
            format!("(match {scr} {rendered})")
        }
    }
}

fn normalize_pattern(
    pattern: &Pattern,
    env: &mut HashMap<String, String>,
    alpha: &mut AlphaState,
) -> String {
    match pattern {
        Pattern::Wildcard { .. } => "_".to_string(),
        Pattern::Var { name, .. } => {
            let renamed = alpha.name_for(name, "m");
            env.insert(name.clone(), renamed.clone());
            renamed
        }
        Pattern::Symbol { value, .. } => format!("'{}'", value),
        Pattern::Int { value, .. } => value.to_string(),
        Pattern::Bool { value, .. } => value.to_string(),
        Pattern::Ctor { name, args, .. } => {
            let inner = args
                .iter()
                .map(|a| normalize_pattern(a, env, alpha))
                .collect::<Vec<_>>()
                .join(" ");
            format!("({name} {inner})")
        }
    }
}

fn skeleton_formula(formula: &Formula) -> String {
    match formula {
        Formula::True => "true".to_string(),
        Formula::Atom(atom) => {
            let terms = atom
                .terms
                .iter()
                .map(skeleton_logic_term)
                .collect::<Vec<_>>()
                .join(",");
            format!("P({terms})")
        }
        Formula::And(items) => {
            let mut parts = items.iter().map(skeleton_formula).collect::<Vec<_>>();
            parts.sort();
            format!("AND({})", parts.join(","))
        }
        Formula::Not(inner) => format!("NOT({})", skeleton_formula(inner)),
    }
}

fn skeleton_logic_term(term: &LogicTerm) -> String {
    match term {
        LogicTerm::Var(_) => "V".to_string(),
        LogicTerm::Symbol(_) => "S".to_string(),
        LogicTerm::Int(_) => "I".to_string(),
        LogicTerm::Bool(_) => "B".to_string(),
        LogicTerm::Ctor { args, .. } => format!(
            "C({})",
            args.iter()
                .map(skeleton_logic_term)
                .collect::<Vec<_>>()
                .join(",")
        ),
    }
}

fn skeleton_expr(expr: &Expr) -> String {
    match expr {
        Expr::Var { .. } => "V".to_string(),
        Expr::Symbol { .. } => "S".to_string(),
        Expr::Int { .. } => "I".to_string(),
        Expr::Bool { .. } => "B".to_string(),
        Expr::Call { args, .. } => format!(
            "CALL({})",
            args.iter().map(skeleton_expr).collect::<Vec<_>>().join(",")
        ),
        Expr::Let { bindings, body, .. } => format!(
            "LET({};{})",
            bindings
                .iter()
                .map(|(_, e, _)| skeleton_expr(e))
                .collect::<Vec<_>>()
                .join(","),
            skeleton_expr(body)
        ),
        Expr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => format!(
            "IF({},{},{})",
            skeleton_expr(cond),
            skeleton_expr(then_branch),
            skeleton_expr(else_branch)
        ),
        Expr::Match {
            scrutinee, arms, ..
        } => format!(
            "MATCH({};{})",
            skeleton_expr(scrutinee),
            arms.iter()
                .map(|a| format!(
                    "A({},{})",
                    skeleton_pattern(&a.pattern),
                    skeleton_expr(&a.body)
                ))
                .collect::<Vec<_>>()
                .join(",")
        ),
    }
}

fn skeleton_pattern(pattern: &Pattern) -> String {
    match pattern {
        Pattern::Wildcard { .. } => "_".to_string(),
        Pattern::Var { .. } => "V".to_string(),
        Pattern::Symbol { .. } => "S".to_string(),
        Pattern::Int { .. } => "I".to_string(),
        Pattern::Bool { .. } => "B".to_string(),
        Pattern::Ctor { args, .. } => format!(
            "C({})",
            args.iter()
                .map(skeleton_pattern)
                .collect::<Vec<_>>()
                .join(",")
        ),
    }
}
