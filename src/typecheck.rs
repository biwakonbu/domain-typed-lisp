use std::collections::{HashMap, HashSet};

use crate::ast::{Defn, Expr, MatchArm, Pattern, Program};
use crate::diagnostics::Diagnostic;
use crate::logic_engine::{DerivedFacts, GroundFact, KnowledgeBase, Value, solve_facts};
use crate::name_resolve::{normalize_program_aliases, resolve_program};
use crate::stratify::compute_strata;
use crate::types::{Atom, Formula, LogicTerm, Type};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeReport {
    pub functions_checked: usize,
    pub errors: usize,
}

#[derive(Debug, Clone)]
struct FunctionSig {
    params: Vec<Type>,
    ret: Type,
    param_names: Vec<String>,
}

#[derive(Debug, Clone)]
struct ConstructorSig {
    fields: Vec<Type>,
    ret: Type,
}

#[derive(Debug, Clone)]
struct TypeContext {
    relation_sigs: HashMap<String, Vec<Type>>,
    function_sigs: HashMap<String, FunctionSig>,
    constructor_sigs: HashMap<String, ConstructorSig>,
    data_constructors: HashMap<String, Vec<String>>,
    kb_template: KnowledgeBase,
}

#[derive(Debug, Clone, Copy)]
struct OriginInfo {
    param_index: usize,
    strict_subterm: bool,
}

struct RecursionEdgeRule<'a> {
    caller_name: &'a str,
    caller_adt_param_indices: &'a HashSet<usize>,
    scc_callee_rules: &'a HashMap<String, CalleeRule>,
}

#[derive(Debug, Clone)]
struct CalleeRule {
    callee_name: String,
    adt_param_indices: HashSet<usize>,
    param_len: usize,
}

const TOTAL_REASON_NON_TAIL_CALL: &str = "non_tail_recursive_call";
const TOTAL_REASON_ARITY_MISMATCH: &str = "recursive_call_arity_mismatch";
const TOTAL_REASON_NO_ADT_PARAM: &str = "no_adt_parameter";
const TOTAL_REASON_NON_DECREASING_ARG: &str = "non_decreasing_argument";

pub fn check_program(program: &Program) -> Result<TypeReport, Vec<Diagnostic>> {
    let normalized = normalize_program_aliases(program)?;
    let mut errors = resolve_program(&normalized);
    if !errors.is_empty() {
        return Err(errors);
    }

    if let Err(mut stratify_errors) = compute_strata(&normalized) {
        errors.append(&mut stratify_errors);
        return Err(errors);
    }

    let mut totality_errors = check_totality(&normalized);
    if !totality_errors.is_empty() {
        errors.append(&mut totality_errors);
        return Err(errors);
    }

    let kb = KnowledgeBase::from_program(&normalized)?;
    let _ = solve_facts(&kb)?;

    let data_names: HashSet<String> = normalized
        .data_decls
        .iter()
        .map(|d| d.name.clone())
        .collect();
    let relation_sigs = build_relation_sigs(&normalized, &data_names);
    let function_sigs = build_function_sigs(&normalized, &data_names);
    let constructor_sigs = build_constructor_sigs(&normalized, &data_names);
    let data_constructors = build_data_constructor_map(&normalized);

    let ctx = TypeContext {
        relation_sigs,
        function_sigs,
        constructor_sigs,
        data_constructors,
        kb_template: kb,
    };

    for defn in &normalized.defns {
        if let Err(mut e) = check_defn(defn, &ctx) {
            errors.append(&mut e);
        }
    }

    if errors.is_empty() {
        Ok(TypeReport {
            functions_checked: normalized.defns.len(),
            errors: 0,
        })
    } else {
        Err(errors)
    }
}

fn check_totality(program: &Program) -> Vec<Diagnostic> {
    let function_names: HashSet<String> = program.defns.iter().map(|d| d.name.clone()).collect();
    let mut calls: HashMap<String, HashSet<String>> = HashMap::new();

    for defn in &program.defns {
        let mut called = HashSet::new();
        collect_function_calls(&defn.body, &function_names, &mut called);
        calls.insert(defn.name.clone(), called);
    }

    let mut errors = Vec::new();
    let data_names: HashSet<String> = program.data_decls.iter().map(|d| d.name.clone()).collect();
    let defn_map: HashMap<String, &Defn> =
        program.defns.iter().map(|d| (d.name.clone(), d)).collect();
    let adt_param_indices_map = program
        .defns
        .iter()
        .map(|defn| (defn.name.clone(), adt_param_indices(defn, &data_names)))
        .collect::<HashMap<_, _>>();
    let components = strongly_connected_components(&function_names, &calls);

    for component in components {
        let recursive_component = if component.len() > 1 {
            true
        } else {
            component
                .first()
                .and_then(|name| calls.get(name).map(|nexts| nexts.contains(name)))
                .unwrap_or(false)
        };
        if !recursive_component {
            continue;
        }

        let mut scc_callee_rules = HashMap::new();
        for callee_name in &component {
            if let Some(defn) = defn_map.get(callee_name) {
                scc_callee_rules.insert(
                    callee_name.clone(),
                    CalleeRule {
                        callee_name: callee_name.clone(),
                        adt_param_indices: adt_param_indices_map
                            .get(callee_name)
                            .cloned()
                            .unwrap_or_default(),
                        param_len: defn.params.len(),
                    },
                );
            }
        }

        for caller_name in &component {
            let Some(defn) = defn_map.get(caller_name) else {
                continue;
            };
            let caller_adt_param_indices = adt_param_indices_map
                .get(caller_name)
                .expect("caller should exist in ADT parameter map");

            let mut origin_env = HashMap::new();
            for (idx, p) in defn.params.iter().enumerate() {
                origin_env.insert(
                    p.name.clone(),
                    OriginInfo {
                        param_index: idx,
                        strict_subterm: false,
                    },
                );
            }

            let rule = RecursionEdgeRule {
                caller_name,
                caller_adt_param_indices,
                scc_callee_rules: &scc_callee_rules,
            };
            collect_totality_violations(&defn.body, true, &origin_env, &rule, &mut errors);
        }
    }

    errors
}

fn adt_param_indices(defn: &Defn, data_names: &HashSet<String>) -> HashSet<usize> {
    defn.params
        .iter()
        .enumerate()
        .filter_map(|(idx, p)| {
            let ty = canonicalize_type(&p.ty, data_names);
            if matches!(ty.as_base(), Type::Adt(_)) {
                Some(idx)
            } else {
                None
            }
        })
        .collect()
}

fn strongly_connected_components(
    function_names: &HashSet<String>,
    calls: &HashMap<String, HashSet<String>>,
) -> Vec<Vec<String>> {
    #[derive(Default)]
    struct TarjanState {
        index: usize,
        indices: HashMap<String, usize>,
        lowlinks: HashMap<String, usize>,
        stack: Vec<String>,
        on_stack: HashSet<String>,
        components: Vec<Vec<String>>,
    }

    fn strong_connect(
        node: String,
        calls: &HashMap<String, HashSet<String>>,
        state: &mut TarjanState,
    ) {
        let current_index = state.index;
        state.index += 1;
        state.indices.insert(node.clone(), current_index);
        state.lowlinks.insert(node.clone(), current_index);
        state.stack.push(node.clone());
        state.on_stack.insert(node.clone());

        if let Some(nexts) = calls.get(&node) {
            for next in nexts {
                if !state.indices.contains_key(next) {
                    strong_connect(next.clone(), calls, state);
                    let low_next = *state
                        .lowlinks
                        .get(next)
                        .expect("next node lowlink should exist");
                    let low_node = state
                        .lowlinks
                        .get_mut(&node)
                        .expect("node lowlink should exist");
                    *low_node = (*low_node).min(low_next);
                } else if state.on_stack.contains(next) {
                    let idx_next = *state
                        .indices
                        .get(next)
                        .expect("next node index should exist");
                    let low_node = state
                        .lowlinks
                        .get_mut(&node)
                        .expect("node lowlink should exist");
                    *low_node = (*low_node).min(idx_next);
                }
            }
        }

        let low_node = *state
            .lowlinks
            .get(&node)
            .expect("node lowlink should exist");
        let idx_node = *state.indices.get(&node).expect("node index should exist");
        if low_node == idx_node {
            let mut component = Vec::new();
            loop {
                let top = state.stack.pop().expect("stack should contain node");
                state.on_stack.remove(&top);
                component.push(top.clone());
                if top == node {
                    break;
                }
            }
            component.sort();
            state.components.push(component);
        }
    }

    let mut nodes = function_names.iter().cloned().collect::<Vec<_>>();
    nodes.sort();
    let mut state = TarjanState::default();
    for node in nodes {
        if !state.indices.contains_key(&node) {
            strong_connect(node, calls, &mut state);
        }
    }
    state.components.sort_by(|a, b| a[0].cmp(&b[0]));
    state.components
}

fn collect_totality_violations(
    expr: &Expr,
    is_tail_position: bool,
    origin_env: &HashMap<String, OriginInfo>,
    rule: &RecursionEdgeRule<'_>,
    errors: &mut Vec<Diagnostic>,
) {
    match expr {
        Expr::Var { .. } | Expr::Symbol { .. } | Expr::Int { .. } | Expr::Bool { .. } => {}
        Expr::Call { name, args, span } => {
            for arg in args {
                collect_totality_violations(arg, false, origin_env, rule, errors);
            }

            if let Some(callee_rule) = rule.scc_callee_rules.get(name) {
                check_recursive_call(
                    args,
                    span,
                    is_tail_position,
                    origin_env,
                    rule,
                    callee_rule,
                    errors,
                );
            }
        }
        Expr::Let { bindings, body, .. } => {
            let mut local_env = origin_env.clone();
            for (name, bexpr, _) in bindings {
                collect_totality_violations(bexpr, false, &local_env, rule, errors);
                if let Some(origin) = origin_of_expr(bexpr, &local_env) {
                    local_env.insert(name.clone(), origin);
                } else {
                    local_env.remove(name);
                }
            }
            collect_totality_violations(body, is_tail_position, &local_env, rule, errors);
        }
        Expr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            collect_totality_violations(cond, false, origin_env, rule, errors);
            collect_totality_violations(then_branch, is_tail_position, origin_env, rule, errors);
            collect_totality_violations(else_branch, is_tail_position, origin_env, rule, errors);
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            collect_totality_violations(scrutinee, false, origin_env, rule, errors);

            let scrutinee_origin = origin_of_expr(scrutinee, origin_env);
            for arm in arms {
                let mut arm_env = origin_env.clone();
                bind_pattern_origins(&arm.pattern, scrutinee_origin, &mut arm_env);
                collect_totality_violations(&arm.body, is_tail_position, &arm_env, rule, errors);
            }
        }
    }
}

fn check_recursive_call(
    args: &[Expr],
    span: &crate::diagnostics::Span,
    is_tail_position: bool,
    origin_env: &HashMap<String, OriginInfo>,
    edge_rule: &RecursionEdgeRule<'_>,
    callee_rule: &CalleeRule,
    errors: &mut Vec<Diagnostic>,
) {
    let edge = format!("{} -> {}", edge_rule.caller_name, callee_rule.callee_name);
    if !is_tail_position {
        errors.push(
            Diagnostic::new(
                "E-TOTAL",
                format!("recursive call is not in tail position: {edge}"),
                Some(span.clone()),
            )
            .with_reason(TOTAL_REASON_NON_TAIL_CALL),
        );
        return;
    }

    if args.len() != callee_rule.param_len {
        errors.push(
            Diagnostic::new(
                "E-TOTAL",
                format!(
                    "recursive call arity mismatch in {edge}: expected {param_len}, got {got}",
                    param_len = callee_rule.param_len,
                    got = args.len()
                ),
                Some(span.clone()),
            )
            .with_reason(TOTAL_REASON_ARITY_MISMATCH),
        );
        return;
    }

    if callee_rule.adt_param_indices.is_empty() {
        errors.push(
            Diagnostic::new(
                "E-TOTAL",
                format!("recursive call target has no ADT parameter in {edge}"),
                Some(span.clone()),
            )
            .with_reason(TOTAL_REASON_NO_ADT_PARAM),
        );
        return;
    }

    let mut positions = callee_rule
        .adt_param_indices
        .iter()
        .copied()
        .collect::<Vec<_>>();
    positions.sort_unstable();
    let has_decreasing_arg = positions.iter().any(|idx| {
        is_strict_subterm_of_caller_adt_param(
            args.get(*idx)
                .expect("recursive call arity already validated"),
            origin_env,
            edge_rule.caller_adt_param_indices,
        )
    });

    if !has_decreasing_arg {
        let one_based = positions.iter().map(|idx| idx + 1).collect::<Vec<_>>();
        let one_based_text = one_based
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        errors.push(
            Diagnostic::new(
                "E-TOTAL",
                format!(
                    "recursive call is not structurally decreasing in {edge}: argument index [{one_based_text}]"
                ),
                Some(span.clone()),
            )
            .with_reason(TOTAL_REASON_NON_DECREASING_ARG)
            .with_arg_indices(one_based),
        );
    }
}

fn is_strict_subterm_of_caller_adt_param(
    expr: &Expr,
    origin_env: &HashMap<String, OriginInfo>,
    caller_adt_param_indices: &HashSet<usize>,
) -> bool {
    let Expr::Var { name, .. } = expr else {
        return false;
    };
    let Some(origin) = origin_env.get(name) else {
        return false;
    };
    caller_adt_param_indices.contains(&origin.param_index) && origin.strict_subterm
}

fn origin_of_expr(expr: &Expr, env: &HashMap<String, OriginInfo>) -> Option<OriginInfo> {
    let Expr::Var { name, .. } = expr else {
        return None;
    };
    env.get(name).copied()
}

fn bind_pattern_origins(
    pattern: &Pattern,
    scrutinee_origin: Option<OriginInfo>,
    env: &mut HashMap<String, OriginInfo>,
) {
    match pattern {
        Pattern::Wildcard { .. }
        | Pattern::Symbol { .. }
        | Pattern::Int { .. }
        | Pattern::Bool { .. } => {}
        Pattern::Var { name, .. } => {
            if let Some(origin) = scrutinee_origin {
                env.insert(name.clone(), origin);
            } else {
                env.remove(name);
            }
        }
        Pattern::Ctor { args, .. } => {
            let child_origin = scrutinee_origin.map(|origin| OriginInfo {
                param_index: origin.param_index,
                strict_subterm: true,
            });
            for arg in args {
                bind_pattern_origins(arg, child_origin, env);
            }
        }
    }
}

fn collect_function_calls(
    expr: &Expr,
    function_names: &HashSet<String>,
    out: &mut HashSet<String>,
) {
    match expr {
        Expr::Var { .. } | Expr::Symbol { .. } | Expr::Int { .. } | Expr::Bool { .. } => {}
        Expr::Call { name, args, .. } => {
            if function_names.contains(name) {
                out.insert(name.clone());
            }
            for arg in args {
                collect_function_calls(arg, function_names, out);
            }
        }
        Expr::Let { bindings, body, .. } => {
            for (_, bexpr, _) in bindings {
                collect_function_calls(bexpr, function_names, out);
            }
            collect_function_calls(body, function_names, out);
        }
        Expr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            collect_function_calls(cond, function_names, out);
            collect_function_calls(then_branch, function_names, out);
            collect_function_calls(else_branch, function_names, out);
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            collect_function_calls(scrutinee, function_names, out);
            for arm in arms {
                collect_function_calls(&arm.body, function_names, out);
            }
        }
    }
}

fn build_relation_sigs(
    program: &Program,
    data_names: &HashSet<String>,
) -> HashMap<String, Vec<Type>> {
    let mut map = HashMap::new();
    for rel in &program.relations {
        map.insert(
            rel.name.clone(),
            rel.arg_sorts
                .iter()
                .map(|s| type_from_name(s, data_names))
                .collect::<Vec<_>>(),
        );
    }
    map
}

fn build_function_sigs(
    program: &Program,
    data_names: &HashSet<String>,
) -> HashMap<String, FunctionSig> {
    let mut map = HashMap::new();
    for f in &program.defns {
        map.insert(
            f.name.clone(),
            FunctionSig {
                params: f
                    .params
                    .iter()
                    .map(|p| canonicalize_type(&p.ty, data_names))
                    .collect(),
                ret: canonicalize_type(&f.ret_type, data_names),
                param_names: f.params.iter().map(|p| p.name.clone()).collect(),
            },
        );
    }
    map
}

fn build_constructor_sigs(
    program: &Program,
    data_names: &HashSet<String>,
) -> HashMap<String, ConstructorSig> {
    let mut map = HashMap::new();
    for data in &program.data_decls {
        for ctor in &data.constructors {
            map.insert(
                ctor.name.clone(),
                ConstructorSig {
                    fields: ctor
                        .fields
                        .iter()
                        .map(|ty| canonicalize_type(ty, data_names))
                        .collect(),
                    ret: Type::Adt(data.name.clone()),
                },
            );
        }
    }
    map
}

fn build_data_constructor_map(program: &Program) -> HashMap<String, Vec<String>> {
    let mut map = HashMap::new();
    for data in &program.data_decls {
        map.insert(
            data.name.clone(),
            data.constructors.iter().map(|c| c.name.clone()).collect(),
        );
    }
    map
}

fn check_defn(defn: &Defn, ctx: &TypeContext) -> Result<(), Vec<Diagnostic>> {
    let mut env = HashMap::new();
    for p in &defn.params {
        env.insert(p.name.clone(), canonicalize_type_for_ctx(&p.ty, ctx));
    }

    let actual = infer_expr(&defn.body, &env, ctx)?;
    let expected = canonicalize_type_for_ctx(&defn.ret_type, ctx);
    match is_subtype(&actual, &expected, ctx) {
        Ok(()) => Ok(()),
        Err(e) => Err(vec![e]),
    }
}

fn infer_expr(
    expr: &Expr,
    env: &HashMap<String, Type>,
    ctx: &TypeContext,
) -> Result<Type, Vec<Diagnostic>> {
    match expr {
        Expr::Var { name, span } => env.get(name).cloned().ok_or_else(|| {
            vec![Diagnostic::new(
                "E-TYPE",
                format!("unknown variable: {name}"),
                Some(span.clone()),
            )]
        }),
        Expr::Symbol { .. } => Ok(Type::Symbol),
        Expr::Int { .. } => Ok(Type::Int),
        Expr::Bool { .. } => Ok(Type::Bool),
        Expr::Let {
            bindings,
            body,
            span: _,
        } => {
            let mut local_env = env.clone();
            for (name, bexpr, _) in bindings {
                let ty = infer_expr(bexpr, &local_env, ctx)?;
                local_env.insert(name.clone(), ty);
            }
            infer_expr(body, &local_env, ctx)
        }
        Expr::If {
            cond,
            then_branch,
            else_branch,
            span,
        } => {
            let cond_ty = infer_expr(cond, env, ctx)?;
            ensure_subtype(
                &cond_ty,
                &Type::Bool,
                ctx,
                span,
                "if condition must be Bool",
            )?;

            let t_ty = infer_expr(then_branch, env, ctx)?;
            let e_ty = infer_expr(else_branch, env, ctx)?;

            if is_subtype(&t_ty, &e_ty, ctx).is_ok() {
                Ok(e_ty)
            } else if is_subtype(&e_ty, &t_ty, ctx).is_ok() {
                Ok(t_ty)
            } else {
                Err(vec![Diagnostic::new(
                    "E-TYPE",
                    "if branches have incompatible types",
                    Some(span.clone()),
                )])
            }
        }
        Expr::Match {
            scrutinee,
            arms,
            span,
        } => infer_match_expr(scrutinee, arms, span, env, ctx),
        Expr::Call { name, args, span } => {
            if let Some(sig) = ctx.function_sigs.get(name) {
                if sig.params.len() != args.len() {
                    return Err(vec![Diagnostic::new(
                        "E-TYPE",
                        format!(
                            "function {} arity mismatch: expected {}, got {}",
                            name,
                            sig.params.len(),
                            args.len()
                        ),
                        Some(span.clone()),
                    )]);
                }

                let mut substitution = HashMap::new();
                for (idx, (arg, expected)) in args.iter().zip(sig.params.iter()).enumerate() {
                    let actual = infer_expr(arg, env, ctx)?;
                    ensure_subtype(
                        &actual,
                        expected,
                        ctx,
                        arg.span(),
                        "function argument type mismatch",
                    )?;
                    if let Some(term) = expr_to_logic_term(arg, ctx) {
                        substitution.insert(sig.param_names[idx].clone(), term);
                    }
                }

                Ok(substitute_type(&sig.ret, &substitution))
            } else if let Some(sig) = ctx.constructor_sigs.get(name) {
                if sig.fields.len() != args.len() {
                    return Err(vec![Diagnostic::new(
                        "E-TYPE",
                        format!(
                            "constructor {} arity mismatch: expected {}, got {}",
                            name,
                            sig.fields.len(),
                            args.len()
                        ),
                        Some(span.clone()),
                    )]);
                }
                for (arg, expected) in args.iter().zip(sig.fields.iter()) {
                    let actual = infer_expr(arg, env, ctx)?;
                    ensure_subtype(
                        &actual,
                        expected,
                        ctx,
                        arg.span(),
                        "constructor argument type mismatch",
                    )?;
                }
                Ok(sig.ret.clone())
            } else if let Some(rel_sig) = ctx.relation_sigs.get(name) {
                if rel_sig.len() != args.len() {
                    return Err(vec![Diagnostic::new(
                        "E-TYPE",
                        format!(
                            "relation {} arity mismatch: expected {}, got {}",
                            name,
                            rel_sig.len(),
                            args.len()
                        ),
                        Some(span.clone()),
                    )]);
                }

                let mut terms = Vec::new();
                for (arg, expected) in args.iter().zip(rel_sig.iter()) {
                    let actual = infer_expr(arg, env, ctx)?;
                    ensure_subtype(
                        &actual,
                        expected,
                        ctx,
                        arg.span(),
                        "relation argument type mismatch",
                    )?;
                    let Some(term) = expr_to_logic_term(arg, ctx) else {
                        return Err(vec![Diagnostic::new(
                            "E-TYPE",
                            "relation argument must be variable/literal/constructor",
                            Some(arg.span().clone()),
                        )]);
                    };
                    terms.push(term);
                }

                Ok(Type::Refine {
                    var: "b".to_string(),
                    base: Box::new(Type::Bool),
                    formula: Formula::Atom(Atom {
                        pred: name.clone(),
                        terms,
                    }),
                })
            } else {
                Err(vec![Diagnostic::new(
                    "E-TYPE",
                    format!("unknown function or relation or constructor: {name}"),
                    Some(span.clone()),
                )])
            }
        }
    }
}

fn infer_match_expr(
    scrutinee: &Expr,
    arms: &[MatchArm],
    span: &crate::diagnostics::Span,
    env: &HashMap<String, Type>,
    ctx: &TypeContext,
) -> Result<Type, Vec<Diagnostic>> {
    let scrutinee_ty = infer_expr(scrutinee, env, ctx)?;
    let mut branch_ty: Option<Type> = None;

    let mut errors = Vec::new();
    let mut covered_all = false;
    let mut covered_bool = HashSet::new();
    let mut covered_ctor = HashSet::new();

    for arm in arms {
        if covered_all {
            errors.push(Diagnostic::new(
                "E-MATCH",
                "unreachable match arm",
                Some(arm.span.clone()),
            ));
            continue;
        }

        let mut arm_env = env.clone();
        let arm_key = bind_pattern(&arm.pattern, &scrutinee_ty, &mut arm_env, ctx)?;

        match &arm_key {
            PatternKey::Any => {
                covered_all = true;
            }
            PatternKey::Bool(v) => {
                if covered_bool.contains(v) {
                    errors.push(Diagnostic::new(
                        "E-MATCH",
                        "unreachable duplicate boolean pattern",
                        Some(arm.span.clone()),
                    ));
                }
                covered_bool.insert(*v);
            }
            PatternKey::Ctor(name) => {
                if covered_ctor.contains(name) {
                    errors.push(Diagnostic::new(
                        "E-MATCH",
                        format!("unreachable duplicate constructor pattern: {name}"),
                        Some(arm.span.clone()),
                    ));
                }
                covered_ctor.insert(name.clone());
            }
            PatternKey::Other => {}
        }

        let ty = infer_expr(&arm.body, &arm_env, ctx)?;
        if let Some(prev) = &branch_ty {
            if is_subtype(&ty, prev, ctx).is_ok() {
            } else if is_subtype(prev, &ty, ctx).is_ok() {
                branch_ty = Some(ty);
            } else {
                errors.push(Diagnostic::new(
                    "E-MATCH",
                    "match arms have incompatible result types",
                    Some(arm.span.clone()),
                ));
            }
        } else {
            branch_ty = Some(ty);
        }
    }

    if !is_exhaustive(
        &scrutinee_ty,
        covered_all,
        &covered_bool,
        &covered_ctor,
        ctx,
    ) {
        errors.push(Diagnostic::new(
            "E-MATCH",
            "non-exhaustive match",
            Some(span.clone()),
        ));
    }

    if errors.is_empty() {
        Ok(branch_ty.unwrap_or(Type::Bool))
    } else {
        Err(errors)
    }
}

#[derive(Debug, Clone)]
enum PatternKey {
    Any,
    Bool(bool),
    Ctor(String),
    Other,
}

fn bind_pattern(
    pattern: &Pattern,
    expected: &Type,
    env: &mut HashMap<String, Type>,
    ctx: &TypeContext,
) -> Result<PatternKey, Vec<Diagnostic>> {
    match pattern {
        Pattern::Wildcard { .. } => Ok(PatternKey::Any),
        Pattern::Var { name, .. } => {
            if let Some(prev) = env.get(name)
                && is_subtype(expected, prev, ctx).is_err()
            {
                return Err(vec![Diagnostic::new(
                    "E-MATCH",
                    format!("pattern variable type mismatch: {name}"),
                    Some(pattern.span().clone()),
                )]);
            }
            env.insert(name.clone(), expected.clone());
            Ok(PatternKey::Any)
        }
        Pattern::Bool { value, .. } => {
            ensure_subtype(
                expected,
                &Type::Bool,
                ctx,
                pattern.span(),
                "pattern expects Bool",
            )?;
            Ok(PatternKey::Bool(*value))
        }
        Pattern::Int { .. } => {
            ensure_subtype(
                expected,
                &Type::Int,
                ctx,
                pattern.span(),
                "pattern expects Int",
            )?;
            Ok(PatternKey::Other)
        }
        Pattern::Symbol { .. } => {
            ensure_subtype(
                expected,
                &Type::Symbol,
                ctx,
                pattern.span(),
                "pattern expects Symbol",
            )?;
            Ok(PatternKey::Other)
        }
        Pattern::Ctor { name, args, .. } => {
            let Some(sig) = ctx.constructor_sigs.get(name) else {
                return Err(vec![Diagnostic::new(
                    "E-MATCH",
                    format!("unknown constructor in pattern: {name}"),
                    Some(pattern.span().clone()),
                )]);
            };
            if sig.fields.len() != args.len() {
                return Err(vec![Diagnostic::new(
                    "E-MATCH",
                    format!(
                        "constructor {} arity mismatch in pattern: expected {}, got {}",
                        name,
                        sig.fields.len(),
                        args.len()
                    ),
                    Some(pattern.span().clone()),
                )]);
            }
            ensure_subtype(
                &sig.ret,
                expected,
                ctx,
                pattern.span(),
                "pattern constructor type mismatch",
            )?;

            for (child, child_expected) in args.iter().zip(sig.fields.iter()) {
                let _ = bind_pattern(child, child_expected, env, ctx)?;
            }
            Ok(PatternKey::Ctor(name.clone()))
        }
    }
}

fn is_exhaustive(
    scrutinee_ty: &Type,
    covered_all: bool,
    covered_bool: &HashSet<bool>,
    covered_ctor: &HashSet<String>,
    ctx: &TypeContext,
) -> bool {
    if covered_all {
        return true;
    }

    match scrutinee_ty {
        Type::Bool => covered_bool.contains(&true) && covered_bool.contains(&false),
        Type::Adt(name) => {
            let Some(ctors) = ctx.data_constructors.get(name) else {
                return false;
            };
            ctors.iter().all(|c| covered_ctor.contains(c))
        }
        _ => false,
    }
}

fn ensure_subtype(
    actual: &Type,
    expected: &Type,
    ctx: &TypeContext,
    span: &crate::diagnostics::Span,
    message: &str,
) -> Result<(), Vec<Diagnostic>> {
    is_subtype(actual, expected, ctx).map_err(|_| {
        vec![Diagnostic::new(
            "E-TYPE",
            format!("{message}: got {:?}, expected {:?}", actual, expected),
            Some(span.clone()),
        )]
    })
}

fn is_subtype(actual: &Type, expected: &Type, ctx: &TypeContext) -> Result<(), Diagnostic> {
    match (actual, expected) {
        (_, Type::Refine { var, base, formula }) => {
            let (left_base, left_formula) = match actual {
                Type::Refine {
                    var: av,
                    base: ab,
                    formula: af,
                } => {
                    let lhs = if av == var {
                        af.clone()
                    } else {
                        rename_formula_var(af, av, var)
                    };
                    (ab.as_ref(), lhs)
                }
                other => (other, Formula::True),
            };

            is_subtype(left_base, base, ctx)?;
            if entails(&left_formula, formula, ctx) {
                Ok(())
            } else {
                Err(Diagnostic::new(
                    "E-ENTAIL",
                    "refinement implication failed",
                    None,
                ))
            }
        }
        (Type::Refine { base, .. }, _) => is_subtype(base, expected, ctx),
        (Type::Bool, Type::Bool) | (Type::Int, Type::Int) | (Type::Symbol, Type::Symbol) => Ok(()),
        (Type::Domain(a), Type::Domain(b)) if a == b => Ok(()),
        (Type::Adt(a), Type::Adt(b)) if a == b => Ok(()),
        (Type::Fun(a_args, a_ret), Type::Fun(b_args, b_ret)) => {
            if a_args.len() != b_args.len() {
                return Err(Diagnostic::new("E-TYPE", "function arity mismatch", None));
            }
            for (a, b) in a_args.iter().zip(b_args.iter()) {
                if is_subtype(a, b, ctx).is_err() || is_subtype(b, a, ctx).is_err() {
                    return Err(Diagnostic::new(
                        "E-TYPE",
                        "function argument type mismatch",
                        None,
                    ));
                }
            }
            is_subtype(a_ret, b_ret, ctx)
        }
        _ => Err(Diagnostic::new("E-TYPE", "type mismatch", None)),
    }
}

fn entails(lhs: &Formula, rhs: &Formula, ctx: &TypeContext) -> bool {
    let vars = collect_vars(lhs)
        .into_iter()
        .chain(collect_vars(rhs))
        .collect::<HashSet<_>>();

    let var_map: HashMap<String, Value> = vars
        .into_iter()
        .map(|v| (v.clone(), Value::Symbol(format!("__v_{v}"))))
        .collect();

    let assumptions = positive_atoms(lhs)
        .into_iter()
        .filter_map(|a| atom_to_ground_fact(&a, &var_map))
        .collect::<Vec<_>>();

    let kb = ctx.kb_template.with_extra_facts(assumptions);
    let Ok(derived) = solve_facts(&kb) else {
        return false;
    };

    if !eval_formula(lhs, &derived, &var_map) {
        return true;
    }
    eval_formula(rhs, &derived, &var_map)
}

fn eval_formula(formula: &Formula, derived: &DerivedFacts, vars: &HashMap<String, Value>) -> bool {
    match formula {
        Formula::True => true,
        Formula::Atom(atom) => atom_to_ground_tuple(atom, vars)
            .map(|(pred, tuple)| {
                derived
                    .facts
                    .get(&pred)
                    .map(|set| set.contains(&tuple))
                    .unwrap_or(false)
            })
            .unwrap_or(false),
        Formula::And(items) => items.iter().all(|f| eval_formula(f, derived, vars)),
        Formula::Not(inner) => !eval_formula(inner, derived, vars),
    }
}

fn atom_to_ground_fact(atom: &Atom, vars: &HashMap<String, Value>) -> Option<GroundFact> {
    let (_, tuple) = atom_to_ground_tuple(atom, vars)?;
    Some(GroundFact {
        pred: atom.pred.clone(),
        terms: tuple,
    })
}

fn atom_to_ground_tuple(
    atom: &Atom,
    vars: &HashMap<String, Value>,
) -> Option<(String, Vec<Value>)> {
    let mut tuple = Vec::new();
    for term in &atom.terms {
        let v = logic_term_to_value(term, vars)?;
        tuple.push(v);
    }
    Some((atom.pred.clone(), tuple))
}

fn logic_term_to_value(term: &LogicTerm, vars: &HashMap<String, Value>) -> Option<Value> {
    match term {
        LogicTerm::Var(name) => vars.get(name).cloned(),
        LogicTerm::Symbol(s) => Some(Value::Symbol(s.clone())),
        LogicTerm::Int(i) => Some(Value::Int(*i)),
        LogicTerm::Bool(b) => Some(Value::Bool(*b)),
        LogicTerm::Ctor { name, args } => {
            let mut fields = Vec::new();
            for arg in args {
                fields.push(logic_term_to_value(arg, vars)?);
            }
            Some(Value::Adt {
                ctor: name.clone(),
                fields,
            })
        }
    }
}

fn positive_atoms(formula: &Formula) -> Vec<Atom> {
    let mut out = Vec::new();
    collect_positive_atoms(formula, false, &mut out);
    out
}

fn collect_positive_atoms(formula: &Formula, neg: bool, out: &mut Vec<Atom>) {
    match formula {
        Formula::True => {}
        Formula::Atom(atom) => {
            if !neg {
                out.push(atom.clone());
            }
        }
        Formula::And(items) => {
            for item in items {
                collect_positive_atoms(item, neg, out);
            }
        }
        Formula::Not(inner) => collect_positive_atoms(inner, !neg, out),
    }
}

fn collect_vars(formula: &Formula) -> HashSet<String> {
    let mut vars = HashSet::new();
    collect_vars_inner(formula, &mut vars);
    vars
}

fn collect_vars_inner(formula: &Formula, out: &mut HashSet<String>) {
    match formula {
        Formula::True => {}
        Formula::Atom(atom) => {
            for t in &atom.terms {
                collect_vars_in_term(t, out);
            }
        }
        Formula::And(items) => {
            for item in items {
                collect_vars_inner(item, out);
            }
        }
        Formula::Not(inner) => collect_vars_inner(inner, out),
    }
}

fn collect_vars_in_term(term: &LogicTerm, out: &mut HashSet<String>) {
    match term {
        LogicTerm::Var(v) => {
            out.insert(v.clone());
        }
        LogicTerm::Ctor { args, .. } => {
            for arg in args {
                collect_vars_in_term(arg, out);
            }
        }
        LogicTerm::Symbol(_) | LogicTerm::Int(_) | LogicTerm::Bool(_) => {}
    }
}

fn rename_formula_var(formula: &Formula, from: &str, to: &str) -> Formula {
    match formula {
        Formula::True => Formula::True,
        Formula::Atom(atom) => Formula::Atom(Atom {
            pred: atom.pred.clone(),
            terms: atom
                .terms
                .iter()
                .map(|t| rename_term_var(t, from, to))
                .collect(),
        }),
        Formula::And(items) => Formula::And(
            items
                .iter()
                .map(|f| rename_formula_var(f, from, to))
                .collect(),
        ),
        Formula::Not(inner) => Formula::Not(Box::new(rename_formula_var(inner, from, to))),
    }
}

fn rename_term_var(term: &LogicTerm, from: &str, to: &str) -> LogicTerm {
    match term {
        LogicTerm::Var(v) if v == from => LogicTerm::Var(to.to_string()),
        LogicTerm::Ctor { name, args } => LogicTerm::Ctor {
            name: name.clone(),
            args: args.iter().map(|t| rename_term_var(t, from, to)).collect(),
        },
        other => other.clone(),
    }
}

fn substitute_type(ty: &Type, subst: &HashMap<String, LogicTerm>) -> Type {
    match ty {
        Type::Bool => Type::Bool,
        Type::Int => Type::Int,
        Type::Symbol => Type::Symbol,
        Type::Domain(s) => Type::Domain(s.clone()),
        Type::Adt(s) => Type::Adt(s.clone()),
        Type::Fun(args, ret) => Type::Fun(
            args.iter().map(|a| substitute_type(a, subst)).collect(),
            Box::new(substitute_type(ret, subst)),
        ),
        Type::Refine { var, base, formula } => {
            let mut next = subst.clone();
            next.remove(var);
            Type::Refine {
                var: var.clone(),
                base: Box::new(substitute_type(base, &next)),
                formula: substitute_formula(formula, &next),
            }
        }
    }
}

fn substitute_formula(formula: &Formula, subst: &HashMap<String, LogicTerm>) -> Formula {
    match formula {
        Formula::True => Formula::True,
        Formula::Atom(atom) => Formula::Atom(Atom {
            pred: atom.pred.clone(),
            terms: atom
                .terms
                .iter()
                .map(|t| substitute_term(t, subst))
                .collect(),
        }),
        Formula::And(items) => {
            Formula::And(items.iter().map(|f| substitute_formula(f, subst)).collect())
        }
        Formula::Not(inner) => Formula::Not(Box::new(substitute_formula(inner, subst))),
    }
}

fn substitute_term(term: &LogicTerm, subst: &HashMap<String, LogicTerm>) -> LogicTerm {
    match term {
        LogicTerm::Var(v) => subst
            .get(v)
            .cloned()
            .unwrap_or_else(|| LogicTerm::Var(v.clone())),
        LogicTerm::Ctor { name, args } => LogicTerm::Ctor {
            name: name.clone(),
            args: args.iter().map(|t| substitute_term(t, subst)).collect(),
        },
        other => other.clone(),
    }
}

fn expr_to_logic_term(expr: &Expr, ctx: &TypeContext) -> Option<LogicTerm> {
    match expr {
        Expr::Var { name, .. } => Some(LogicTerm::Var(name.clone())),
        Expr::Symbol { value, .. } => Some(LogicTerm::Symbol(value.clone())),
        Expr::Int { value, .. } => Some(LogicTerm::Int(*value)),
        Expr::Bool { value, .. } => Some(LogicTerm::Bool(*value)),
        Expr::Call { name, args, .. } => {
            if !ctx.constructor_sigs.contains_key(name) {
                return None;
            }
            let mut terms = Vec::new();
            for arg in args {
                terms.push(expr_to_logic_term(arg, ctx)?);
            }
            Some(LogicTerm::Ctor {
                name: name.clone(),
                args: terms,
            })
        }
        Expr::Let { .. } | Expr::If { .. } | Expr::Match { .. } => None,
    }
}

fn canonicalize_type_for_ctx(ty: &Type, ctx: &TypeContext) -> Type {
    let data_names: HashSet<String> = ctx.data_constructors.keys().cloned().collect();
    canonicalize_type(ty, &data_names)
}

fn canonicalize_type(ty: &Type, data_names: &HashSet<String>) -> Type {
    match ty {
        Type::Domain(name) if data_names.contains(name) => Type::Adt(name.clone()),
        Type::Domain(name) => Type::Domain(name.clone()),
        Type::Adt(name) => Type::Adt(name.clone()),
        Type::Bool => Type::Bool,
        Type::Int => Type::Int,
        Type::Symbol => Type::Symbol,
        Type::Fun(args, ret) => Type::Fun(
            args.iter()
                .map(|a| canonicalize_type(a, data_names))
                .collect(),
            Box::new(canonicalize_type(ret, data_names)),
        ),
        Type::Refine { var, base, formula } => Type::Refine {
            var: var.clone(),
            base: Box::new(canonicalize_type(base, data_names)),
            formula: formula.clone(),
        },
    }
}

fn type_from_name(name: &str, data_names: &HashSet<String>) -> Type {
    match name {
        "Bool" => Type::Bool,
        "Int" => Type::Int,
        "Symbol" => Type::Symbol,
        n if data_names.contains(n) => Type::Adt(n.to_string()),
        other => Type::Domain(other.to_string()),
    }
}
