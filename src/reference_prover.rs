use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use crate::ast::{Defn, Expr, Param, Pattern, Program};
use crate::diagnostics::Diagnostic;
use crate::logic_engine::{GroundFact, Value};
use crate::name_resolve::{normalize_program_aliases, resolve_program};
use crate::prover::{
    ClaimCoverage, CounterexampleTrace, NameValue, ObligationTrace, PROOF_TRACE_SCHEMA_VERSION,
    ProofSummary, ProofTrace,
};
use crate::stratify::compute_strata;
use crate::typecheck::check_program;
use crate::types::{Atom, Formula, LogicTerm, Type};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FunctionValue {
    table: Vec<(Vec<ReferenceValue>, ReferenceValue)>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReferenceValue {
    Symbol(String),
    Int(i64),
    Bool(bool),
    Adt {
        ctor: String,
        fields: Vec<ReferenceValue>,
    },
    Function(FunctionValue),
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

const MAX_FUNCTION_MODEL_VALUES: usize = 4096;

pub fn prove_program_reference(program: &Program) -> Result<ProofTrace, Vec<Diagnostic>> {
    let normalized = prepare_program(program)?;
    let results = reference_prove_program_results(&normalized)?;
    let derived = reference_solve_facts(&normalized)?;

    let obligations = results
        .into_iter()
        .map(|result| {
            let failed = result.result == "failed";
            ObligationTrace {
                id: result.id,
                kind: result.kind,
                result: result.result,
                valuation: render_reference_valuation(&result.valuation),
                premises: result.premises.iter().cloned().collect(),
                derived: if failed {
                    derived_fact_strings(&derived)
                } else {
                    Vec::new()
                },
                counterexample: if failed {
                    Some(CounterexampleTrace {
                        valuation: render_reference_valuation(&result.valuation),
                        premises: result.premises.iter().cloned().collect(),
                        missing_goals: result.missing_goals.iter().cloned().collect(),
                    })
                } else {
                    None
                },
            }
        })
        .collect::<Vec<_>>();

    let proved = obligations
        .iter()
        .filter(|item| item.result == "proved")
        .count();
    let total = obligations.len();
    Ok(ProofTrace {
        schema_version: PROOF_TRACE_SCHEMA_VERSION.to_string(),
        profile: "standard".to_string(),
        engine: "reference".to_string(),
        summary: ProofSummary {
            total,
            proved,
            failed: total.saturating_sub(proved),
        },
        claim_coverage: ClaimCoverage {
            total_claims: total,
            proved_claims: proved,
        },
        obligations,
    })
}

pub fn reference_solve_facts(program: &Program) -> Result<ReferenceDerivedFacts, Vec<Diagnostic>> {
    reference_solve_facts_with_extra(program, &[])
}

pub fn reference_prove_program_results(
    program: &Program,
) -> Result<Vec<ReferenceObligationResult>, Vec<Diagnostic>> {
    let derived = reference_solve_facts(program)?;
    let universe_map = build_universe_map(program).map_err(as_prove_error)?;
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
        let valuations =
            enumerate_valuations(obligation.params, &universe_map).map_err(as_prove_error)?;
        let mut failure = None;
        for valuation in valuations {
            let result = match &obligation.body {
                ReferenceObligationBody::Assert => {
                    if reference_eval_formula(obligation.goal, &valuation, &derived)
                        .map_err(as_prove_error)?
                    {
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
                            )
                            .map_err(as_prove_error)?,
                        })
                    }
                }
                ReferenceObligationBody::Refine { expr, result_var } => {
                    let expr_result = reference_eval_expr_with_context(
                        expr,
                        &valuation,
                        &derived,
                        &defn_map,
                        &relation_names,
                        &constructor_names,
                    )
                    .map_err(as_prove_error)?;
                    let mut goal_env = valuation.clone();
                    goal_env.insert(result_var.clone(), expr_result.value.clone());
                    match expr_result.value {
                        ReferenceValue::Bool(true) => {
                            if reference_eval_formula(obligation.goal, &goal_env, &derived)
                                .map_err(as_prove_error)?
                            {
                                None
                            } else {
                                Some(ReferenceObligationResult {
                                    id: obligation.id.clone(),
                                    kind: obligation.kind.clone(),
                                    result: "failed".to_string(),
                                    valuation: valuation.clone(),
                                    premises: expr_result.positive_facts,
                                    missing_goals: collect_missing_goals(
                                        obligation.goal,
                                        &goal_env,
                                        &derived,
                                    )
                                    .map_err(as_prove_error)?,
                                })
                            }
                        }
                        ReferenceValue::Bool(false) => None,
                        other => {
                            return Err(vec![Diagnostic::new(
                                "E-PROVE",
                                format!(
                                    "refine body did not evaluate to Bool: {}",
                                    reference_value_to_string(&other)
                                ),
                                None,
                            )]);
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

pub fn reference_prove_program(
    program: &Program,
) -> Result<Vec<ReferenceObligationResult>, Vec<Diagnostic>> {
    reference_prove_program_results(program)
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
        ReferenceValue::Function(fun) => {
            let mut rendered = String::from("(fun");
            for (inputs, output) in &fun.table {
                rendered.push(' ');
                rendered.push('(');
                for (idx, input) in inputs.iter().enumerate() {
                    if idx > 0 {
                        rendered.push(' ');
                    }
                    rendered.push_str(&reference_value_to_string(input));
                }
                rendered.push_str(" => ");
                rendered.push_str(&reference_value_to_string(output));
                rendered.push(')');
            }
            rendered.push(')');
            rendered
        }
    }
}

fn prepare_program(program: &Program) -> Result<Program, Vec<Diagnostic>> {
    let normalized = normalize_program_aliases(program)?;
    let mut errors = resolve_program(&normalized);
    if !errors.is_empty() {
        return Err(errors);
    }
    if let Err(mut e) = compute_strata(&normalized) {
        errors.append(&mut e);
        return Err(errors);
    }
    if let Err(mut e) = check_program(&normalized) {
        errors.append(&mut e);
        return Err(errors);
    }
    Ok(normalized)
}

fn reference_solve_facts_with_extra(
    program: &Program,
    extra: &[GroundFact],
) -> Result<ReferenceDerivedFacts, Vec<Diagnostic>> {
    let relation_schemas = program
        .relations
        .iter()
        .map(|rel| (rel.name.clone(), rel.arg_sorts.len()))
        .collect::<HashMap<_, _>>();
    let strata = compute_strata(program)?;

    let mut db = program
        .relations
        .iter()
        .map(|rel| (rel.name.clone(), BTreeSet::new()))
        .collect::<BTreeMap<_, _>>();

    for fact in &program.facts {
        let tuple = fact
            .terms
            .iter()
            .map(logic_term_to_reference_const)
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| {
                vec![Diagnostic::new(
                    "E-RESOLVE",
                    format!("fact contains variable: {}", fact.name),
                    Some(fact.span.clone()),
                )]
            })?;
        let Some(expected_arity) = relation_schemas.get(&fact.name) else {
            return Err(vec![Diagnostic::new(
                "E-RESOLVE",
                format!("undefined relation in fact: {}", fact.name),
                Some(fact.span.clone()),
            )]);
        };
        if *expected_arity != tuple.len() {
            return Err(vec![Diagnostic::new(
                "E-RESOLVE",
                format!(
                    "arity mismatch in fact {}: expected {}, got {}",
                    fact.name,
                    expected_arity,
                    tuple.len()
                ),
                Some(fact.span.clone()),
            )]);
        }
        db.entry(fact.name.clone()).or_default().insert(tuple);
    }

    for fact in extra {
        let tuple = fact
            .terms
            .iter()
            .map(concrete_to_reference)
            .collect::<Vec<_>>();
        db.entry(fact.pred.clone()).or_default().insert(tuple);
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
                let produced =
                    reference_apply_rule(&rule.head, &rule.body, &db).map_err(as_resolve_error)?;
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

fn reference_eval_expr_with_context(
    expr: &Expr,
    env: &ReferenceEnv,
    derived: &ReferenceDerivedFacts,
    defn_map: &HashMap<String, &Defn>,
    relation_names: &HashSet<String>,
    constructor_names: &HashSet<String>,
) -> Result<ReferenceExprResult, String> {
    let mut state = ReferenceEvalState {
        derived,
        relation_names,
        constructor_names,
        defn_map,
        cache: HashMap::new(),
        active_calls: HashSet::new(),
    };
    reference_eval_expr_inner(expr, env, &mut state)
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
        Expr::Call { name, args, .. } => {
            let mut arg_values = Vec::new();
            let mut positive_facts = BTreeSet::new();
            for arg in args {
                let result = reference_eval_expr_inner(arg, env, state)?;
                positive_facts.extend(result.positive_facts);
                arg_values.push(result.value);
            }

            if let Some(ReferenceValue::Function(fun)) = env.get(name) {
                let Some(value) = apply_function_value(fun, &arg_values) else {
                    return Err(format!("function value application failed: {name}"));
                };
                return Ok(ReferenceExprResult {
                    value,
                    positive_facts,
                });
            }

            if state.relation_names.contains(name) {
                let truth = state
                    .derived
                    .facts
                    .get(name)
                    .map(|tuples| tuples.contains(&arg_values))
                    .unwrap_or(false);
                if truth {
                    positive_facts.insert(reference_ground_fact_key(name, &arg_values));
                }
                return Ok(ReferenceExprResult {
                    value: ReferenceValue::Bool(truth),
                    positive_facts,
                });
            }

            if state.constructor_names.contains(name) {
                return Ok(ReferenceExprResult {
                    value: ReferenceValue::Adt {
                        ctor: name.clone(),
                        fields: arg_values,
                    },
                    positive_facts,
                });
            }

            let Some(defn) = state.defn_map.get(name) else {
                return Err(format!(
                    "unknown call target during expression evaluation: {name}"
                ));
            };
            if defn.params.len() != arg_values.len() {
                return Err(format!(
                    "arity mismatch during expression evaluation: {} expected {}, got {}",
                    name,
                    defn.params.len(),
                    arg_values.len()
                ));
            }

            let key = ReferenceCallKey {
                name: name.clone(),
                args: arg_values.clone(),
            };
            if let Some(cached) = state.cache.get(&key) {
                let mut merged = positive_facts;
                merged.extend(cached.positive_facts.clone());
                return Ok(ReferenceExprResult {
                    value: cached.value.clone(),
                    positive_facts: merged,
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

            let mut merged = positive_facts;
            merged.extend(result.positive_facts.clone());
            Ok(ReferenceExprResult {
                value: result.value,
                positive_facts: merged,
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
                    let branch = reference_eval_expr_inner(&arm.body, &branch_env, state)?;
                    let mut positive_facts = scrutinee_result.positive_facts.clone();
                    positive_facts.extend(branch.positive_facts.clone());
                    return Ok(ReferenceExprResult {
                        value: branch.value,
                        positive_facts,
                    });
                }
            }
            Err("match expression had no matching arm during reference evaluation".to_string())
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

fn build_universe_map(program: &Program) -> Result<BTreeMap<String, Vec<Value>>, String> {
    let mut map = BTreeMap::new();
    for universe in &program.universes {
        let values = universe
            .values
            .iter()
            .map(logic_term_to_concrete_const)
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| format!("universe value contains variable: {}", universe.ty_name))?;
        map.insert(universe.ty_name.clone(), values);
    }
    Ok(map)
}

fn enumerate_valuations(
    params: &[Param],
    universe_map: &BTreeMap<String, Vec<Value>>,
) -> Result<Vec<ReferenceEnv>, String> {
    let mut domains = Vec::new();
    let mut cache = HashMap::new();
    for param in params {
        let values = enumerate_reference_values_for_type(&param.ty, universe_map, &mut cache)?;
        if values.is_empty() {
            return Err(format!(
                "universe for type {} must not be empty",
                type_label(&param.ty)
            ));
        }
        domains.push((param.name.clone(), values));
    }
    let mut out = Vec::new();
    let mut current = ReferenceEnv::new();
    enumerate_cartesian(&domains, 0, &mut current, &mut out);
    Ok(out)
}

fn enumerate_reference_values_for_type(
    ty: &Type,
    universe: &BTreeMap<String, Vec<Value>>,
    cache: &mut HashMap<Type, Vec<ReferenceValue>>,
) -> Result<Vec<ReferenceValue>, String> {
    let normalized = match ty {
        Type::Refine { base, .. } => base.as_ref().clone(),
        _ => ty.clone(),
    };
    if let Some(values) = cache.get(&normalized) {
        return Ok(values.clone());
    }
    let values = match &normalized {
        Type::Bool => universe
            .get("Bool")
            .ok_or_else(|| "missing universe declaration for type: Bool".to_string())?
            .iter()
            .map(concrete_to_reference)
            .collect(),
        Type::Int => universe
            .get("Int")
            .ok_or_else(|| "missing universe declaration for type: Int".to_string())?
            .iter()
            .map(concrete_to_reference)
            .collect(),
        Type::Symbol => universe
            .get("Symbol")
            .ok_or_else(|| "missing universe declaration for type: Symbol".to_string())?
            .iter()
            .map(concrete_to_reference)
            .collect(),
        Type::Domain(name) | Type::Adt(name) => universe
            .get(name)
            .ok_or_else(|| format!("missing universe declaration for type: {name}"))?
            .iter()
            .map(concrete_to_reference)
            .collect(),
        Type::Fun(args, ret) => enumerate_function_values(args, ret, universe, cache)?,
        Type::Refine { .. } => unreachable!(),
    };
    cache.insert(normalized, values.clone());
    Ok(values)
}

fn enumerate_function_values(
    args: &[Type],
    ret: &Type,
    universe: &BTreeMap<String, Vec<Value>>,
    cache: &mut HashMap<Type, Vec<ReferenceValue>>,
) -> Result<Vec<ReferenceValue>, String> {
    let mut arg_domains = Vec::new();
    for arg in args {
        let values = enumerate_reference_values_for_type(arg, universe, cache)?;
        if values.is_empty() {
            return Ok(Vec::new());
        }
        arg_domains.push(values);
    }
    let input_tuples = enumerate_tuples(&arg_domains);
    let outputs = enumerate_reference_values_for_type(ret, universe, cache)?;
    if outputs.is_empty() {
        return Ok(Vec::new());
    }

    let output_count = outputs.len();
    let exponent =
        u32::try_from(input_tuples.len()).map_err(|_| "function model too large".to_string())?;
    let total = output_count
        .checked_pow(exponent)
        .ok_or_else(|| "function model cardinality overflow".to_string())?;
    if total > MAX_FUNCTION_MODEL_VALUES {
        return Err("function model exceeds phase2 limit".to_string());
    }

    if input_tuples.is_empty() {
        return Ok(outputs
            .into_iter()
            .map(|output| {
                ReferenceValue::Function(FunctionValue {
                    table: vec![(Vec::new(), output)],
                })
            })
            .collect());
    }

    let mut selectors = vec![0usize; input_tuples.len()];
    let mut out = Vec::with_capacity(total);
    loop {
        out.push(ReferenceValue::Function(FunctionValue {
            table: input_tuples
                .iter()
                .enumerate()
                .map(|(idx, input)| (input.clone(), outputs[selectors[idx]].clone()))
                .collect(),
        }));
        if !increment_digits(&mut selectors, output_count) {
            break;
        }
    }
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

fn enumerate_tuples<T: Clone>(domains: &[Vec<T>]) -> Vec<Vec<T>> {
    if domains.is_empty() {
        return vec![Vec::new()];
    }
    let mut out = vec![Vec::new()];
    for domain in domains {
        let mut next = Vec::new();
        for prefix in &out {
            for item in domain {
                let mut tuple = prefix.clone();
                tuple.push(item.clone());
                next.push(tuple);
            }
        }
        out = next;
    }
    out
}

fn increment_digits(digits: &mut [usize], base: usize) -> bool {
    for digit in digits {
        *digit += 1;
        if *digit < base {
            return true;
        }
        *digit = 0;
    }
    false
}

fn reference_apply_rule(
    head: &Atom,
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
        } => {
            matches!(value, ReferenceValue::Symbol(actual) if actual == expected)
        }
        Pattern::Int {
            value: expected, ..
        } => {
            matches!(value, ReferenceValue::Int(actual) if actual == expected)
        }
        Pattern::Bool {
            value: expected, ..
        } => {
            matches!(value, ReferenceValue::Bool(actual) if actual == expected)
        }
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

fn apply_function_value(fun: &FunctionValue, args: &[ReferenceValue]) -> Option<ReferenceValue> {
    fun.table
        .iter()
        .find(|(inputs, _)| inputs == args)
        .map(|(_, output)| output.clone())
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

fn logic_term_to_concrete_const(term: &LogicTerm) -> Option<Value> {
    match term {
        LogicTerm::Var(_) => None,
        LogicTerm::Symbol(symbol) => Some(Value::Symbol(symbol.clone())),
        LogicTerm::Int(value) => Some(Value::Int(*value)),
        LogicTerm::Bool(value) => Some(Value::Bool(*value)),
        LogicTerm::Ctor { name, args } => Some(Value::Adt {
            ctor: name.clone(),
            fields: args
                .iter()
                .map(logic_term_to_concrete_const)
                .collect::<Option<Vec<_>>>()?,
        }),
    }
}

fn concrete_to_reference(value: &Value) -> ReferenceValue {
    match value {
        Value::Symbol(symbol) => ReferenceValue::Symbol(symbol.clone()),
        Value::Int(value) => ReferenceValue::Int(*value),
        Value::Bool(value) => ReferenceValue::Bool(*value),
        Value::Adt { ctor, fields } => ReferenceValue::Adt {
            ctor: ctor.clone(),
            fields: fields.iter().map(concrete_to_reference).collect(),
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

fn render_reference_valuation(valuation: &ReferenceEnv) -> Vec<NameValue> {
    valuation
        .iter()
        .map(|(name, value)| NameValue {
            name: name.clone(),
            value: reference_value_to_string(value),
        })
        .collect()
}

fn derived_fact_strings(derived: &ReferenceDerivedFacts) -> Vec<String> {
    let mut out = Vec::new();
    for (pred, tuples) in &derived.facts {
        for tuple in tuples {
            out.push(reference_ground_fact_key(pred, tuple));
        }
    }
    out.sort();
    out
}

fn reference_ground_fact_key(pred: &str, tuple: &[ReferenceValue]) -> String {
    let args = tuple
        .iter()
        .map(reference_value_to_string)
        .collect::<Vec<_>>()
        .join(",");
    format!("{pred}({args})")
}

fn type_label(ty: &Type) -> String {
    match ty {
        Type::Bool => "Bool".to_string(),
        Type::Int => "Int".to_string(),
        Type::Symbol => "Symbol".to_string(),
        Type::Domain(name) | Type::Adt(name) => name.clone(),
        Type::Fun(args, ret) => format!(
            "(-> ({}) {})",
            args.iter().map(type_label).collect::<Vec<_>>().join(" "),
            type_label(ret)
        ),
        Type::Refine { base, .. } => type_label(base),
    }
}

fn as_prove_error(message: String) -> Vec<Diagnostic> {
    vec![Diagnostic::new("E-PROVE", message, None)]
}

fn as_resolve_error(message: String) -> Vec<Diagnostic> {
    vec![Diagnostic::new("E-RESOLVE", message, None)]
}
