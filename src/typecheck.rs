use std::collections::{HashMap, HashSet};

use crate::ast::{Defn, Expr, Program};
use crate::diagnostics::Diagnostic;
use crate::logic_engine::{DerivedFacts, GroundFact, KnowledgeBase, Value, solve_facts};
use crate::name_resolve::resolve_program;
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
struct TypeContext {
    relation_sigs: HashMap<String, Vec<Type>>,
    function_sigs: HashMap<String, FunctionSig>,
    kb_template: KnowledgeBase,
}

pub fn check_program(program: &Program) -> Result<TypeReport, Vec<Diagnostic>> {
    let mut errors = resolve_program(program);
    if !errors.is_empty() {
        return Err(errors);
    }

    if let Err(mut stratify_errors) = compute_strata(program) {
        errors.append(&mut stratify_errors);
        return Err(errors);
    }

    let kb = KnowledgeBase::from_program(program)?;
    let _ = solve_facts(&kb)?;

    let relation_sigs = build_relation_sigs(program);
    let function_sigs = build_function_sigs(program);
    let ctx = TypeContext {
        relation_sigs,
        function_sigs,
        kb_template: kb,
    };

    for defn in &program.defns {
        if let Err(mut e) = check_defn(defn, &ctx) {
            errors.append(&mut e);
        }
    }

    if errors.is_empty() {
        Ok(TypeReport {
            functions_checked: program.defns.len(),
            errors: 0,
        })
    } else {
        Err(errors)
    }
}

fn build_relation_sigs(program: &Program) -> HashMap<String, Vec<Type>> {
    let mut map = HashMap::new();
    for rel in &program.relations {
        map.insert(
            rel.name.clone(),
            rel.arg_sorts
                .iter()
                .map(|s| sort_to_type(s))
                .collect::<Vec<_>>(),
        );
    }
    map
}

fn build_function_sigs(program: &Program) -> HashMap<String, FunctionSig> {
    let mut map = HashMap::new();
    for f in &program.defns {
        map.insert(
            f.name.clone(),
            FunctionSig {
                params: f.params.iter().map(|p| p.ty.clone()).collect(),
                ret: f.ret_type.clone(),
                param_names: f.params.iter().map(|p| p.name.clone()).collect(),
            },
        );
    }
    map
}

fn check_defn(defn: &Defn, ctx: &TypeContext) -> Result<(), Vec<Diagnostic>> {
    let mut env = HashMap::new();
    for p in &defn.params {
        env.insert(p.name.clone(), p.ty.clone());
    }

    let actual = infer_expr(&defn.body, &env, ctx)?;
    match is_subtype(&actual, &defn.ret_type, ctx) {
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
                    if let Some(term) = expr_to_logic_term(arg) {
                        substitution.insert(sig.param_names[idx].clone(), term);
                    }
                }

                Ok(substitute_type(&sig.ret, &substitution))
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
                    let Some(term) = expr_to_logic_term(arg) else {
                        return Err(vec![Diagnostic::new(
                            "E-TYPE",
                            "relation argument must be variable or literal",
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
                    format!("unknown function or relation: {name}"),
                    Some(span.clone()),
                )])
            }
        }
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
        (Type::Bool, Type::Bool)
        | (Type::Int, Type::Int)
        | (Type::Symbol, Type::Symbol)
        | (Type::Symbol, Type::Domain(_))
        | (Type::Domain(_), Type::Symbol) => Ok(()),
        (Type::Domain(a), Type::Domain(b)) if a == b => Ok(()),
        (Type::Fun(a_args, a_ret), Type::Fun(b_args, b_ret)) => {
            if a_args.len() != b_args.len() {
                return Err(Diagnostic::new("E-TYPE", "function arity mismatch", None));
            }
            for (a, b) in a_args.iter().zip(b_args.iter()) {
                if a != b {
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
        let v = match term {
            LogicTerm::Var(name) => vars.get(name)?.clone(),
            LogicTerm::Symbol(s) => Value::Symbol(s.clone()),
            LogicTerm::Int(i) => Value::Int(*i),
            LogicTerm::Bool(b) => Value::Bool(*b),
        };
        tuple.push(v);
    }
    Some((atom.pred.clone(), tuple))
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
                if let LogicTerm::Var(v) = t {
                    out.insert(v.clone());
                }
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

fn rename_formula_var(formula: &Formula, from: &str, to: &str) -> Formula {
    match formula {
        Formula::True => Formula::True,
        Formula::Atom(atom) => Formula::Atom(Atom {
            pred: atom.pred.clone(),
            terms: atom
                .terms
                .iter()
                .map(|t| match t {
                    LogicTerm::Var(v) if v == from => LogicTerm::Var(to.to_string()),
                    other => other.clone(),
                })
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

fn substitute_type(ty: &Type, subst: &HashMap<String, LogicTerm>) -> Type {
    match ty {
        Type::Bool => Type::Bool,
        Type::Int => Type::Int,
        Type::Symbol => Type::Symbol,
        Type::Domain(s) => Type::Domain(s.clone()),
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
                .map(|t| match t {
                    LogicTerm::Var(v) => subst
                        .get(v)
                        .cloned()
                        .unwrap_or_else(|| LogicTerm::Var(v.clone())),
                    other => other.clone(),
                })
                .collect(),
        }),
        Formula::And(items) => {
            Formula::And(items.iter().map(|f| substitute_formula(f, subst)).collect())
        }
        Formula::Not(inner) => Formula::Not(Box::new(substitute_formula(inner, subst))),
    }
}

fn expr_to_logic_term(expr: &Expr) -> Option<LogicTerm> {
    match expr {
        Expr::Var { name, .. } => Some(LogicTerm::Var(name.clone())),
        Expr::Symbol { value, .. } => Some(LogicTerm::Symbol(value.clone())),
        Expr::Int { value, .. } => Some(LogicTerm::Int(*value)),
        Expr::Bool { value, .. } => Some(LogicTerm::Bool(*value)),
        Expr::Call { .. } | Expr::Let { .. } | Expr::If { .. } => None,
    }
}

fn sort_to_type(sort: &str) -> Type {
    match sort {
        "Bool" => Type::Bool,
        "Int" => Type::Int,
        "Symbol" => Type::Symbol,
        other => Type::Domain(other.to_string()),
    }
}
