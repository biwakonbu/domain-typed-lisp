use std::collections::HashMap;

use dtl::ast::Rule;
use dtl::logic_engine::{GroundFact, KnowledgeBase, Value, solve_facts};
use dtl::types::{Atom, Formula, LogicTerm};

#[test]
fn solve_facts_rejects_undefined_fact_relation() {
    let kb = KnowledgeBase {
        relation_schemas: HashMap::new(),
        facts: vec![GroundFact {
            pred: "p".to_string(),
            terms: vec![Value::Symbol("a".to_string())],
        }],
        rules: vec![],
        strata: HashMap::new(),
    };

    let errs = solve_facts(&kb).expect_err("solve should fail");
    assert!(
        errs.iter()
            .any(|d| d.message.contains("undefined relation in fact"))
    );
}

#[test]
fn solve_facts_rejects_unbound_head_variable() {
    let mut relation_schemas = HashMap::new();
    relation_schemas.insert("p".to_string(), vec!["Symbol".to_string()]);

    let rule = Rule {
        head: Atom {
            pred: "p".to_string(),
            terms: vec![LogicTerm::Var("x".to_string())],
        },
        body: Formula::True,
        span: dtl::Span {
            start: 0,
            end: 0,
            line: 1,
            column: 1,
            file_id: None,
        },
    };

    let mut strata = HashMap::new();
    strata.insert("p".to_string(), 0);

    let kb = KnowledgeBase {
        relation_schemas,
        facts: vec![],
        rules: vec![rule],
        strata,
    };

    let errs = solve_facts(&kb).expect_err("solve should fail");
    assert!(
        errs.iter()
            .any(|d| d.message.contains("unbound head variable"))
    );
}
