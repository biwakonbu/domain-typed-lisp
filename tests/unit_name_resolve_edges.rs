use dtl::{check_program, parse_program};

fn expect_resolve_error(src: &str, needle: &str) {
    let program = parse_program(src).expect("parse should succeed");
    let errs = check_program(&program).expect_err("resolve should fail");
    assert!(errs.iter().any(|d| d.code == "E-RESOLVE"));
    assert!(errs.iter().any(|d| d.message.contains(needle)));
}

#[test]
fn resolve_rejects_duplicate_relation() {
    expect_resolve_error(
        "(sort A) (relation r (A)) (relation r (A))",
        "duplicate relation",
    );
}

#[test]
fn resolve_rejects_unknown_sort_in_relation() {
    expect_resolve_error("(relation r (Unknown))", "unknown sort in relation");
}

#[test]
fn resolve_rejects_fact_arity_mismatch() {
    expect_resolve_error(
        "(sort A) (relation r (A A)) (fact r x)",
        "arity mismatch in fact",
    );
}

#[test]
fn resolve_rejects_undefined_relation_in_rule_head() {
    expect_resolve_error(
        "(sort A) (relation p (A)) (rule (q ?x) (p ?x))",
        "undefined relation in rule head",
    );
}

#[test]
fn resolve_rejects_undefined_relation_in_rule_body() {
    expect_resolve_error(
        "(sort A) (relation p (A)) (rule (p ?x) (q ?x))",
        "undefined relation in rule body",
    );
}

#[test]
fn resolve_rejects_unsafe_head_variable() {
    expect_resolve_error(
        "(sort A) (relation p (A)) (relation q (A)) (rule (p ?x) (q a))",
        "unsafe rule: head variable",
    );
}

#[test]
fn resolve_rejects_unsafe_negated_variable() {
    expect_resolve_error(
        "(sort A) (relation p (A)) (relation q (A)) (relation seed (A)) (fact seed a) (rule (p ?x) (and (seed ?x) (not (q ?y))))",
        "unsafe rule: negated variable",
    );
}

#[test]
fn resolve_rejects_duplicate_function() {
    expect_resolve_error(
        "(defn f ((x Symbol)) Bool true) (defn f ((x Symbol)) Bool true)",
        "duplicate function",
    );
}

#[test]
fn resolve_rejects_duplicate_param() {
    expect_resolve_error(
        "(defn f ((x Symbol) (x Symbol)) Bool true)",
        "duplicate parameter name",
    );
}

#[test]
fn resolve_rejects_unknown_predicate_in_refinement() {
    expect_resolve_error(
        "(defn f ((x Symbol)) (Refine b Bool (p x)) true)",
        "unknown predicate in refinement",
    );
}

#[test]
fn resolve_rejects_arity_mismatch_in_refinement_predicate() {
    expect_resolve_error(
        "(relation p (Symbol Symbol)) (defn f ((x Symbol)) (Refine b Bool (p x)) true)",
        "arity mismatch in refinement predicate",
    );
}

#[test]
fn resolve_rejects_shadowed_let_binding() {
    expect_resolve_error(
        "(defn f ((x Symbol)) Symbol (let ((y x) (y x)) y))",
        "duplicate or shadowed let binding",
    );
}
