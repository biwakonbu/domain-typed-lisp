#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use dtl::ast::{AssertDecl, Defn, Expr, Param, Pattern, Program};
use dtl::stratify::compute_strata;
use dtl::types::{Atom, Formula, LogicTerm, Type};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReferenceValue {
    Symbol(String),
    Int(i64),
    Bool(bool),
    Adt {
        ctor: String,
        fields: Vec<ReferenceValue>,
    },
}

pub type ReferenceEnv = BTreeMap<String, ReferenceValue>;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReferenceDerivedFacts {
    pub facts: BTreeMap<String, BTreeSet<Vec<ReferenceValue>>>,
}

impl ReferenceDerivedFacts {
    pub fn relation_facts(&self, pred: &str) -> BTreeSet<Vec<String>> {
        self.facts
            .get(pred)
            .map(|tuples| {
                tuples
                    .iter()
                    .map(|tuple| tuple.iter().map(reference_value_to_string).collect())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn all_fact_strings(&self) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        for (pred, tuples) in &self.facts {
            for tuple in tuples {
                out.insert(reference_ground_fact_key(pred, tuple));
            }
        }
        out
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceObligationResult {
    pub id: String,
    pub kind: String,
    pub result: String,
    pub valuation: ReferenceEnv,
    pub premises: BTreeSet<String>,
    pub missing_goals: BTreeSet<String>,
}

#[derive(Debug, Clone)]
struct ReferenceObligation<'a> {
    id: String,
    kind: String,
    goal: &'a Formula,
    params: &'a [Param],
    body: ReferenceObligationBody<'a>,
}

#[derive(Debug, Clone)]
enum ReferenceObligationBody<'a> {
    Assert,
    Refine { expr: &'a Expr, result_var: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReferenceExprResult {
    value: ReferenceValue,
    positive_facts: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ReferenceCallKey {
    name: String,
    args: Vec<ReferenceValue>,
}

struct ReferenceEvalState<'a> {
    derived: &'a ReferenceDerivedFacts,
    relation_names: &'a HashSet<String>,
    constructor_names: &'a HashSet<String>,
    defn_map: &'a HashMap<String, &'a Defn>,
    cache: HashMap<ReferenceCallKey, ReferenceExprResult>,
    active_calls: HashSet<ReferenceCallKey>,
}

pub fn reference_solve_facts(program: &Program) -> Result<ReferenceDerivedFacts, String> {
    let relation_schemas = program
        .relations
        .iter()
        .map(|rel| (rel.name.clone(), rel.arg_sorts.len()))
        .collect::<HashMap<_, _>>();
    let strata = compute_strata(program).map_err(render_diagnostics)?;

    let mut db = program
        .relations
        .iter()
        .map(|rel| (rel.name.clone(), BTreeSet::new()))
        .collect::<BTreeMap<_, _>>();

    for fact in &program.facts {
        let Some(expected_arity) = relation_schemas.get(&fact.name) else {
            return Err(format!("undefined relation in fact: {}", fact.name));
        };
        if *expected_arity != fact.terms.len() {
            return Err(format!(
                "arity mismatch in fact {}: expected {}, got {}",
                fact.name,
                expected_arity,
                fact.terms.len()
            ));
        }
        let tuple = fact
            .terms
            .iter()
            .map(logic_term_to_reference_const)
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| format!("fact contains variable: {}", fact.name))?;
        db.entry(fact.name.clone()).or_default().insert(tuple);
    }

    let mut stratum_values = strata.values().copied().collect::<Vec<_>>();
    stratum_values.sort_unstable();
    stratum_values.dedup();

    for stratum in stratum_values {
        let rules = program
            .rules
            .iter()
            .filter(|rule| strata.get(&rule.head.pred).copied().unwrap_or(0) == stratum)
            .collect::<Vec<_>>();
        let mut changed = true;
        while changed {
            changed = false;
            for rule in &rules {
                let produced = reference_apply_rule(rule.head.clone(), &rule.body, &db)?;
                let target = db.entry(rule.head.pred.clone()).or_default();
                for tuple in produced {
                    if target.insert(tuple) {
                        changed = true;
                    }
                }
            }
        }
    }

    Ok(ReferenceDerivedFacts { facts: db })
}

pub fn reference_eval_formula(
    formula: &Formula,
    env: &ReferenceEnv,
    derived: &ReferenceDerivedFacts,
) -> Result<bool, String> {
    match formula {
        Formula::True => Ok(true),
        Formula::Atom(atom) => {
            let tuple = instantiate_logic_terms(&atom.terms, env)?;
            Ok(derived
                .facts
                .get(&atom.pred)
                .map(|tuples| tuples.contains(&tuple))
                .unwrap_or(false))
        }
        Formula::And(items) => {
            for item in items {
                if !reference_eval_formula(item, env, derived)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        Formula::Not(inner) => Ok(!reference_eval_formula(inner, env, derived)?),
    }
}

pub fn reference_eval_expr(
    expr: &Expr,
    env: &ReferenceEnv,
    derived: &ReferenceDerivedFacts,
    defn_map: &HashMap<String, &Defn>,
    relation_names: &HashSet<String>,
    constructor_names: &HashSet<String>,
) -> Result<(ReferenceValue, BTreeSet<String>), String> {
    let mut state = ReferenceEvalState {
        derived,
        relation_names,
        constructor_names,
        defn_map,
        cache: HashMap::new(),
        active_calls: HashSet::new(),
    };
    let result = reference_eval_expr_inner(expr, env, &mut state)?;
    Ok((result.value, result.positive_facts))
}

pub fn reference_prove_program(
    program: &Program,
) -> Result<Vec<ReferenceObligationResult>, String> {
    let derived = reference_solve_facts(program)?;
    let universe_map = build_universe_map(program)?;
    let relation_names = program
        .relations
        .iter()
        .map(|rel| rel.name.clone())
        .collect::<HashSet<_>>();
    let constructor_names = program
        .data_decls
        .iter()
        .flat_map(|decl| decl.constructors.iter().map(|ctor| ctor.name.clone()))
        .collect::<HashSet<_>>();
    let defn_map = program
        .defns
        .iter()
        .map(|defn| (defn.name.clone(), defn))
        .collect::<HashMap<_, _>>();

    let mut out = Vec::new();
    for obligation in build_obligations(program) {
        let valuations = enumerate_valuations(obligation.params, &universe_map)?;
        let mut failure = None;
        for valuation in valuations {
            let result = match &obligation.body {
                ReferenceObligationBody::Assert => {
                    if reference_eval_formula(obligation.goal, &valuation, &derived)? {
                        None
                    } else {
                        Some(ReferenceObligationResult {
                            id: obligation.id.clone(),
                            kind: obligation.kind.clone(),
                            result: "failed".to_string(),
                            valuation: valuation.clone(),
                            premises: BTreeSet::new(),
                            missing_goals: collect_missing_goals(
                                obligation.goal,
                                &valuation,
                                &derived,
                            )?,
                        })
                    }
                }
                ReferenceObligationBody::Refine { expr, result_var } => {
                    let (value, positive_facts) = reference_eval_expr(
                        expr,
                        &valuation,
                        &derived,
                        &defn_map,
                        &relation_names,
                        &constructor_names,
                    )?;
                    let mut goal_env = valuation.clone();
                    goal_env.insert(result_var.clone(), value.clone());
                    match value {
                        ReferenceValue::Bool(true) => {
                            if reference_eval_formula(obligation.goal, &goal_env, &derived)? {
                                None
                            } else {
                                Some(ReferenceObligationResult {
                                    id: obligation.id.clone(),
                                    kind: obligation.kind.clone(),
                                    result: "failed".to_string(),
                                    valuation: valuation.clone(),
                                    premises: positive_facts,
                                    missing_goals: collect_missing_goals(
                                        obligation.goal,
                                        &goal_env,
                                        &derived,
                                    )?,
                                })
                            }
                        }
                        ReferenceValue::Bool(false) => None,
                        other => {
                            return Err(format!(
                                "refine body did not evaluate to Bool: {}",
                                reference_value_to_string(&other)
                            ));
                        }
                    }
                }
            };
            if result.is_some() {
                failure = result;
                break;
            }
        }

        out.push(failure.unwrap_or_else(|| ReferenceObligationResult {
            id: obligation.id,
            kind: obligation.kind,
            result: "proved".to_string(),
            valuation: ReferenceEnv::new(),
            premises: BTreeSet::new(),
            missing_goals: BTreeSet::new(),
        }));
    }

    Ok(out)
}

pub fn reference_check_assert(
    program: &Program,
    assertion: &AssertDecl,
) -> Result<ReferenceObligationResult, String> {
    reference_prove_program(program)?
        .into_iter()
        .find(|item| item.id == format!("assert::{}", assertion.name))
        .ok_or_else(|| format!("missing obligation for assert::{}", assertion.name))
}

pub fn reference_check_refine(
    program: &Program,
    defn: &Defn,
) -> Result<ReferenceObligationResult, String> {
    reference_prove_program(program)?
        .into_iter()
        .find(|item| item.id == format!("defn::{}", defn.name))
        .ok_or_else(|| format!("missing obligation for defn::{}", defn.name))
}

pub fn reference_value_to_string(value: &ReferenceValue) -> String {
    match value {
        ReferenceValue::Symbol(symbol) => symbol.clone(),
        ReferenceValue::Int(value) => value.to_string(),
        ReferenceValue::Bool(value) => value.to_string(),
        ReferenceValue::Adt { ctor, fields } => {
            let mut rendered = format!("({ctor}");
            for field in fields {
                rendered.push(' ');
                rendered.push_str(&reference_value_to_string(field));
            }
            rendered.push(')');
            rendered
        }
    }
}

fn build_obligations(program: &Program) -> Vec<ReferenceObligation<'_>> {
    let mut obligations = Vec::new();
    for defn in &program.defns {
        if let Type::Refine { var, formula, .. } = &defn.ret_type {
            obligations.push(ReferenceObligation {
                id: format!("defn::{}", defn.name),
                kind: "defn".to_string(),
                goal: formula,
                params: &defn.params,
                body: ReferenceObligationBody::Refine {
                    expr: &defn.body,
                    result_var: var.clone(),
                },
            });
        }
    }
    for assertion in &program.asserts {
        obligations.push(ReferenceObligation {
            id: format!("assert::{}", assertion.name),
            kind: "assert".to_string(),
            goal: &assertion.formula,
            params: &assertion.params,
            body: ReferenceObligationBody::Assert,
        });
    }
    obligations
}

fn build_universe_map(program: &Program) -> Result<BTreeMap<String, Vec<ReferenceValue>>, String> {
    let mut map = BTreeMap::new();
    for universe in &program.universes {
        let values = universe
            .values
            .iter()
            .map(logic_term_to_reference_const)
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| format!("universe value contains variable: {}", universe.ty_name))?;
        map.insert(universe.ty_name.clone(), values);
    }
    Ok(map)
}

fn enumerate_valuations(
    params: &[Param],
    universe_map: &BTreeMap<String, Vec<ReferenceValue>>,
) -> Result<Vec<ReferenceEnv>, String> {
    let mut domains = Vec::new();
    for param in params {
        let key = type_key(&param.ty)?;
        let Some(values) = universe_map.get(&key) else {
            return Err(format!("missing universe declaration for type: {key}"));
        };
        if values.is_empty() {
            return Err(format!("universe for type {key} must not be empty"));
        }
        domains.push((param.name.clone(), values.clone()));
    }

    let mut out = Vec::new();
    let mut current = ReferenceEnv::new();
    enumerate_cartesian(&domains, 0, &mut current, &mut out);
    Ok(out)
}

fn enumerate_cartesian(
    domains: &[(String, Vec<ReferenceValue>)],
    idx: usize,
    current: &mut ReferenceEnv,
    out: &mut Vec<ReferenceEnv>,
) {
    if idx == domains.len() {
        out.push(current.clone());
        return;
    }
    let (name, values) = &domains[idx];
    for value in values {
        current.insert(name.clone(), value.clone());
        enumerate_cartesian(domains, idx + 1, current, out);
    }
}

fn type_key(ty: &Type) -> Result<String, String> {
    match ty {
        Type::Bool => Ok("Bool".to_string()),
        Type::Int => Ok("Int".to_string()),
        Type::Symbol => Ok("Symbol".to_string()),
        Type::Domain(name) | Type::Adt(name) => Ok(name.clone()),
        Type::Refine { base, .. } => type_key(base),
        Type::Fun(_, _) => {
            Err("function-typed quantified variables are unsupported in phase1".to_string())
        }
    }
}

fn reference_apply_rule(
    head: Atom,
    body: &Formula,
    db: &BTreeMap<String, BTreeSet<Vec<ReferenceValue>>>,
) -> Result<BTreeSet<Vec<ReferenceValue>>, String> {
    let mut positives = Vec::new();
    let mut negatives = Vec::new();
    flatten_formula(body, false, &mut positives, &mut negatives);

    let mut envs = vec![ReferenceEnv::new()];
    for atom in positives {
        let tuples = db.get(&atom.pred).cloned().unwrap_or_default();
        let mut next = Vec::new();
        for env in &envs {
            for tuple in &tuples {
                if let Some(bound) = unify_atom(atom, tuple, env) {
                    next.push(bound);
                }
            }
        }
        envs = next;
        if envs.is_empty() {
            break;
        }
    }

    for atom in negatives {
        let tuples = db.get(&atom.pred).cloned().unwrap_or_default();
        envs.retain(|env| {
            instantiate_logic_terms(&atom.terms, env)
                .map(|tuple| !tuples.contains(&tuple))
                .unwrap_or(false)
        });
    }

    let mut produced = BTreeSet::new();
    for env in &envs {
        produced.insert(instantiate_logic_terms(&head.terms, env)?);
    }
    Ok(produced)
}

fn flatten_formula<'a>(
    formula: &'a Formula,
    negated: bool,
    positives: &mut Vec<&'a Atom>,
    negatives: &mut Vec<&'a Atom>,
) {
    match formula {
        Formula::True => {}
        Formula::Atom(atom) => {
            if negated {
                negatives.push(atom);
            } else {
                positives.push(atom);
            }
        }
        Formula::And(items) => {
            for item in items {
                flatten_formula(item, negated, positives, negatives);
            }
        }
        Formula::Not(inner) => flatten_formula(inner, !negated, positives, negatives),
    }
}

fn unify_atom(atom: &Atom, tuple: &[ReferenceValue], env: &ReferenceEnv) -> Option<ReferenceEnv> {
    if atom.terms.len() != tuple.len() {
        return None;
    }
    let mut bound = env.clone();
    for (term, value) in atom.terms.iter().zip(tuple.iter()) {
        if !unify_logic_term(term, value, &mut bound) {
            return None;
        }
    }
    Some(bound)
}

fn unify_logic_term(term: &LogicTerm, value: &ReferenceValue, env: &mut ReferenceEnv) -> bool {
    match term {
        LogicTerm::Var(name) => match env.get(name) {
            Some(bound) => bound == value,
            None => {
                env.insert(name.clone(), value.clone());
                true
            }
        },
        LogicTerm::Symbol(expected) => {
            matches!(value, ReferenceValue::Symbol(actual) if actual == expected)
        }
        LogicTerm::Int(expected) => {
            matches!(value, ReferenceValue::Int(actual) if actual == expected)
        }
        LogicTerm::Bool(expected) => {
            matches!(value, ReferenceValue::Bool(actual) if actual == expected)
        }
        LogicTerm::Ctor { name, args } => match value {
            ReferenceValue::Adt { ctor, fields } if ctor == name && fields.len() == args.len() => {
                for (arg, field) in args.iter().zip(fields.iter()) {
                    if !unify_logic_term(arg, field, env) {
                        return false;
                    }
                }
                true
            }
            _ => false,
        },
    }
}

fn instantiate_logic_terms(
    terms: &[LogicTerm],
    env: &ReferenceEnv,
) -> Result<Vec<ReferenceValue>, String> {
    terms
        .iter()
        .map(|term| instantiate_logic_term(term, env))
        .collect()
}

fn instantiate_logic_term(term: &LogicTerm, env: &ReferenceEnv) -> Result<ReferenceValue, String> {
    match term {
        LogicTerm::Var(name) => env
            .get(name)
            .cloned()
            .ok_or_else(|| format!("unbound variable during instantiation: {name}")),
        LogicTerm::Symbol(symbol) => Ok(ReferenceValue::Symbol(symbol.clone())),
        LogicTerm::Int(value) => Ok(ReferenceValue::Int(*value)),
        LogicTerm::Bool(value) => Ok(ReferenceValue::Bool(*value)),
        LogicTerm::Ctor { name, args } => Ok(ReferenceValue::Adt {
            ctor: name.clone(),
            fields: args
                .iter()
                .map(|arg| instantiate_logic_term(arg, env))
                .collect::<Result<Vec<_>, _>>()?,
        }),
    }
}

fn logic_term_to_reference_const(term: &LogicTerm) -> Option<ReferenceValue> {
    match term {
        LogicTerm::Var(_) => None,
        LogicTerm::Symbol(symbol) => Some(ReferenceValue::Symbol(symbol.clone())),
        LogicTerm::Int(value) => Some(ReferenceValue::Int(*value)),
        LogicTerm::Bool(value) => Some(ReferenceValue::Bool(*value)),
        LogicTerm::Ctor { name, args } => Some(ReferenceValue::Adt {
            ctor: name.clone(),
            fields: args
                .iter()
                .map(logic_term_to_reference_const)
                .collect::<Option<Vec<_>>>()?,
        }),
    }
}

fn reference_eval_expr_inner(
    expr: &Expr,
    env: &ReferenceEnv,
    state: &mut ReferenceEvalState<'_>,
) -> Result<ReferenceExprResult, String> {
    match expr {
        Expr::Var { name, .. } => env
            .get(name)
            .cloned()
            .map(|value| ReferenceExprResult {
                value,
                positive_facts: BTreeSet::new(),
            })
            .ok_or_else(|| format!("unknown variable during expression evaluation: {name}")),
        Expr::Symbol { value, .. } => Ok(ReferenceExprResult {
            value: ReferenceValue::Symbol(value.clone()),
            positive_facts: BTreeSet::new(),
        }),
        Expr::Int { value, .. } => Ok(ReferenceExprResult {
            value: ReferenceValue::Int(*value),
            positive_facts: BTreeSet::new(),
        }),
        Expr::Bool { value, .. } => Ok(ReferenceExprResult {
            value: ReferenceValue::Bool(*value),
            positive_facts: BTreeSet::new(),
        }),
        Expr::Call { name, args, .. } if state.relation_names.contains(name) => {
            let mut arg_values = Vec::new();
            let mut positive_facts = BTreeSet::new();
            for arg in args {
                let result = reference_eval_expr_inner(arg, env, state)?;
                positive_facts.extend(result.positive_facts);
                arg_values.push(result.value);
            }
            let truth = state
                .derived
                .facts
                .get(name)
                .map(|tuples| tuples.contains(&arg_values))
                .unwrap_or(false);
            if truth {
                positive_facts.insert(reference_ground_fact_key(name, &arg_values));
            }
            Ok(ReferenceExprResult {
                value: ReferenceValue::Bool(truth),
                positive_facts,
            })
        }
        Expr::Call { name, args, .. } if state.constructor_names.contains(name) => {
            let mut fields = Vec::new();
            let mut positive_facts = BTreeSet::new();
            for arg in args {
                let result = reference_eval_expr_inner(arg, env, state)?;
                positive_facts.extend(result.positive_facts);
                fields.push(result.value);
            }
            Ok(ReferenceExprResult {
                value: ReferenceValue::Adt {
                    ctor: name.clone(),
                    fields,
                },
                positive_facts,
            })
        }
        Expr::Call { name, args, .. } => {
            let Some(defn) = state.defn_map.get(name) else {
                return Err(format!(
                    "unknown call target during expression evaluation: {name}"
                ));
            };
            if defn.params.len() != args.len() {
                return Err(format!(
                    "arity mismatch during expression evaluation: {} expected {}, got {}",
                    name,
                    defn.params.len(),
                    args.len()
                ));
            }

            let mut arg_values = Vec::new();
            let mut arg_facts = BTreeSet::new();
            for arg in args {
                let result = reference_eval_expr_inner(arg, env, state)?;
                arg_facts.extend(result.positive_facts);
                arg_values.push(result.value);
            }

            let key = ReferenceCallKey {
                name: name.clone(),
                args: arg_values.clone(),
            };
            if let Some(cached) = state.cache.get(&key) {
                let mut positive_facts = arg_facts;
                positive_facts.extend(cached.positive_facts.clone());
                return Ok(ReferenceExprResult {
                    value: cached.value.clone(),
                    positive_facts,
                });
            }
            if !state.active_calls.insert(key.clone()) {
                return Err(format!("recursive evaluation cycle detected in {name}"));
            }

            let mut call_env = ReferenceEnv::new();
            for (param, value) in defn.params.iter().zip(arg_values.iter()) {
                call_env.insert(param.name.clone(), value.clone());
            }
            let result = reference_eval_expr_inner(&defn.body, &call_env, state)?;
            state.active_calls.remove(&key);
            state.cache.insert(key, result.clone());

            let mut positive_facts = arg_facts;
            positive_facts.extend(result.positive_facts.clone());
            Ok(ReferenceExprResult {
                value: result.value,
                positive_facts,
            })
        }
        Expr::Let { bindings, body, .. } => {
            let mut local_env = env.clone();
            let mut positive_facts = BTreeSet::new();
            for (name, bound_expr, _) in bindings {
                let result = reference_eval_expr_inner(bound_expr, &local_env, state)?;
                positive_facts.extend(result.positive_facts);
                local_env.insert(name.clone(), result.value);
            }
            let result = reference_eval_expr_inner(body, &local_env, state)?;
            positive_facts.extend(result.positive_facts.clone());
            Ok(ReferenceExprResult {
                value: result.value,
                positive_facts,
            })
        }
        Expr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            let cond_result = reference_eval_expr_inner(cond, env, state)?;
            match cond_result.value {
                ReferenceValue::Bool(true) => {
                    let branch = reference_eval_expr_inner(then_branch, env, state)?;
                    let mut positive_facts = cond_result.positive_facts;
                    positive_facts.extend(branch.positive_facts.clone());
                    Ok(ReferenceExprResult {
                        value: branch.value,
                        positive_facts,
                    })
                }
                ReferenceValue::Bool(false) => {
                    let branch = reference_eval_expr_inner(else_branch, env, state)?;
                    let mut positive_facts = cond_result.positive_facts;
                    positive_facts.extend(branch.positive_facts.clone());
                    Ok(ReferenceExprResult {
                        value: branch.value,
                        positive_facts,
                    })
                }
                other => Err(format!(
                    "if condition did not evaluate to Bool: {}",
                    reference_value_to_string(&other)
                )),
            }
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            let scrutinee_result = reference_eval_expr_inner(scrutinee, env, state)?;
            for arm in arms {
                let mut branch_env = env.clone();
                if bind_pattern(&arm.pattern, &scrutinee_result.value, &mut branch_env) {
                    let branch_result = reference_eval_expr_inner(&arm.body, &branch_env, state)?;
                    let mut positive_facts = scrutinee_result.positive_facts.clone();
                    positive_facts.extend(branch_result.positive_facts.clone());
                    return Ok(ReferenceExprResult {
                        value: branch_result.value,
                        positive_facts,
                    });
                }
            }
            Err("match expression had no matching arm during reference evaluation".to_string())
        }
    }
}

fn bind_pattern(pattern: &Pattern, value: &ReferenceValue, env: &mut ReferenceEnv) -> bool {
    match pattern {
        Pattern::Wildcard { .. } => true,
        Pattern::Var { name, .. } => match env.get(name) {
            Some(bound) => bound == value,
            None => {
                env.insert(name.clone(), value.clone());
                true
            }
        },
        Pattern::Symbol {
            value: expected, ..
        } => matches!(value, ReferenceValue::Symbol(actual) if actual == expected),
        Pattern::Int {
            value: expected, ..
        } => matches!(value, ReferenceValue::Int(actual) if actual == expected),
        Pattern::Bool {
            value: expected, ..
        } => matches!(value, ReferenceValue::Bool(actual) if actual == expected),
        Pattern::Ctor { name, args, .. } => match value {
            ReferenceValue::Adt { ctor, fields } if ctor == name && fields.len() == args.len() => {
                for (arg, field) in args.iter().zip(fields.iter()) {
                    if !bind_pattern(arg, field, env) {
                        return false;
                    }
                }
                true
            }
            _ => false,
        },
    }
}

fn collect_missing_goals(
    formula: &Formula,
    env: &ReferenceEnv,
    derived: &ReferenceDerivedFacts,
) -> Result<BTreeSet<String>, String> {
    let mut out = BTreeSet::new();
    collect_missing_goals_inner(formula, env, derived, &mut out)?;
    Ok(out)
}

fn collect_missing_goals_inner(
    formula: &Formula,
    env: &ReferenceEnv,
    derived: &ReferenceDerivedFacts,
    out: &mut BTreeSet<String>,
) -> Result<(), String> {
    match formula {
        Formula::True => Ok(()),
        Formula::Atom(atom) => {
            let tuple = instantiate_logic_terms(&atom.terms, env)?;
            let exists = derived
                .facts
                .get(&atom.pred)
                .map(|tuples| tuples.contains(&tuple))
                .unwrap_or(false);
            if !exists {
                out.insert(reference_ground_fact_key(&atom.pred, &tuple));
            }
            Ok(())
        }
        Formula::And(items) => {
            for item in items {
                collect_missing_goals_inner(item, env, derived, out)?;
            }
            Ok(())
        }
        Formula::Not(inner) => {
            if reference_eval_formula(inner, env, derived)? {
                out.insert(format!("not {}", grounded_formula_to_string(inner, env)?));
            }
            Ok(())
        }
    }
}

fn reference_ground_fact_key(pred: &str, tuple: &[ReferenceValue]) -> String {
    let args = tuple
        .iter()
        .map(reference_value_to_string)
        .collect::<Vec<_>>()
        .join(",");
    format!("{pred}({args})")
}

fn reference_formula_to_string(formula: &Formula) -> String {
    match formula {
        Formula::True => "true".to_string(),
        Formula::Atom(atom) => {
            if atom.terms.is_empty() {
                format!("({})", atom.pred)
            } else {
                format!(
                    "({} {})",
                    atom.pred,
                    atom.terms
                        .iter()
                        .map(reference_logic_term_to_string)
                        .collect::<Vec<_>>()
                        .join(" ")
                )
            }
        }
        Formula::And(items) => format!(
            "(and {})",
            items
                .iter()
                .map(reference_formula_to_string)
                .collect::<Vec<_>>()
                .join(" ")
        ),
        Formula::Not(inner) => format!("(not {})", reference_formula_to_string(inner)),
    }
}

fn grounded_formula_to_string(formula: &Formula, env: &ReferenceEnv) -> Result<String, String> {
    match formula {
        Formula::True => Ok("true".to_string()),
        Formula::Atom(atom) => {
            let tuple = instantiate_logic_terms(&atom.terms, env)?;
            let rendered = tuple
                .iter()
                .map(reference_value_to_string)
                .collect::<Vec<_>>()
                .join(" ");
            if rendered.is_empty() {
                Ok(format!("({})", atom.pred))
            } else {
                Ok(format!("({} {})", atom.pred, rendered))
            }
        }
        Formula::And(items) => Ok(format!(
            "(and {})",
            items
                .iter()
                .map(|item| grounded_formula_to_string(item, env))
                .collect::<Result<Vec<_>, _>>()?
                .join(" ")
        )),
        Formula::Not(inner) => Ok(format!("(not {})", grounded_formula_to_string(inner, env)?)),
    }
}

fn reference_logic_term_to_string(term: &LogicTerm) -> String {
    match term {
        LogicTerm::Var(name) => name.clone(),
        LogicTerm::Symbol(symbol) => symbol.clone(),
        LogicTerm::Int(value) => value.to_string(),
        LogicTerm::Bool(value) => value.to_string(),
        LogicTerm::Ctor { name, args } => {
            let mut rendered = format!("({name}");
            for arg in args {
                rendered.push(' ');
                rendered.push_str(&reference_logic_term_to_string(arg));
            }
            rendered.push(')');
            rendered
        }
    }
}

fn render_diagnostics(diags: Vec<dtl::Diagnostic>) -> String {
    diags
        .into_iter()
        .map(|diag| diag.message)
        .collect::<Vec<_>>()
        .join("; ")
}
