use std::collections::{HashMap, HashSet};

use crate::ast::Program;
use crate::diagnostics::Diagnostic;
use crate::types::Formula;

pub fn compute_strata(program: &Program) -> Result<HashMap<String, usize>, Vec<Diagnostic>> {
    let relation_names: HashSet<String> =
        program.relations.iter().map(|r| r.name.clone()).collect();
    let n = relation_names.len().max(1);
    let mut strata: HashMap<String, usize> = relation_names
        .iter()
        .map(|name| (name.clone(), 0usize))
        .collect();

    let mut dependencies = Vec::new();
    let mut errors = Vec::new();

    for rule in &program.rules {
        let mut pos = Vec::new();
        let mut neg = Vec::new();
        flatten_formula(&rule.body, false, &mut pos, &mut neg);

        for p in pos {
            dependencies.push((
                rule.head.pred.clone(),
                p.pred.clone(),
                false,
                rule.span.clone(),
            ));
        }
        for p in neg {
            if p.pred == rule.head.pred {
                errors.push(Diagnostic::new(
                    "E-STRATIFY",
                    format!("self-negation detected on relation {}", p.pred),
                    Some(rule.span.clone()),
                ));
            }
            dependencies.push((
                rule.head.pred.clone(),
                p.pred.clone(),
                true,
                rule.span.clone(),
            ));
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    for _ in 0..(n * n + 1) {
        let mut changed = false;
        for (head, dep, is_neg, _) in &dependencies {
            let sh = *strata.get(head).unwrap_or(&0);
            let sd = *strata.get(dep).unwrap_or(&0);
            let required = if *is_neg { sd + 1 } else { sd };
            if sh < required {
                strata.insert(head.clone(), required);
                if required > n {
                    errors.push(Diagnostic::new(
                        "E-STRATIFY",
                        "negative dependency cycle detected",
                        None,
                    ));
                    return Err(errors);
                }
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    for (head, dep, is_neg, span) in &dependencies {
        let sh = *strata.get(head).unwrap_or(&0);
        let sd = *strata.get(dep).unwrap_or(&0);
        if (!is_neg && sh < sd) || (*is_neg && sh <= sd) {
            errors.push(Diagnostic::new(
                "E-STRATIFY",
                format!(
                    "stratification constraint violated: {} {} {}",
                    head,
                    if *is_neg { ">" } else { ">=" },
                    dep
                ),
                Some(span.clone()),
            ));
        }
    }

    if errors.is_empty() {
        Ok(strata)
    } else {
        Err(errors)
    }
}

fn flatten_formula<'a>(
    formula: &'a Formula,
    negated: bool,
    pos: &mut Vec<&'a crate::types::Atom>,
    neg: &mut Vec<&'a crate::types::Atom>,
) {
    match formula {
        Formula::True => {}
        Formula::Atom(atom) => {
            if negated {
                neg.push(atom);
            } else {
                pos.push(atom);
            }
        }
        Formula::And(items) => {
            for item in items {
                flatten_formula(item, negated, pos, neg);
            }
        }
        Formula::Not(inner) => flatten_formula(inner, !negated, pos, neg),
    }
}
