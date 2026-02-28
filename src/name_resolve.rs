use std::collections::{HashMap, HashSet};

use crate::ast::{Defn, Expr, Pattern, Program, Rule, UniverseDecl};
use crate::diagnostics::Diagnostic;
use crate::types::{Formula, LogicTerm, Type};

#[derive(Debug, Clone)]
struct ConstructorSig {
    data_name: String,
    arity: usize,
}

pub fn normalize_program_aliases(program: &Program) -> Result<Program, Vec<Diagnostic>> {
    if program.aliases.is_empty() {
        return Ok(program.clone());
    }

    let constructor_names = program
        .data_decls
        .iter()
        .flat_map(|d| d.constructors.iter().map(|ctor| ctor.name.clone()))
        .collect::<HashSet<_>>();
    let relation_names = program
        .relations
        .iter()
        .map(|r| r.name.clone())
        .collect::<HashSet<_>>();
    let function_names = program
        .defns
        .iter()
        .map(|f| f.name.clone())
        .collect::<HashSet<_>>();

    let mut errors = Vec::new();
    let mut raw_alias_map: HashMap<String, (String, crate::diagnostics::Span)> = HashMap::new();
    for alias in &program.aliases {
        if raw_alias_map
            .insert(
                alias.alias.clone(),
                (alias.canonical.clone(), alias.span.clone()),
            )
            .is_some()
        {
            errors.push(Diagnostic::new(
                "E-DATA",
                format!("duplicate alias declaration: {}", alias.alias),
                Some(alias.span.clone()),
            ));
            continue;
        }

        if constructor_names.contains(&alias.alias) {
            errors.push(Diagnostic::new(
                "E-DATA",
                format!("alias conflicts with constructor: {}", alias.alias),
                Some(alias.span.clone()),
            ));
        }
        if relation_names.contains(&alias.alias) {
            errors.push(Diagnostic::new(
                "E-RESOLVE",
                format!("alias conflicts with relation: {}", alias.alias),
                Some(alias.span.clone()),
            ));
        }
        if function_names.contains(&alias.alias) {
            errors.push(Diagnostic::new(
                "E-RESOLVE",
                format!("alias conflicts with function: {}", alias.alias),
                Some(alias.span.clone()),
            ));
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }

    let mut resolved_alias_map: HashMap<String, String> = HashMap::new();
    for alias in raw_alias_map.keys() {
        let mut stack = Vec::new();
        resolve_alias_target(
            alias,
            &raw_alias_map,
            &constructor_names,
            &mut resolved_alias_map,
            &mut stack,
            &mut errors,
        );
    }
    if !errors.is_empty() {
        return Err(errors);
    }

    let mut normalized = program.clone();
    for fact in &mut normalized.facts {
        for term in &mut fact.terms {
            normalize_logic_term_alias(term, &resolved_alias_map);
        }
    }
    for rule in &mut normalized.rules {
        for term in &mut rule.head.terms {
            normalize_logic_term_alias(term, &resolved_alias_map);
        }
        normalize_formula_alias(&mut rule.body, &resolved_alias_map);
    }
    for assertion in &mut normalized.asserts {
        for param in &mut assertion.params {
            normalize_type_alias(&mut param.ty, &resolved_alias_map);
        }
        normalize_formula_alias(&mut assertion.formula, &resolved_alias_map);
    }
    for universe in &mut normalized.universes {
        for term in &mut universe.values {
            normalize_logic_term_alias(term, &resolved_alias_map);
        }
    }
    for defn in &mut normalized.defns {
        for param in &mut defn.params {
            normalize_type_alias(&mut param.ty, &resolved_alias_map);
        }
        normalize_type_alias(&mut defn.ret_type, &resolved_alias_map);
        normalize_expr_alias(&mut defn.body, &resolved_alias_map);
    }

    Ok(normalized)
}

fn resolve_alias_target(
    alias: &str,
    raw_alias_map: &HashMap<String, (String, crate::diagnostics::Span)>,
    constructor_names: &HashSet<String>,
    resolved_alias_map: &mut HashMap<String, String>,
    stack: &mut Vec<String>,
    errors: &mut Vec<Diagnostic>,
) -> Option<String> {
    if let Some(target) = resolved_alias_map.get(alias) {
        return Some(target.clone());
    }

    if let Some(pos) = stack.iter().position(|item| item == alias) {
        let mut cycle = stack[pos..].to_vec();
        cycle.push(alias.to_string());
        let (target, span) = raw_alias_map
            .get(alias)
            .expect("alias must exist while resolving cycle");
        let _ = target;
        errors.push(Diagnostic::new(
            "E-RESOLVE",
            format!("alias cycle detected: {}", cycle.join(" -> ")),
            Some(span.clone()),
        ));
        return None;
    }

    let (direct_target, span) = raw_alias_map.get(alias)?;
    stack.push(alias.to_string());

    let resolved = if constructor_names.contains(direct_target) {
        Some(direct_target.clone())
    } else if raw_alias_map.contains_key(direct_target) {
        resolve_alias_target(
            direct_target,
            raw_alias_map,
            constructor_names,
            resolved_alias_map,
            stack,
            errors,
        )
    } else {
        errors.push(Diagnostic::new(
            "E-RESOLVE",
            format!(
                "alias canonical constructor is undefined: {} -> {}",
                alias, direct_target
            ),
            Some(span.clone()),
        ));
        None
    };

    stack.pop();
    if let Some(target) = resolved.clone() {
        resolved_alias_map.insert(alias.to_string(), target);
    }
    resolved
}

fn normalize_type_alias(ty: &mut Type, alias_map: &HashMap<String, String>) {
    match ty {
        Type::Fun(args, ret) => {
            for arg in args {
                normalize_type_alias(arg, alias_map);
            }
            normalize_type_alias(ret, alias_map);
        }
        Type::Refine { base, formula, .. } => {
            normalize_type_alias(base, alias_map);
            normalize_formula_alias(formula, alias_map);
        }
        Type::Bool | Type::Int | Type::Symbol | Type::Domain(_) | Type::Adt(_) => {}
    }
}

fn normalize_formula_alias(formula: &mut Formula, alias_map: &HashMap<String, String>) {
    match formula {
        Formula::True => {}
        Formula::Atom(atom) => {
            for term in &mut atom.terms {
                normalize_logic_term_alias(term, alias_map);
            }
        }
        Formula::And(items) => {
            for item in items {
                normalize_formula_alias(item, alias_map);
            }
        }
        Formula::Not(inner) => normalize_formula_alias(inner, alias_map),
    }
}

fn normalize_logic_term_alias(term: &mut LogicTerm, alias_map: &HashMap<String, String>) {
    match term {
        LogicTerm::Ctor { name, args } => {
            if let Some(canonical) = alias_map.get(name) {
                *name = canonical.clone();
            }
            for arg in args {
                normalize_logic_term_alias(arg, alias_map);
            }
        }
        LogicTerm::Var(_) | LogicTerm::Symbol(_) | LogicTerm::Int(_) | LogicTerm::Bool(_) => {}
    }
}

fn normalize_expr_alias(expr: &mut Expr, alias_map: &HashMap<String, String>) {
    match expr {
        Expr::Var { .. } | Expr::Symbol { .. } | Expr::Int { .. } | Expr::Bool { .. } => {}
        Expr::Call { name, args, .. } => {
            if let Some(canonical) = alias_map.get(name) {
                *name = canonical.clone();
            }
            for arg in args {
                normalize_expr_alias(arg, alias_map);
            }
        }
        Expr::Let { bindings, body, .. } => {
            for (_, bexpr, _) in bindings {
                normalize_expr_alias(bexpr, alias_map);
            }
            normalize_expr_alias(body, alias_map);
        }
        Expr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            normalize_expr_alias(cond, alias_map);
            normalize_expr_alias(then_branch, alias_map);
            normalize_expr_alias(else_branch, alias_map);
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            normalize_expr_alias(scrutinee, alias_map);
            for arm in arms {
                normalize_pattern_alias(&mut arm.pattern, alias_map);
                normalize_expr_alias(&mut arm.body, alias_map);
            }
        }
    }
}

fn normalize_pattern_alias(pattern: &mut Pattern, alias_map: &HashMap<String, String>) {
    match pattern {
        Pattern::Ctor { name, args, .. } => {
            if let Some(canonical) = alias_map.get(name) {
                *name = canonical.clone();
            }
            for arg in args {
                normalize_pattern_alias(arg, alias_map);
            }
        }
        Pattern::Wildcard { .. }
        | Pattern::Var { .. }
        | Pattern::Symbol { .. }
        | Pattern::Int { .. }
        | Pattern::Bool { .. } => {}
    }
}

pub fn resolve_program(program: &Program) -> Vec<Diagnostic> {
    let normalized = match normalize_program_aliases(program) {
        Ok(program) => program,
        Err(errors) => return errors,
    };
    resolve_program_internal(&normalized)
}

fn resolve_program_internal(program: &Program) -> Vec<Diagnostic> {
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

    let mut data_map = HashMap::new();
    let mut constructor_map: HashMap<String, ConstructorSig> = HashMap::new();
    for d in &program.data_decls {
        if data_map.insert(d.name.clone(), d).is_some() {
            errors.push(Diagnostic::new(
                "E-DATA",
                format!("duplicate data declaration: {}", d.name),
                Some(d.span.clone()),
            ));
        }
        if sort_set.contains(&d.name) {
            errors.push(Diagnostic::new(
                "E-DATA",
                format!("data name conflicts with sort: {}", d.name),
                Some(d.span.clone()),
            ));
        }
        for ctor in &d.constructors {
            if constructor_map
                .insert(
                    ctor.name.clone(),
                    ConstructorSig {
                        data_name: d.name.clone(),
                        arity: ctor.fields.len(),
                    },
                )
                .is_some()
            {
                errors.push(Diagnostic::new(
                    "E-DATA",
                    format!("duplicate constructor: {}", ctor.name),
                    Some(ctor.span.clone()),
                ));
            }
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
            if !is_known_type_name(sort, &sort_set, &data_map) {
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
            if logic_term_contains_var(t) {
                errors.push(Diagnostic::new(
                    "E-RESOLVE",
                    "fact contains variable",
                    Some(fact.span.clone()),
                ));
            }
            validate_constructor_term(t, &constructor_map, &mut errors, &fact.span);
        }
    }

    for rule in &program.rules {
        validate_rule(rule, &relation_arity, &constructor_map, &mut errors);
    }

    let mut assert_names = HashSet::new();
    for assertion in &program.asserts {
        if !assert_names.insert(assertion.name.clone()) {
            errors.push(Diagnostic::new(
                "E-RESOLVE",
                format!("duplicate assert: {}", assertion.name),
                Some(assertion.span.clone()),
            ));
        }

        let mut param_names = HashSet::new();
        for p in &assertion.params {
            if !param_names.insert(p.name.clone()) {
                errors.push(Diagnostic::new(
                    "E-RESOLVE",
                    format!("duplicate assert parameter name: {}", p.name),
                    Some(p.span.clone()),
                ));
            }
            if let Some(msg) = validate_type(
                &p.ty,
                &sort_set,
                &data_map,
                &relation_sorts,
                &param_names,
                &constructor_map,
            ) {
                errors.push(Diagnostic::new("E-RESOLVE", msg, Some(p.span.clone())));
            }
        }
        if let Some(msg) = validate_formula(
            &assertion.formula,
            &relation_sorts,
            &param_names,
            &constructor_map,
        ) {
            errors.push(Diagnostic::new(
                "E-RESOLVE",
                msg,
                Some(assertion.span.clone()),
            ));
        }
    }

    validate_universes(
        &program.universes,
        &sort_set,
        &data_map,
        &constructor_map,
        &mut errors,
    );

    for defn in &program.defns {
        validate_defn(
            defn,
            &sort_set,
            &data_map,
            &relation_sorts,
            &function_sigs,
            &constructor_map,
            &mut errors,
        );
    }

    errors
}

fn validate_universes(
    universes: &[UniverseDecl],
    sort_set: &HashSet<String>,
    data_map: &HashMap<String, &crate::ast::DataDecl>,
    constructor_map: &HashMap<String, ConstructorSig>,
    errors: &mut Vec<Diagnostic>,
) {
    let mut seen = HashSet::new();
    for u in universes {
        if !is_known_type_name(&u.ty_name, sort_set, data_map) {
            errors.push(Diagnostic::new(
                "E-RESOLVE",
                format!("unknown universe type: {}", u.ty_name),
                Some(u.span.clone()),
            ));
            continue;
        }
        if !seen.insert(u.ty_name.clone()) {
            errors.push(Diagnostic::new(
                "E-RESOLVE",
                format!("duplicate universe declaration: {}", u.ty_name),
                Some(u.span.clone()),
            ));
        }

        for term in &u.values {
            if logic_term_contains_var(term) {
                errors.push(Diagnostic::new(
                    "E-RESOLVE",
                    "universe contains variable",
                    Some(u.span.clone()),
                ));
                continue;
            }

            if data_map.contains_key(&u.ty_name) {
                if let Some(msg) = validate_adt_value(term, &u.ty_name, constructor_map) {
                    errors.push(Diagnostic::new("E-DATA", msg, Some(u.span.clone())));
                }
            } else {
                match u.ty_name.as_str() {
                    "Bool" => {
                        if !matches!(term, LogicTerm::Bool(_)) {
                            errors.push(Diagnostic::new(
                                "E-RESOLVE",
                                "Bool universe must contain Bool values",
                                Some(u.span.clone()),
                            ));
                        }
                    }
                    "Int" => {
                        if !matches!(term, LogicTerm::Int(_)) {
                            errors.push(Diagnostic::new(
                                "E-RESOLVE",
                                "Int universe must contain Int values",
                                Some(u.span.clone()),
                            ));
                        }
                    }
                    _ => {
                        if !matches!(term, LogicTerm::Symbol(_)) {
                            errors.push(Diagnostic::new(
                                "E-RESOLVE",
                                "sort universe must contain symbolic constants",
                                Some(u.span.clone()),
                            ));
                        }
                    }
                }
            }
        }
    }
}

fn validate_adt_value(
    term: &LogicTerm,
    expected_data: &str,
    constructor_map: &HashMap<String, ConstructorSig>,
) -> Option<String> {
    let LogicTerm::Ctor { name, args } = term else {
        return Some(format!(
            "ADT universe value must be constructor application for {expected_data}"
        ));
    };
    let Some(sig) = constructor_map.get(name) else {
        return Some(format!("unknown constructor in universe: {name}"));
    };
    if sig.data_name != expected_data {
        return Some(format!(
            "constructor {name} belongs to {}, expected {}",
            sig.data_name, expected_data
        ));
    }
    if sig.arity != args.len() {
        return Some(format!(
            "constructor {name} arity mismatch: expected {}, got {}",
            sig.arity,
            args.len()
        ));
    }
    for arg in args {
        if logic_term_contains_var(arg) {
            return Some("ADT universe value cannot contain variables".to_string());
        }
    }
    None
}

fn validate_rule(
    rule: &Rule,
    relation_arity: &HashMap<String, usize>,
    constructor_map: &HashMap<String, ConstructorSig>,
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

    for term in &rule.head.terms {
        validate_constructor_term(term, constructor_map, errors, &rule.span);
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

        for term in &atom.terms {
            validate_constructor_term(term, constructor_map, errors, &rule.span);
        }
    }

    let mut positive_vars = HashSet::new();
    for atom in &positives {
        for term in &atom.terms {
            collect_vars_in_term(term, &mut positive_vars);
        }
    }

    for term in &rule.head.terms {
        check_all_vars_bound(term, &positive_vars, errors, &rule.span, true);
    }

    for atom in &negatives {
        for term in &atom.terms {
            check_all_vars_bound(term, &positive_vars, errors, &rule.span, false);
        }
    }
}

fn check_all_vars_bound(
    term: &LogicTerm,
    positive_vars: &HashSet<String>,
    errors: &mut Vec<Diagnostic>,
    span: &crate::diagnostics::Span,
    in_head: bool,
) {
    match term {
        LogicTerm::Var(v) => {
            if !positive_vars.contains(v) {
                errors.push(Diagnostic::new(
                    "E-RESOLVE",
                    if in_head {
                        format!(
                            "unsafe rule: head variable ?{} is not bound in positive body",
                            v
                        )
                    } else {
                        format!("unsafe rule: negated variable ?{} is not bound", v)
                    },
                    Some(span.clone()),
                ));
            }
        }
        LogicTerm::Ctor { args, .. } => {
            for arg in args {
                check_all_vars_bound(arg, positive_vars, errors, span, in_head);
            }
        }
        LogicTerm::Symbol(_) | LogicTerm::Int(_) | LogicTerm::Bool(_) => {}
    }
}

fn validate_defn(
    defn: &Defn,
    sort_set: &HashSet<String>,
    data_map: &HashMap<String, &crate::ast::DataDecl>,
    relation_sorts: &HashMap<String, Vec<String>>,
    function_sigs: &HashMap<String, (Vec<Type>, Type)>,
    constructor_map: &HashMap<String, ConstructorSig>,
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
        if let Some(msg) = validate_type(
            &p.ty,
            sort_set,
            data_map,
            relation_sorts,
            &param_names,
            constructor_map,
        ) {
            errors.push(Diagnostic::new("E-RESOLVE", msg, Some(p.span.clone())));
        }
    }

    if let Some(msg) = validate_type(
        &defn.ret_type,
        sort_set,
        data_map,
        relation_sorts,
        &param_names,
        constructor_map,
    ) {
        errors.push(Diagnostic::new("E-RESOLVE", msg, Some(defn.span.clone())));
    }

    validate_expr_names(
        &defn.body,
        &param_names,
        function_sigs,
        relation_sorts,
        constructor_map,
        errors,
    );
}

fn validate_type(
    ty: &Type,
    sort_set: &HashSet<String>,
    data_map: &HashMap<String, &crate::ast::DataDecl>,
    relation_sorts: &HashMap<String, Vec<String>>,
    scope: &HashSet<String>,
    constructor_map: &HashMap<String, ConstructorSig>,
) -> Option<String> {
    match ty {
        Type::Bool | Type::Int | Type::Symbol => None,
        Type::Domain(s) => {
            if is_known_type_name(s, sort_set, data_map) {
                None
            } else {
                Some(format!("unknown type: {s}"))
            }
        }
        Type::Adt(s) => {
            if data_map.contains_key(s) {
                None
            } else {
                Some(format!("unknown ADT type: {s}"))
            }
        }
        Type::Fun(args, ret) => {
            for a in args {
                if let Some(msg) = validate_type(
                    a,
                    sort_set,
                    data_map,
                    relation_sorts,
                    scope,
                    constructor_map,
                ) {
                    return Some(msg);
                }
            }
            validate_type(
                ret,
                sort_set,
                data_map,
                relation_sorts,
                scope,
                constructor_map,
            )
        }
        Type::Refine { var, base, formula } => {
            if let Some(msg) = validate_type(
                base,
                sort_set,
                data_map,
                relation_sorts,
                scope,
                constructor_map,
            ) {
                return Some(msg);
            }
            let mut next_scope = scope.clone();
            next_scope.insert(var.clone());
            validate_formula(formula, relation_sorts, &next_scope, constructor_map)
        }
    }
}

fn validate_formula(
    formula: &Formula,
    relation_sorts: &HashMap<String, Vec<String>>,
    scope: &HashSet<String>,
    constructor_map: &HashMap<String, ConstructorSig>,
) -> Option<String> {
    match formula {
        Formula::True => None,
        Formula::Atom(atom) => {
            let Some(sorts) = relation_sorts.get(&atom.pred) else {
                return Some(format!(
                    "unknown predicate in refinement/assert: {}",
                    atom.pred
                ));
            };
            if sorts.len() != atom.terms.len() {
                return Some(format!(
                    "arity mismatch in predicate {}: expected {}, got {}",
                    atom.pred,
                    sorts.len(),
                    atom.terms.len()
                ));
            }
            for t in &atom.terms {
                match t {
                    LogicTerm::Var(v) if !scope.contains(v) => {
                        return Some(format!("unknown variable in formula: {v}"));
                    }
                    LogicTerm::Ctor { name, args } => {
                        let Some(sig) = constructor_map.get(name) else {
                            return Some(format!("unknown constructor in formula: {name}"));
                        };
                        if sig.arity != args.len() {
                            return Some(format!(
                                "constructor {} arity mismatch: expected {}, got {}",
                                name,
                                sig.arity,
                                args.len()
                            ));
                        }
                        for arg in args {
                            if let Some(msg) = validate_formula_term(arg, scope, constructor_map) {
                                return Some(msg);
                            }
                        }
                    }
                    _ => {}
                }
            }
            None
        }
        Formula::And(items) => {
            for item in items {
                if let Some(msg) = validate_formula(item, relation_sorts, scope, constructor_map) {
                    return Some(msg);
                }
            }
            None
        }
        Formula::Not(inner) => validate_formula(inner, relation_sorts, scope, constructor_map),
    }
}

fn validate_formula_term(
    term: &LogicTerm,
    scope: &HashSet<String>,
    constructor_map: &HashMap<String, ConstructorSig>,
) -> Option<String> {
    match term {
        LogicTerm::Var(v) => {
            if scope.contains(v) {
                None
            } else {
                Some(format!("unknown variable in formula: {v}"))
            }
        }
        LogicTerm::Ctor { name, args } => {
            let Some(sig) = constructor_map.get(name) else {
                return Some(format!("unknown constructor in formula: {name}"));
            };
            if sig.arity != args.len() {
                return Some(format!(
                    "constructor {} arity mismatch: expected {}, got {}",
                    name,
                    sig.arity,
                    args.len()
                ));
            }
            for arg in args {
                if let Some(msg) = validate_formula_term(arg, scope, constructor_map) {
                    return Some(msg);
                }
            }
            None
        }
        LogicTerm::Symbol(_) | LogicTerm::Int(_) | LogicTerm::Bool(_) => None,
    }
}

fn validate_expr_names(
    expr: &Expr,
    scope: &HashSet<String>,
    function_sigs: &HashMap<String, (Vec<Type>, Type)>,
    relation_sorts: &HashMap<String, Vec<String>>,
    constructor_map: &HashMap<String, ConstructorSig>,
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
            if !function_sigs.contains_key(name)
                && !relation_sorts.contains_key(name)
                && !constructor_map.contains_key(name)
            {
                errors.push(Diagnostic::new(
                    "E-RESOLVE",
                    format!("unknown function/relation/constructor: {name}"),
                    Some(span.clone()),
                ));
            }
            for arg in args {
                validate_expr_names(
                    arg,
                    scope,
                    function_sigs,
                    relation_sorts,
                    constructor_map,
                    errors,
                );
            }
        }
        Expr::Let {
            bindings,
            body,
            span: _,
        } => {
            let mut local_scope = scope.clone();
            for (name, bexpr, bspan) in bindings {
                validate_expr_names(
                    bexpr,
                    &local_scope,
                    function_sigs,
                    relation_sorts,
                    constructor_map,
                    errors,
                );
                if local_scope.contains(name) {
                    errors.push(Diagnostic::new(
                        "E-RESOLVE",
                        format!("duplicate or shadowed let binding: {name}"),
                        Some(bspan.clone()),
                    ));
                }
                local_scope.insert(name.clone());
            }
            validate_expr_names(
                body,
                &local_scope,
                function_sigs,
                relation_sorts,
                constructor_map,
                errors,
            );
        }
        Expr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            validate_expr_names(
                cond,
                scope,
                function_sigs,
                relation_sorts,
                constructor_map,
                errors,
            );
            validate_expr_names(
                then_branch,
                scope,
                function_sigs,
                relation_sorts,
                constructor_map,
                errors,
            );
            validate_expr_names(
                else_branch,
                scope,
                function_sigs,
                relation_sorts,
                constructor_map,
                errors,
            );
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            validate_expr_names(
                scrutinee,
                scope,
                function_sigs,
                relation_sorts,
                constructor_map,
                errors,
            );
            for arm in arms {
                let mut arm_scope = scope.clone();
                let mut vars = HashSet::new();
                validate_pattern_names(&arm.pattern, constructor_map, &mut vars, errors);
                arm_scope.extend(vars);
                validate_expr_names(
                    &arm.body,
                    &arm_scope,
                    function_sigs,
                    relation_sorts,
                    constructor_map,
                    errors,
                );
            }
        }
    }
}

fn validate_pattern_names(
    pattern: &Pattern,
    constructor_map: &HashMap<String, ConstructorSig>,
    vars: &mut HashSet<String>,
    errors: &mut Vec<Diagnostic>,
) {
    match pattern {
        Pattern::Wildcard { .. }
        | Pattern::Symbol { .. }
        | Pattern::Int { .. }
        | Pattern::Bool { .. } => {}
        Pattern::Var { name, span } => {
            if vars.contains(name) {
                errors.push(Diagnostic::new(
                    "E-RESOLVE",
                    format!("duplicate pattern variable: {name}"),
                    Some(span.clone()),
                ));
            }
            vars.insert(name.clone());
        }
        Pattern::Ctor { name, args, span } => {
            let Some(sig) = constructor_map.get(name) else {
                errors.push(Diagnostic::new(
                    "E-RESOLVE",
                    format!("unknown constructor in pattern: {name}"),
                    Some(span.clone()),
                ));
                return;
            };
            if sig.arity != args.len() {
                errors.push(Diagnostic::new(
                    "E-RESOLVE",
                    format!(
                        "constructor {} arity mismatch in pattern: expected {}, got {}",
                        name,
                        sig.arity,
                        args.len()
                    ),
                    Some(span.clone()),
                ));
            }
            for arg in args {
                validate_pattern_names(arg, constructor_map, vars, errors);
            }
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

fn validate_constructor_term(
    term: &LogicTerm,
    constructor_map: &HashMap<String, ConstructorSig>,
    errors: &mut Vec<Diagnostic>,
    span: &crate::diagnostics::Span,
) {
    match term {
        LogicTerm::Ctor { name, args } => {
            let Some(sig) = constructor_map.get(name) else {
                errors.push(Diagnostic::new(
                    "E-RESOLVE",
                    format!("unknown constructor: {name}"),
                    Some(span.clone()),
                ));
                return;
            };
            if sig.arity != args.len() {
                errors.push(Diagnostic::new(
                    "E-RESOLVE",
                    format!(
                        "constructor {} arity mismatch: expected {}, got {}",
                        name,
                        sig.arity,
                        args.len()
                    ),
                    Some(span.clone()),
                ));
            }
            for arg in args {
                validate_constructor_term(arg, constructor_map, errors, span);
            }
        }
        LogicTerm::Var(_) | LogicTerm::Symbol(_) | LogicTerm::Int(_) | LogicTerm::Bool(_) => {}
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

fn logic_term_contains_var(term: &LogicTerm) -> bool {
    match term {
        LogicTerm::Var(_) => true,
        LogicTerm::Ctor { args, .. } => args.iter().any(logic_term_contains_var),
        LogicTerm::Symbol(_) | LogicTerm::Int(_) | LogicTerm::Bool(_) => false,
    }
}

fn is_known_type_name(
    name: &str,
    sort_set: &HashSet<String>,
    data_map: &HashMap<String, &crate::ast::DataDecl>,
) -> bool {
    matches!(name, "Bool" | "Int" | "Symbol")
        || sort_set.contains(name)
        || data_map.contains_key(name)
}
