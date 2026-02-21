use std::collections::{BTreeSet, HashMap, HashSet};

use crate::ast::{Program, Rule};
use crate::diagnostics::Diagnostic;
use crate::name_resolve::resolve_program;
use crate::stratify::compute_strata;
use crate::types::{Atom, Formula, LogicTerm};

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Value {
    Symbol(String),
    Int(i64),
    Bool(bool),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GroundFact {
    pub pred: String,
    pub terms: Vec<Value>,
}

#[derive(Debug, Clone)]
pub struct KnowledgeBase {
    pub relation_schemas: HashMap<String, Vec<String>>,
    pub facts: Vec<GroundFact>,
    pub rules: Vec<Rule>,
    pub strata: HashMap<String, usize>,
}

impl KnowledgeBase {
    pub fn from_program(program: &Program) -> Result<Self, Vec<Diagnostic>> {
        let resolve_errors = resolve_program(program);
        if !resolve_errors.is_empty() {
            return Err(resolve_errors);
        }

        let strata = compute_strata(program)?;

        let mut relation_schemas = HashMap::new();
        for rel in &program.relations {
            relation_schemas.insert(rel.name.clone(), rel.arg_sorts.clone());
        }

        let mut facts = Vec::new();
        for fact in &program.facts {
            let mut terms = Vec::new();
            for t in &fact.terms {
                let Some(value) = term_to_const_value(t) else {
                    return Err(vec![Diagnostic::new(
                        "E-RESOLVE",
                        "fact contains variable",
                        Some(fact.span.clone()),
                    )]);
                };
                terms.push(value);
            }
            facts.push(GroundFact {
                pred: fact.name.clone(),
                terms,
            });
        }

        Ok(Self {
            relation_schemas,
            facts,
            rules: program.rules.clone(),
            strata,
        })
    }

    pub fn with_extra_facts(&self, extra: Vec<GroundFact>) -> Self {
        let mut seen: HashSet<GroundFact> = self.facts.iter().cloned().collect();
        for f in extra {
            seen.insert(f);
        }
        Self {
            relation_schemas: self.relation_schemas.clone(),
            facts: seen.into_iter().collect(),
            rules: self.rules.clone(),
            strata: self.strata.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DerivedFacts {
    pub facts: HashMap<String, BTreeSet<Vec<Value>>>,
}

impl DerivedFacts {
    pub fn contains(&self, pred: &str, tuple: &[&str]) -> bool {
        let Some(set) = self.facts.get(pred) else {
            return false;
        };
        let candidate: Vec<Value> = tuple
            .iter()
            .map(|x| Value::Symbol((*x).to_string()))
            .collect();
        set.contains(&candidate)
    }

    pub fn relation_facts(&self, pred: &str) -> BTreeSet<Vec<String>> {
        self.facts
            .get(pred)
            .map(|set| {
                set.iter()
                    .map(|tuple| tuple.iter().map(value_to_string).collect::<Vec<_>>())
                    .collect::<BTreeSet<_>>()
            })
            .unwrap_or_default()
    }

    pub fn all_facts(&self) -> Vec<GroundFact> {
        let mut out = Vec::new();
        for (pred, tuples) in &self.facts {
            for terms in tuples {
                out.push(GroundFact {
                    pred: pred.clone(),
                    terms: terms.clone(),
                });
            }
        }
        out
    }
}

pub fn solve_facts(kb: &KnowledgeBase) -> Result<DerivedFacts, Vec<Diagnostic>> {
    let mut db: HashMap<String, BTreeSet<Vec<Value>>> = HashMap::new();
    for name in kb.relation_schemas.keys() {
        db.insert(name.clone(), BTreeSet::new());
    }

    for fact in &kb.facts {
        let Some(schema) = kb.relation_schemas.get(&fact.pred) else {
            return Err(vec![Diagnostic::new(
                "E-RESOLVE",
                format!("undefined relation in fact: {}", fact.pred),
                None,
            )]);
        };
        if schema.len() != fact.terms.len() {
            return Err(vec![Diagnostic::new(
                "E-RESOLVE",
                format!(
                    "arity mismatch in fact {}: expected {}, got {}",
                    fact.pred,
                    schema.len(),
                    fact.terms.len()
                ),
                None,
            )]);
        }
        db.entry(fact.pred.clone())
            .or_default()
            .insert(fact.terms.clone());
    }

    let mut strata_values: Vec<usize> = kb.strata.values().copied().collect();
    strata_values.sort_unstable();
    strata_values.dedup();

    for stratum in strata_values {
        let rules: Vec<&Rule> = kb
            .rules
            .iter()
            .filter(|r| kb.strata.get(&r.head.pred).copied().unwrap_or(0) == stratum)
            .collect();

        let mut changed = true;
        while changed {
            changed = false;
            for rule in &rules {
                let tuples = evaluate_rule(rule, &db)?;
                let target = db.entry(rule.head.pred.clone()).or_default();
                for tuple in tuples {
                    if target.insert(tuple) {
                        changed = true;
                    }
                }
            }
        }
    }

    Ok(DerivedFacts { facts: db })
}

fn evaluate_rule(
    rule: &Rule,
    db: &HashMap<String, BTreeSet<Vec<Value>>>,
) -> Result<BTreeSet<Vec<Value>>, Vec<Diagnostic>> {
    let mut positives = Vec::new();
    let mut negatives = Vec::new();
    flatten_formula(&rule.body, false, &mut positives, &mut negatives);

    let mut assignments: Vec<HashMap<String, Value>> = vec![HashMap::new()];

    for atom in positives {
        let mut next = Vec::new();
        let tuples = db.get(&atom.pred).cloned().unwrap_or_default();
        for assign in &assignments {
            for tuple in &tuples {
                if let Some(new_assign) = unify(atom, tuple, assign) {
                    next.push(new_assign);
                }
            }
        }
        assignments = next;
        if assignments.is_empty() {
            break;
        }
    }

    for atom in negatives {
        let tuples = db.get(&atom.pred).cloned().unwrap_or_default();
        assignments.retain(|assign| {
            let instantiated = instantiate_terms(&atom.terms, assign);
            let Ok(instantiated) = instantiated else {
                return false;
            };
            !tuples.contains(&instantiated)
        });
    }

    let mut produced = BTreeSet::new();
    for assign in &assignments {
        let tuple = instantiate_terms(&rule.head.terms, assign).map_err(|e| {
            vec![Diagnostic::new(
                "E-RESOLVE",
                format!("unbound head variable: {e}"),
                Some(rule.span.clone()),
            )]
        })?;
        produced.insert(tuple);
    }

    Ok(produced)
}

fn unify(
    atom: &Atom,
    tuple: &[Value],
    base: &HashMap<String, Value>,
) -> Option<HashMap<String, Value>> {
    if atom.terms.len() != tuple.len() {
        return None;
    }
    let mut env = base.clone();
    for (term, val) in atom.terms.iter().zip(tuple.iter()) {
        match term {
            LogicTerm::Var(v) => {
                if let Some(bound) = env.get(v) {
                    if bound != val {
                        return None;
                    }
                } else {
                    env.insert(v.clone(), val.clone());
                }
            }
            LogicTerm::Symbol(s) => {
                if val != &Value::Symbol(s.clone()) {
                    return None;
                }
            }
            LogicTerm::Int(i) => {
                if val != &Value::Int(*i) {
                    return None;
                }
            }
            LogicTerm::Bool(b) => {
                if val != &Value::Bool(*b) {
                    return None;
                }
            }
        }
    }
    Some(env)
}

fn instantiate_terms(
    terms: &[LogicTerm],
    env: &HashMap<String, Value>,
) -> Result<Vec<Value>, String> {
    let mut out = Vec::new();
    for t in terms {
        match t {
            LogicTerm::Var(v) => {
                let Some(val) = env.get(v) else {
                    return Err(v.clone());
                };
                out.push(val.clone());
            }
            LogicTerm::Symbol(s) => out.push(Value::Symbol(s.clone())),
            LogicTerm::Int(i) => out.push(Value::Int(*i)),
            LogicTerm::Bool(b) => out.push(Value::Bool(*b)),
        }
    }
    Ok(out)
}

fn term_to_const_value(term: &LogicTerm) -> Option<Value> {
    match term {
        LogicTerm::Var(_) => None,
        LogicTerm::Symbol(s) => Some(Value::Symbol(s.clone())),
        LogicTerm::Int(i) => Some(Value::Int(*i)),
        LogicTerm::Bool(b) => Some(Value::Bool(*b)),
    }
}

fn flatten_formula<'a>(
    formula: &'a Formula,
    negated: bool,
    pos: &mut Vec<&'a Atom>,
    neg: &mut Vec<&'a Atom>,
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

fn value_to_string(v: &Value) -> String {
    match v {
        Value::Symbol(s) => s.clone(),
        Value::Int(i) => i.to_string(),
        Value::Bool(b) => b.to_string(),
    }
}
