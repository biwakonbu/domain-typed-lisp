use dtl::{check_program, parse_program};

#[test]
fn typecheck_rejects_recursive_function_by_totality_rule() {
    let src = r#"
        (defn loop ((x Int)) Int (loop x))
    "#;

    let program = parse_program(src).expect("parse");
    let errs = check_program(&program).expect_err("should fail");
    assert!(errs.iter().any(|d| d.code == "E-TOTAL"));
}

#[test]
fn typecheck_accepts_constructor_and_exhaustive_match() {
    let src = r#"
        (data Subject (alice) (bob))
        (defn is-alice ((u Subject)) Bool
          (match u
            ((alice) true)
            ((bob) false)))
    "#;

    let program = parse_program(src).expect("parse");
    let report = check_program(&program).expect("should pass");
    assert_eq!(report.errors, 0);
}

#[test]
fn typecheck_rejects_non_exhaustive_match() {
    let src = r#"
        (data Subject (alice) (bob))
        (defn bad ((u Subject)) Bool
          (match u
            ((alice) true)))
    "#;

    let program = parse_program(src).expect("parse");
    let errs = check_program(&program).expect_err("should fail");
    assert!(errs.iter().any(|d| d.code == "E-MATCH"));
    assert!(errs.iter().any(|d| d.message.contains("non-exhaustive")));
}

#[test]
fn typecheck_rejects_unreachable_match_arm() {
    let src = r#"
        (data Subject (alice) (bob))
        (defn bad ((u Subject)) Bool
          (match u
            (_ true)
            ((alice) false)))
    "#;

    let program = parse_program(src).expect("parse");
    let errs = check_program(&program).expect_err("should fail");
    assert!(errs.iter().any(|d| d.code == "E-MATCH"));
    assert!(errs.iter().any(|d| d.message.contains("unreachable")));
}
