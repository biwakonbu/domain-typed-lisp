use dtl::{check_program, parse_program};

fn expect_type_error(src: &str, code: &str, needle: &str) {
    let program = parse_program(src).expect("parse should succeed");
    let errs = check_program(&program).expect_err("typecheck should fail");
    assert!(errs.iter().any(|d| d.code == code));
    assert!(errs.iter().any(|d| d.message.contains(needle)));
}

#[test]
fn typecheck_rejects_if_cond_non_bool() {
    expect_type_error(
        "(defn f ((x Symbol)) Bool (if x true false))",
        "E-TYPE",
        "if condition must be Bool",
    );
}

#[test]
fn typecheck_rejects_if_branch_mismatch() {
    expect_type_error(
        "(defn f ((x Bool)) Symbol (if x one 1))",
        "E-TYPE",
        "if branches have incompatible types",
    );
}

#[test]
fn typecheck_rejects_function_arity_mismatch() {
    expect_type_error(
        "(defn callee ((x Symbol)) Bool true) (defn caller ((x Symbol)) Bool (callee x x))",
        "E-TYPE",
        "arity mismatch",
    );
}

#[test]
fn typecheck_rejects_relation_arity_mismatch() {
    expect_type_error(
        "(relation p (Symbol)) (defn f ((x Symbol)) Bool (p x x))",
        "E-TYPE",
        "relation p arity mismatch",
    );
}

#[test]
fn typecheck_rejects_non_literal_relation_arg() {
    expect_type_error(
        "(relation p (Symbol)) (defn id ((x Symbol)) Symbol x) (defn f ((x Symbol)) Bool (p (id x)))",
        "E-TYPE",
        "relation argument must be variable or literal",
    );
}

#[test]
fn typecheck_accepts_entailment_via_rule() {
    let src = r#"
        (sort Subject)
        (sort Resource)
        (sort Action)
        (relation can-access (Subject Resource Action))
        (relation granted (Subject Resource))
        (rule (can-access ?u ?r read) (granted ?u ?r))

        (defn f ((u Subject) (r Resource))
          (Refine b Bool (can-access u r read))
          (granted u r))
    "#;
    let program = parse_program(src).expect("parse should succeed");
    let report = check_program(&program).expect("typecheck should succeed");
    assert_eq!(report.errors, 0);
}

#[test]
fn typecheck_rejects_return_type_mismatch() {
    expect_type_error("(defn f ((x Symbol)) Int x)", "E-TYPE", "type mismatch");
}
