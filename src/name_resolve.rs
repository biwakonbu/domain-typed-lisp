use std::collections::{HashMap, HashSet};

use crate::ast::{Defn, Expr, Program, Rule};
use crate::diagnostics::Diagnostic;
use crate::types::{Formula, LogicTerm, Type};

pub fn resolve_program(program: &Program) -> Vec<Diagnostic> {
    let mut errors = Vec::new();

    let mut sort_set = HashSet::new();
    for s in &program.sorts {
        if !sort_set.insert(s.name.clone()) {
            errors.push(Diagnostic::new(
                "E-RESOLVE",
                format!("duplicate sort: {}", s.name),
                Some(s.span.clone()),
            ));
        }
    }

    let mut relation_arity: HashMap<String, usize> = HashMap::new();
    let mut relation_sorts: HashMap<String, Vec<String>> = HashMap::new();
    for r in &program.relations {
        if relation_arity.contains_key(&r.name) {
            errors.push(Diagnostic::new(
                "E-RESOLVE",
                format!("duplicate relation: {}", r.name),
                Some(r.span.clone()),
            ));
            continue;
        }
        for sort in &r.arg_sorts {
            if !is_known_sort(sort, &sort_set) {
                errors.push(Diagnostic::new(
                    "E-RESOLVE",
                    format!("unknown sort in relation {}: {sort}", r.name),
                    Some(r.span.clone()),
                ));
            }
        }
        relation_arity.insert(r.name.clone(), r.arg_sorts.len());
        relation_sorts.insert(r.name.clone(), r.arg_sorts.clone());
    }

    let mut function_sigs: HashMap<String, (Vec<Type>, Type)> = HashMap::new();
    for f in &program.defns {
        if function_sigs.contains_key(&f.name) {
            errors.push(Diagnostic::new(
                "E-RESOLVE",
                format!("duplicate function: {}", f.name),
                Some(f.span.clone()),
            ));
            continue;
        }
        let mut params = Vec::new();
        for p in &f.params {
            params.push(p.ty.clone());
        }
        function_sigs.insert(f.name.clone(), (params, f.ret_type.clone()));
    }

    for fact in &program.facts {
        let Some(arity) = relation_arity.get(&fact.name) else {
            errors.push(Diagnostic::new(
                "E-RESOLVE",
                format!("undefined relation in fact: {}", fact.name),
                Some(fact.span.clone()),
            ));
            continue;
        };
        if *arity != fact.terms.len() {
            errors.push(Diagnostic::new(
                "E-RESOLVE",
                format!(
                    "arity mismatch in fact {}: expected {}, got {}",
                    fact.name,
                    arity,
                    fact.terms.len()
                ),
                Some(fact.span.clone()),
            ));
        }
        for t in &fact.terms {
            if matches!(t, LogicTerm::Var(_)) {
                errors.push(Diagnostic::new(
                    "E-RESOLVE",
                    "fact contains variable",
                    Some(fact.span.clone()),
                ));
            }
        }
    }

    for rule in &program.rules {
        validate_rule(rule, &relation_arity, &mut errors);
    }

    for defn in &program.defns {
        validate_defn(
            defn,
            &sort_set,
            &relation_sorts,
            &function_sigs,
            &mut errors,
        );
    }

    errors
}

fn validate_rule(
    rule: &Rule,
    relation_arity: &HashMap<String, usize>,
    errors: &mut Vec<Diagnostic>,
) {
    let Some(head_arity) = relation_arity.get(&rule.head.pred) else {
        errors.push(Diagnostic::new(
            "E-RESOLVE",
            format!("undefined relation in rule head: {}", rule.head.pred),
            Some(rule.span.clone()),
        ));
        return;
    };

    if *head_arity != rule.head.terms.len() {
        errors.push(Diagnostic::new(
            "E-RESOLVE",
            format!(
                "arity mismatch in rule head {}: expected {}, got {}",
                rule.head.pred,
                head_arity,
                rule.head.terms.len()
            ),
            Some(rule.span.clone()),
        ));
    }

    let mut positives = Vec::new();
    let mut negatives = Vec::new();
    flatten_body(&rule.body, false, &mut positives, &mut negatives);

    for atom in positives.iter().chain(negatives.iter()) {
        let Some(arity) = relation_arity.get(&atom.pred) else {
            errors.push(Diagnostic::new(
                "E-RESOLVE",
                format!("undefined relation in rule body: {}", atom.pred),
                Some(rule.span.clone()),
            ));
            continue;
        };
        if *arity != atom.terms.len() {
            errors.push(Diagnostic::new(
                "E-RESOLVE",
                format!(
                    "arity mismatch in rule body {}: expected {}, got {}",
                    atom.pred,
                    arity,
                    atom.terms.len()
                ),
                Some(rule.span.clone()),
            ));
        }
    }

    let mut positive_vars = HashSet::new();
    for atom in &positives {
        for term in &atom.terms {
            if let LogicTerm::Var(v) = term {
                positive_vars.insert(v.clone());
            }
        }
    }

    for term in &rule.head.terms {
        if let LogicTerm::Var(v) = term
            && !positive_vars.contains(v)
        {
            errors.push(Diagnostic::new(
                "E-RESOLVE",
                format!(
                    "unsafe rule: head variable ?{} is not bound in positive body",
                    v
                ),
                Some(rule.span.clone()),
            ));
        }
    }

    for atom in &negatives {
        for term in &atom.terms {
            if let LogicTerm::Var(v) = term
                && !positive_vars.contains(v)
            {
                errors.push(Diagnostic::new(
                    "E-RESOLVE",
                    format!("unsafe rule: negated variable ?{} is not bound", v),
                    Some(rule.span.clone()),
                ));
            }
        }
    }
}

fn validate_defn(
    defn: &Defn,
    sort_set: &HashSet<String>,
    relation_sorts: &HashMap<String, Vec<String>>,
    function_sigs: &HashMap<String, (Vec<Type>, Type)>,
    errors: &mut Vec<Diagnostic>,
) {
    let mut param_names = HashSet::new();
    for p in &defn.params {
        if !param_names.insert(p.name.clone()) {
            errors.push(Diagnostic::new(
                "E-RESOLVE",
                format!("duplicate parameter name: {}", p.name),
                Some(p.span.clone()),
            ));
        }
        if let Some(msg) = validate_type(&p.ty, sort_set, relation_sorts, &param_names) {
            errors.push(Diagnostic::new("E-RESOLVE", msg, Some(p.span.clone())));
        }
    }

    if let Some(msg) = validate_type(&defn.ret_type, sort_set, relation_sorts, &param_names) {
        errors.push(Diagnostic::new("E-RESOLVE", msg, Some(defn.span.clone())));
    }

    validate_expr_names(
        &defn.body,
        &param_names,
        function_sigs,
        relation_sorts,
        errors,
    );
}

fn validate_type(
    ty: &Type,
    sort_set: &HashSet<String>,
    relation_sorts: &HashMap<String, Vec<String>>,
    scope: &HashSet<String>,
) -> Option<String> {
    match ty {
        Type::Bool | Type::Int | Type::Symbol => None,
        Type::Domain(s) => {
            if is_known_sort(s, sort_set) {
                None
            } else {
                Some(format!("unknown type: {s}"))
            }
        }
        Type::Fun(args, ret) => {
            for a in args {
                if let Some(msg) = validate_type(a, sort_set, relation_sorts, scope) {
                    return Some(msg);
                }
            }
            validate_type(ret, sort_set, relation_sorts, scope)
        }
        Type::Refine { var, base, formula } => {
            if let Some(msg) = validate_type(base, sort_set, relation_sorts, scope) {
                return Some(msg);
            }
            let mut next_scope = scope.clone();
            next_scope.insert(var.clone());
            validate_formula(formula, relation_sorts, &next_scope)
        }
    }
}

fn validate_formula(
    formula: &Formula,
    relation_sorts: &HashMap<String, Vec<String>>,
    scope: &HashSet<String>,
) -> Option<String> {
    match formula {
        Formula::True => None,
        Formula::Atom(atom) => {
            let Some(sorts) = relation_sorts.get(&atom.pred) else {
                return Some(format!("unknown predicate in refinement: {}", atom.pred));
            };
            if sorts.len() != atom.terms.len() {
                return Some(format!(
                    "arity mismatch in refinement predicate {}: expected {}, got {}",
                    atom.pred,
                    sorts.len(),
                    atom.terms.len()
                ));
            }
            for t in &atom.terms {
                if let LogicTerm::Var(v) = t
                    && !scope.contains(v)
                {
                    return Some(format!("unknown variable in refinement: {v}"));
                }
            }
            None
        }
        Formula::And(items) => {
            for item in items {
                if let Some(msg) = validate_formula(item, relation_sorts, scope) {
                    return Some(msg);
                }
            }
            None
        }
        Formula::Not(inner) => validate_formula(inner, relation_sorts, scope),
    }
}

fn validate_expr_names(
    expr: &Expr,
    scope: &HashSet<String>,
    function_sigs: &HashMap<String, (Vec<Type>, Type)>,
    relation_sorts: &HashMap<String, Vec<String>>,
    errors: &mut Vec<Diagnostic>,
) {
    match expr {
        Expr::Var { name, span } => {
            if !scope.contains(name) {
                errors.push(Diagnostic::new(
                    "E-RESOLVE",
                    format!("unknown variable: {name}"),
                    Some(span.clone()),
                ));
            }
        }
        Expr::Symbol { .. } | Expr::Int { .. } | Expr::Bool { .. } => {}
        Expr::Call { name, args, span } => {
            if !function_sigs.contains_key(name) && !relation_sorts.contains_key(name) {
                errors.push(Diagnostic::new(
                    "E-RESOLVE",
                    format!("unknown function or relation: {name}"),
                    Some(span.clone()),
                ));
            }
            for arg in args {
                validate_expr_names(arg, scope, function_sigs, relation_sorts, errors);
            }
        }
        Expr::Let {
            bindings,
            body,
            span: _,
        } => {
            let mut local_scope = scope.clone();
            for (name, bexpr, bspan) in bindings {
                validate_expr_names(bexpr, &local_scope, function_sigs, relation_sorts, errors);
                if local_scope.contains(name) {
                    errors.push(Diagnostic::new(
                        "E-RESOLVE",
                        format!("duplicate or shadowed let binding: {name}"),
                        Some(bspan.clone()),
                    ));
                }
                local_scope.insert(name.clone());
            }
            validate_expr_names(body, &local_scope, function_sigs, relation_sorts, errors);
        }
        Expr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            validate_expr_names(cond, scope, function_sigs, relation_sorts, errors);
            validate_expr_names(then_branch, scope, function_sigs, relation_sorts, errors);
            validate_expr_names(else_branch, scope, function_sigs, relation_sorts, errors);
        }
    }
}

fn flatten_body<'a>(
    formula: &'a Formula,
    negated: bool,
    positives: &mut Vec<&'a crate::types::Atom>,
    negatives: &mut Vec<&'a crate::types::Atom>,
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
                flatten_body(item, negated, positives, negatives);
            }
        }
        Formula::Not(inner) => flatten_body(inner, !negated, positives, negatives),
    }
}

fn is_known_sort(sort: &str, sorts: &HashSet<String>) -> bool {
    matches!(sort, "Bool" | "Int" | "Symbol") || sorts.contains(sort)
}
