use std::collections::{BTreeSet, HashMap, HashSet};

use crate::ast::{AssertDecl, Defn, Expr, Param, Pattern, Program, Rule};
use crate::diagnostics::Span;
use crate::logic_engine::{DerivedFacts, KnowledgeBase, Value, solve_facts};
use crate::name_resolve::{normalize_program_aliases, resolve_program};
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

#[derive(Debug, Clone, Copy)]
struct SemanticDupEvidence {
    model_points: usize,
    checked_points: usize,
    skipped_points: usize,
    depth_limited_points: usize,
    eval_depth_limit: Option<usize>,
    counterexample_found: bool,
}

impl SemanticDupEvidence {
    fn equivalent(self) -> bool {
        !self.counterexample_found && self.checked_points > 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct FunctionValue {
    table: Vec<(Vec<EvalValue>, EvalValue)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum EvalValue {
    Symbol(String),
    Int(i64),
    Bool(bool),
    Adt {
        ctor: String,
        fields: Vec<EvalValue>,
    },
    Function(FunctionValue),
}

const BASE_EVAL_DEPTH_LIMIT: usize = 1024;
const MAX_EVAL_DEPTH_LIMIT: usize = 4096;
const MAX_FUNCTION_MODEL_VALUES: usize = 4096;

pub fn lint_program(program: &Program, options: LintOptions) -> Vec<LintDiagnostic> {
    let normalized = match normalize_program_aliases(program) {
        Ok(program) => program,
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::new();

    // 既存解決エラーがある場合は lint を進めてもノイズになるため打ち切る。
    if !resolve_program(&normalized).is_empty() {
        return out;
    }

    out.extend(lint_exact_duplicates(&normalized));
    out.extend(lint_unused_declarations(&normalized));

    if options.semantic_dup {
        out.extend(lint_semantic_duplicates(&normalized));
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

    let Some(ctx) = build_semantic_dup_context(program) else {
        return out;
    };

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
                if let Some(evidence) = assertions_semantic_evidence(a, b, &ctx) {
                    if !evidence.equivalent() {
                        continue;
                    }
                    out.push(LintDiagnostic::warning(
                        "L-DUP-MAYBE",
                        "duplicate",
                        format!(
                            "assert {} と {} は論理同値の可能性があります",
                            a.name, b.name
                        ),
                        Some(b.span.clone()),
                        Some(semantic_dup_confidence(evidence)),
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
                if let Some(evidence) = defns_semantic_evidence(a, b, &ctx) {
                    if evidence.depth_limited_points > 0 {
                        let limit = evidence.eval_depth_limit.unwrap_or(BASE_EVAL_DEPTH_LIMIT);
                        out.push(LintDiagnostic::warning(
                            "L-DUP-SKIP-EVAL-DEPTH",
                            "duplicate",
                            format!(
                                "defn {} と {} の評価で深さ上限に到達しました: depth_limit={}, checked={}, skipped={}, depth_limited={}",
                                a.name,
                                b.name,
                                limit,
                                evidence.checked_points,
                                evidence.skipped_points,
                                evidence.depth_limited_points
                            ),
                            Some(b.span.clone()),
                            None,
                        ));
                    }
                    if !evidence.equivalent() {
                        continue;
                    }
                    out.push(LintDiagnostic::warning(
                        "L-DUP-MAYBE",
                        "duplicate",
                        format!("defn {} と {} は等価実装の可能性があります", a.name, b.name),
                        Some(b.span.clone()),
                        Some(semantic_dup_confidence(evidence)),
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
                if let Some(evidence) = rules_semantic_evidence(a, b, &ctx) {
                    if !evidence.equivalent() {
                        continue;
                    }
                    out.push(LintDiagnostic::warning(
                        "L-DUP-MAYBE",
                        "duplicate",
                        format!(
                            "rule {} の定義が有限モデル上で同値の可能性があります",
                            a.head.pred
                        ),
                        Some(b.span.clone()),
                        Some(semantic_dup_confidence(evidence)),
                    ));
                }
            }
        }
    }

    out
}

#[derive(Debug, Clone)]
struct ConstructorSig {
    owner: String,
    fields: Vec<Type>,
}

#[derive(Debug)]
struct SemanticDupContext<'a> {
    program: &'a Program,
    derived: DerivedFacts,
    universe: HashMap<String, Vec<Value>>,
    relation_schemas: HashMap<String, Vec<String>>,
    constructor_sigs: HashMap<String, ConstructorSig>,
    defn_indices: HashMap<String, usize>,
}

fn build_semantic_dup_context(program: &Program) -> Option<SemanticDupContext<'_>> {
    let kb = KnowledgeBase::from_program(program).ok()?;
    let derived = solve_facts(&kb).ok()?;
    let universe = build_universe_values(program)?;
    let relation_schemas = program
        .relations
        .iter()
        .map(|r| (r.name.clone(), r.arg_sorts.clone()))
        .collect::<HashMap<_, _>>();
    let constructor_sigs = program
        .data_decls
        .iter()
        .flat_map(|d| {
            d.constructors.iter().map(move |ctor| {
                (
                    ctor.name.clone(),
                    ConstructorSig {
                        owner: d.name.clone(),
                        fields: ctor.fields.clone(),
                    },
                )
            })
        })
        .collect::<HashMap<_, _>>();
    let defn_indices = program
        .defns
        .iter()
        .enumerate()
        .map(|(idx, defn)| (defn.name.clone(), idx))
        .collect::<HashMap<_, _>>();

    Some(SemanticDupContext {
        program,
        derived,
        universe,
        relation_schemas,
        constructor_sigs,
        defn_indices,
    })
}

fn semantic_dup_confidence(evidence: SemanticDupEvidence) -> f64 {
    let coverage = if evidence.model_points == 0 {
        0.0
    } else {
        evidence.checked_points as f64 / evidence.model_points as f64
    };
    let search_strength = if evidence.checked_points == 0 {
        0.0
    } else {
        1.0 - (1.0 / (evidence.checked_points as f64 + 1.0))
    };
    let counterexample_factor = if evidence.counterexample_found {
        0.0
    } else {
        1.0
    };
    let raw = 0.10 + (0.35 * coverage) + (0.35 * search_strength) + (0.20 * counterexample_factor);
    ((raw.clamp(0.0, 0.99) * 100.0).round()) / 100.0
}

fn build_universe_values(program: &Program) -> Option<HashMap<String, Vec<Value>>> {
    let mut out = HashMap::new();
    for universe in &program.universes {
        let mut vals = BTreeSet::new();
        for term in &universe.values {
            vals.insert(logic_term_to_const_value(term)?);
        }
        if vals.is_empty() {
            return None;
        }
        out.insert(universe.ty_name.clone(), vals.into_iter().collect());
    }
    Some(out)
}

fn assertions_semantic_evidence(
    a: &AssertDecl,
    b: &AssertDecl,
    ctx: &SemanticDupContext<'_>,
) -> Option<SemanticDupEvidence> {
    let tuples = enumerate_const_param_tuples(&a.params, &ctx.universe)?;
    let total = tuples.len();
    let mut checked = 0usize;

    for tuple in tuples {
        let env_a = bind_params(&a.params, &tuple);
        let env_b = bind_params(&b.params, &tuple);
        checked += 1;
        let a_ok = eval_formula_with_env(&a.formula, &ctx.derived, &env_a);
        let b_ok = eval_formula_with_env(&b.formula, &ctx.derived, &env_b);
        if a_ok != b_ok {
            return Some(SemanticDupEvidence {
                model_points: total,
                checked_points: checked,
                skipped_points: total.saturating_sub(checked),
                depth_limited_points: 0,
                eval_depth_limit: None,
                counterexample_found: true,
            });
        }
    }
    Some(SemanticDupEvidence {
        model_points: total,
        checked_points: checked,
        skipped_points: total.saturating_sub(checked),
        depth_limited_points: 0,
        eval_depth_limit: None,
        counterexample_found: false,
    })
}

fn rules_semantic_evidence(
    a: &Rule,
    b: &Rule,
    ctx: &SemanticDupContext<'_>,
) -> Option<SemanticDupEvidence> {
    let lhs = rule_head_tuples(a, ctx)?;
    let rhs = rule_head_tuples(b, ctx)?;
    Some(SemanticDupEvidence {
        model_points: lhs.total_valuations.max(rhs.total_valuations),
        checked_points: lhs.evaluated_valuations.min(rhs.evaluated_valuations),
        skipped_points: 0,
        depth_limited_points: 0,
        eval_depth_limit: None,
        counterexample_found: lhs.tuples != rhs.tuples,
    })
}

#[derive(Debug)]
struct RuleHeadSummary {
    tuples: BTreeSet<Vec<Value>>,
    total_valuations: usize,
    evaluated_valuations: usize,
}

fn rule_head_tuples(rule: &Rule, ctx: &SemanticDupContext<'_>) -> Option<RuleHeadSummary> {
    let vars = infer_rule_var_types(rule, &ctx.relation_schemas, &ctx.constructor_sigs)?;
    let valuations = enumerate_named_valuations(&vars, &ctx.universe)?;
    let total_valuations = valuations.len();
    let mut evaluated_valuations = 0usize;
    let mut out = BTreeSet::new();

    for valuation in valuations {
        if !eval_formula_with_env(&rule.body, &ctx.derived, &valuation) {
            evaluated_valuations += 1;
            continue;
        }
        let Some(tuple) = instantiate_terms(&rule.head.terms, &valuation) else {
            continue;
        };
        evaluated_valuations += 1;
        out.insert(tuple);
    }
    Some(RuleHeadSummary {
        tuples: out,
        total_valuations,
        evaluated_valuations,
    })
}

fn infer_rule_var_types(
    rule: &Rule,
    relation_schemas: &HashMap<String, Vec<String>>,
    constructor_sigs: &HashMap<String, ConstructorSig>,
) -> Option<Vec<(String, String)>> {
    let mut vars = HashMap::new();
    infer_atom_var_types(&rule.head, relation_schemas, constructor_sigs, &mut vars)?;
    infer_formula_var_types(&rule.body, relation_schemas, constructor_sigs, &mut vars)?;
    let mut out = vars.into_iter().collect::<Vec<_>>();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Some(out)
}

fn infer_formula_var_types(
    formula: &Formula,
    relation_schemas: &HashMap<String, Vec<String>>,
    constructor_sigs: &HashMap<String, ConstructorSig>,
    vars: &mut HashMap<String, String>,
) -> Option<()> {
    match formula {
        Formula::True => Some(()),
        Formula::Atom(atom) => infer_atom_var_types(atom, relation_schemas, constructor_sigs, vars),
        Formula::And(items) => {
            for item in items {
                infer_formula_var_types(item, relation_schemas, constructor_sigs, vars)?;
            }
            Some(())
        }
        Formula::Not(inner) => {
            infer_formula_var_types(inner, relation_schemas, constructor_sigs, vars)
        }
    }
}

fn infer_atom_var_types(
    atom: &Atom,
    relation_schemas: &HashMap<String, Vec<String>>,
    constructor_sigs: &HashMap<String, ConstructorSig>,
    vars: &mut HashMap<String, String>,
) -> Option<()> {
    let schema = relation_schemas.get(&atom.pred)?;
    if schema.len() != atom.terms.len() {
        return None;
    }

    for (term, expected) in atom.terms.iter().zip(schema.iter()) {
        infer_term_var_types(term, expected, constructor_sigs, vars)?;
    }
    Some(())
}

fn infer_term_var_types(
    term: &LogicTerm,
    expected: &str,
    constructor_sigs: &HashMap<String, ConstructorSig>,
    vars: &mut HashMap<String, String>,
) -> Option<()> {
    match term {
        LogicTerm::Var(name) => {
            if let Some(prev) = vars.get(name) {
                if prev != expected {
                    return None;
                }
            } else {
                vars.insert(name.clone(), expected.to_string());
            }
            Some(())
        }
        LogicTerm::Symbol(_) => (expected == "Symbol").then_some(()),
        LogicTerm::Int(_) => (expected == "Int").then_some(()),
        LogicTerm::Bool(_) => (expected == "Bool").then_some(()),
        LogicTerm::Ctor { name, args } => {
            let sig = constructor_sigs.get(name)?;
            if sig.owner != expected || sig.fields.len() != args.len() {
                return None;
            }
            for (arg, field_ty) in args.iter().zip(sig.fields.iter()) {
                let key = type_key(field_ty)?;
                infer_term_var_types(arg, &key, constructor_sigs, vars)?;
            }
            Some(())
        }
    }
}

fn defns_semantic_evidence(
    a: &Defn,
    b: &Defn,
    ctx: &SemanticDupContext<'_>,
) -> Option<SemanticDupEvidence> {
    let tuples = enumerate_eval_param_tuples(&a.params, &ctx.universe)?;
    let total = tuples.len();
    let mut checked = 0usize;
    let mut depth_limited_points = 0usize;
    let eval_depth_limit = adaptive_eval_depth_limit(a, b);

    for tuple in tuples {
        let mut depth_limited = false;
        let left = eval_defn_with_tuple(a, &tuple, ctx, 0, eval_depth_limit, &mut depth_limited);
        let right = eval_defn_with_tuple(b, &tuple, ctx, 0, eval_depth_limit, &mut depth_limited);
        let (Some(left), Some(right)) = (left, right) else {
            if depth_limited {
                depth_limited_points += 1;
            }
            continue;
        };
        checked += 1;
        if left != right {
            return Some(SemanticDupEvidence {
                model_points: total,
                checked_points: checked,
                skipped_points: total.saturating_sub(checked),
                depth_limited_points,
                eval_depth_limit: Some(eval_depth_limit),
                counterexample_found: true,
            });
        }
    }
    Some(SemanticDupEvidence {
        model_points: total,
        checked_points: checked,
        skipped_points: total.saturating_sub(checked),
        depth_limited_points,
        eval_depth_limit: Some(eval_depth_limit),
        counterexample_found: false,
    })
}

fn adaptive_eval_depth_limit(a: &Defn, b: &Defn) -> usize {
    let complexity = expr_node_count(&a.body).max(expr_node_count(&b.body));
    BASE_EVAL_DEPTH_LIMIT
        .saturating_add(complexity.saturating_mul(8))
        .min(MAX_EVAL_DEPTH_LIMIT)
}

fn expr_node_count(expr: &Expr) -> usize {
    match expr {
        Expr::Var { .. } | Expr::Symbol { .. } | Expr::Int { .. } | Expr::Bool { .. } => 1,
        Expr::Call { args, .. } => 1 + args.iter().map(expr_node_count).sum::<usize>(),
        Expr::Let { bindings, body, .. } => {
            1 + bindings
                .iter()
                .map(|(_, bexpr, _)| expr_node_count(bexpr))
                .sum::<usize>()
                + expr_node_count(body)
        }
        Expr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            1 + expr_node_count(cond) + expr_node_count(then_branch) + expr_node_count(else_branch)
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            1 + expr_node_count(scrutinee)
                + arms
                    .iter()
                    .map(|arm| expr_node_count(&arm.body))
                    .sum::<usize>()
        }
    }
}

fn eval_defn_with_tuple(
    defn: &Defn,
    tuple: &[EvalValue],
    ctx: &SemanticDupContext<'_>,
    depth: usize,
    depth_limit: usize,
    depth_limited: &mut bool,
) -> Option<EvalValue> {
    if defn.params.len() != tuple.len() {
        return None;
    }
    let mut env = HashMap::new();
    for (param, value) in defn.params.iter().zip(tuple.iter()) {
        env.insert(param.name.clone(), value.clone());
    }
    eval_expr_with_env(&defn.body, &env, ctx, depth, depth_limit, depth_limited)
}

fn eval_expr_with_env(
    expr: &Expr,
    env: &HashMap<String, EvalValue>,
    ctx: &SemanticDupContext<'_>,
    depth: usize,
    depth_limit: usize,
    depth_limited: &mut bool,
) -> Option<EvalValue> {
    if depth > depth_limit {
        *depth_limited = true;
        return None;
    }

    match expr {
        Expr::Var { name, .. } => env.get(name).cloned(),
        Expr::Symbol { value, .. } => Some(EvalValue::Symbol(value.clone())),
        Expr::Int { value, .. } => Some(EvalValue::Int(*value)),
        Expr::Bool { value, .. } => Some(EvalValue::Bool(*value)),
        Expr::Call { name, args, .. } => {
            let mut values = Vec::new();
            for arg in args {
                values.push(eval_expr_with_env(
                    arg,
                    env,
                    ctx,
                    depth + 1,
                    depth_limit,
                    depth_limited,
                )?);
            }

            if let Some(EvalValue::Function(fun)) = env.get(name) {
                return apply_function_value(fun, &values);
            }

            if let Some(idx) = ctx.defn_indices.get(name).copied() {
                let defn = &ctx.program.defns[idx];
                return eval_defn_with_tuple(
                    defn,
                    &values,
                    ctx,
                    depth + 1,
                    depth_limit,
                    depth_limited,
                );
            }
            if ctx.constructor_sigs.contains_key(name) {
                return Some(EvalValue::Adt {
                    ctor: name.clone(),
                    fields: values,
                });
            }
            if let Some(schema) = ctx.relation_schemas.get(name) {
                if schema.len() != values.len() {
                    return None;
                }
                let tuple = values
                    .iter()
                    .map(eval_to_concrete)
                    .collect::<Option<Vec<_>>>()?;
                let exists = ctx
                    .derived
                    .facts
                    .get(name)
                    .map(|set| set.contains(&tuple))
                    .unwrap_or(false);
                return Some(EvalValue::Bool(exists));
            }
            None
        }
        Expr::Let { bindings, body, .. } => {
            let mut local = env.clone();
            for (name, bexpr, _) in bindings {
                let value =
                    eval_expr_with_env(bexpr, &local, ctx, depth + 1, depth_limit, depth_limited)?;
                local.insert(name.clone(), value);
            }
            eval_expr_with_env(body, &local, ctx, depth + 1, depth_limit, depth_limited)
        }
        Expr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            let cond_value =
                eval_expr_with_env(cond, env, ctx, depth + 1, depth_limit, depth_limited)?;
            match cond_value {
                EvalValue::Bool(true) => {
                    eval_expr_with_env(then_branch, env, ctx, depth + 1, depth_limit, depth_limited)
                }
                EvalValue::Bool(false) => {
                    eval_expr_with_env(else_branch, env, ctx, depth + 1, depth_limit, depth_limited)
                }
                _ => None,
            }
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            let target =
                eval_expr_with_env(scrutinee, env, ctx, depth + 1, depth_limit, depth_limited)?;
            for arm in arms {
                let mut captures = HashMap::new();
                if !matches_pattern_eval(&arm.pattern, &target, &mut captures) {
                    continue;
                }
                let mut local = env.clone();
                for (name, value) in captures {
                    local.insert(name, value);
                }
                return eval_expr_with_env(
                    &arm.body,
                    &local,
                    ctx,
                    depth + 1,
                    depth_limit,
                    depth_limited,
                );
            }
            None
        }
    }
}

fn apply_function_value(fun: &FunctionValue, args: &[EvalValue]) -> Option<EvalValue> {
    fun.table
        .iter()
        .find(|(inputs, _)| inputs == args)
        .map(|(_, output)| output.clone())
}

fn eval_to_concrete(value: &EvalValue) -> Option<Value> {
    match value {
        EvalValue::Symbol(s) => Some(Value::Symbol(s.clone())),
        EvalValue::Int(i) => Some(Value::Int(*i)),
        EvalValue::Bool(b) => Some(Value::Bool(*b)),
        EvalValue::Adt { ctor, fields } => {
            let concrete_fields = fields
                .iter()
                .map(eval_to_concrete)
                .collect::<Option<Vec<_>>>()?;
            Some(Value::Adt {
                ctor: ctor.clone(),
                fields: concrete_fields,
            })
        }
        EvalValue::Function(_) => None,
    }
}

fn concrete_to_eval(value: &Value) -> EvalValue {
    match value {
        Value::Symbol(s) => EvalValue::Symbol(s.clone()),
        Value::Int(i) => EvalValue::Int(*i),
        Value::Bool(b) => EvalValue::Bool(*b),
        Value::Adt { ctor, fields } => EvalValue::Adt {
            ctor: ctor.clone(),
            fields: fields.iter().map(concrete_to_eval).collect(),
        },
    }
}

fn matches_pattern_eval(
    pattern: &Pattern,
    target: &EvalValue,
    binds: &mut HashMap<String, EvalValue>,
) -> bool {
    match pattern {
        Pattern::Wildcard { .. } => true,
        Pattern::Var { name, .. } => {
            if let Some(prev) = binds.get(name) {
                prev == target
            } else {
                binds.insert(name.clone(), target.clone());
                true
            }
        }
        Pattern::Symbol { value, .. } => matches!(target, EvalValue::Symbol(s) if s == value),
        Pattern::Int { value, .. } => matches!(target, EvalValue::Int(i) if i == value),
        Pattern::Bool { value, .. } => matches!(target, EvalValue::Bool(b) if b == value),
        Pattern::Ctor { name, args, .. } => {
            let EvalValue::Adt { ctor, fields } = target else {
                return false;
            };
            if name != ctor || args.len() != fields.len() {
                return false;
            }
            for (p, value) in args.iter().zip(fields.iter()) {
                if !matches_pattern_eval(p, value, binds) {
                    return false;
                }
            }
            true
        }
    }
}

fn eval_formula_with_env(
    formula: &Formula,
    derived: &DerivedFacts,
    env: &HashMap<String, Value>,
) -> bool {
    match formula {
        Formula::True => true,
        Formula::Atom(atom) => {
            let Some(tuple) = instantiate_terms(&atom.terms, env) else {
                return false;
            };
            derived
                .facts
                .get(&atom.pred)
                .map(|set| set.contains(&tuple))
                .unwrap_or(false)
        }
        Formula::And(items) => items
            .iter()
            .all(|item| eval_formula_with_env(item, derived, env)),
        Formula::Not(inner) => !eval_formula_with_env(inner, derived, env),
    }
}

fn bind_params(params: &[Param], values: &[Value]) -> HashMap<String, Value> {
    params
        .iter()
        .zip(values.iter())
        .map(|(param, value)| (param.name.clone(), value.clone()))
        .collect()
}

fn enumerate_const_param_tuples(
    params: &[Param],
    universe: &HashMap<String, Vec<Value>>,
) -> Option<Vec<Vec<Value>>> {
    let mut domains = Vec::new();
    for param in params {
        let key = type_key(&param.ty)?;
        let values = universe.get(&key)?;
        if values.is_empty() {
            return None;
        }
        domains.push(values.clone());
    }
    Some(enumerate_tuples(&domains))
}

fn enumerate_eval_param_tuples(
    params: &[Param],
    universe: &HashMap<String, Vec<Value>>,
) -> Option<Vec<Vec<EvalValue>>> {
    let mut domains = Vec::new();
    let mut cache = HashMap::new();
    for param in params {
        let values = enumerate_eval_values_for_type(&param.ty, universe, &mut cache)?;
        if values.is_empty() {
            return None;
        }
        domains.push(values);
    }
    Some(enumerate_tuples(&domains))
}

fn enumerate_eval_values_for_type(
    ty: &Type,
    universe: &HashMap<String, Vec<Value>>,
    cache: &mut HashMap<Type, Vec<EvalValue>>,
) -> Option<Vec<EvalValue>> {
    let normalized = match ty {
        Type::Refine { base, .. } => base.as_ref().clone(),
        _ => ty.clone(),
    };
    if let Some(cached) = cache.get(&normalized) {
        return Some(cached.clone());
    }

    let values = match &normalized {
        Type::Bool => universe
            .get("Bool")?
            .iter()
            .map(concrete_to_eval)
            .collect::<Vec<_>>(),
        Type::Int => universe
            .get("Int")?
            .iter()
            .map(concrete_to_eval)
            .collect::<Vec<_>>(),
        Type::Symbol => universe
            .get("Symbol")?
            .iter()
            .map(concrete_to_eval)
            .collect::<Vec<_>>(),
        Type::Domain(name) | Type::Adt(name) => universe
            .get(name)?
            .iter()
            .map(concrete_to_eval)
            .collect::<Vec<_>>(),
        Type::Fun(args, ret) => enumerate_function_values(args, ret, universe, cache)?,
        Type::Refine { .. } => unreachable!(),
    };
    cache.insert(normalized, values.clone());
    Some(values)
}

fn enumerate_function_values(
    args: &[Type],
    ret: &Type,
    universe: &HashMap<String, Vec<Value>>,
    cache: &mut HashMap<Type, Vec<EvalValue>>,
) -> Option<Vec<EvalValue>> {
    let mut arg_domains = Vec::new();
    for arg in args {
        let values = enumerate_eval_values_for_type(arg, universe, cache)?;
        if values.is_empty() {
            return None;
        }
        arg_domains.push(values);
    }
    let input_tuples = enumerate_tuples(&arg_domains);
    let output_values = enumerate_eval_values_for_type(ret, universe, cache)?;
    if output_values.is_empty() {
        return None;
    }

    let output_count = output_values.len();
    let exponent = u32::try_from(input_tuples.len()).ok()?;
    let total = output_count.checked_pow(exponent)?;
    if total > MAX_FUNCTION_MODEL_VALUES {
        return None;
    }

    let mut out = Vec::with_capacity(total);
    let mut selectors = vec![0usize; input_tuples.len()];
    loop {
        let table = input_tuples
            .iter()
            .enumerate()
            .map(|(idx, input)| (input.clone(), output_values[selectors[idx]].clone()))
            .collect::<Vec<_>>();
        out.push(EvalValue::Function(FunctionValue { table }));
        if !increment_digits(&mut selectors, output_count) {
            break;
        }
    }
    Some(out)
}

fn increment_digits(digits: &mut [usize], base: usize) -> bool {
    if digits.is_empty() {
        return false;
    }
    for digit in digits {
        *digit += 1;
        if *digit < base {
            return true;
        }
        *digit = 0;
    }
    false
}

fn enumerate_tuples<T: Clone>(domains: &[Vec<T>]) -> Vec<Vec<T>> {
    let mut out = Vec::new();
    enumerate_tuples_inner(domains, 0, &mut Vec::new(), &mut out);
    out
}

fn enumerate_tuples_inner<T: Clone>(
    domains: &[Vec<T>],
    idx: usize,
    current: &mut Vec<T>,
    out: &mut Vec<Vec<T>>,
) {
    if idx == domains.len() {
        out.push(current.clone());
        return;
    }

    for value in &domains[idx] {
        current.push(value.clone());
        enumerate_tuples_inner(domains, idx + 1, current, out);
        current.pop();
    }
}

fn enumerate_named_valuations(
    vars: &[(String, String)],
    universe: &HashMap<String, Vec<Value>>,
) -> Option<Vec<HashMap<String, Value>>> {
    let mut domains = Vec::new();
    for (name, key) in vars {
        let values = universe.get(key)?;
        if values.is_empty() {
            return None;
        }
        domains.push((name.clone(), values.clone()));
    }

    let mut out = Vec::new();
    enumerate_named(&domains, 0, &mut HashMap::new(), &mut out);
    Some(out)
}

fn enumerate_named(
    domains: &[(String, Vec<Value>)],
    idx: usize,
    current: &mut HashMap<String, Value>,
    out: &mut Vec<HashMap<String, Value>>,
) {
    if idx == domains.len() {
        out.push(current.clone());
        return;
    }

    let (name, values) = &domains[idx];
    for value in values {
        current.insert(name.clone(), value.clone());
        enumerate_named(domains, idx + 1, current, out);
    }
}

fn instantiate_terms(terms: &[LogicTerm], env: &HashMap<String, Value>) -> Option<Vec<Value>> {
    terms
        .iter()
        .map(|term| instantiate_term(term, env))
        .collect::<Option<Vec<_>>>()
}

fn instantiate_term(term: &LogicTerm, env: &HashMap<String, Value>) -> Option<Value> {
    match term {
        LogicTerm::Var(name) => env.get(name).cloned(),
        LogicTerm::Symbol(s) => Some(Value::Symbol(s.clone())),
        LogicTerm::Int(i) => Some(Value::Int(*i)),
        LogicTerm::Bool(b) => Some(Value::Bool(*b)),
        LogicTerm::Ctor { name, args } => {
            let fields = args
                .iter()
                .map(|arg| instantiate_term(arg, env))
                .collect::<Option<Vec<_>>>()?;
            Some(Value::Adt {
                ctor: name.clone(),
                fields,
            })
        }
    }
}

fn logic_term_to_const_value(term: &LogicTerm) -> Option<Value> {
    match term {
        LogicTerm::Var(_) => None,
        LogicTerm::Symbol(s) => Some(Value::Symbol(s.clone())),
        LogicTerm::Int(i) => Some(Value::Int(*i)),
        LogicTerm::Bool(b) => Some(Value::Bool(*b)),
        LogicTerm::Ctor { name, args } => {
            let fields = args
                .iter()
                .map(logic_term_to_const_value)
                .collect::<Option<Vec<_>>>()?;
            Some(Value::Adt {
                ctor: name.clone(),
                fields,
            })
        }
    }
}

fn type_key(ty: &Type) -> Option<String> {
    match ty {
        Type::Bool => Some("Bool".to_string()),
        Type::Int => Some("Int".to_string()),
        Type::Symbol => Some("Symbol".to_string()),
        Type::Domain(name) | Type::Adt(name) => Some(name.clone()),
        Type::Refine { base, .. } => type_key(base),
        Type::Fun(_, _) => None,
    }
}

fn missing_universe_types(program: &Program) -> Option<Vec<String>> {
    let declared = program
        .universes
        .iter()
        .map(|u| u.ty_name.clone())
        .collect::<HashSet<_>>();
    let mut required = HashSet::new();
    for relation in &program.relations {
        for sort in &relation.arg_sorts {
            required.insert(sort.clone());
        }
    }
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
