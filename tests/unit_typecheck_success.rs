use dtl::{check_program, parse_program};

#[test]
fn typecheck_accepts_let_and_if_paths() {
    let src = r#"
        (defn f ((x Bool) (s Symbol))
          Symbol
          (if x
              (let ((y s)) y)
              s))
    "#;
    let program = parse_program(src).expect("parse");
    let report = check_program(&program).expect("typecheck should pass");
    assert_eq!(report.errors, 0);
}

#[test]
fn typecheck_accepts_function_value_and_call() {
    let src = r#"
        (defn is-admin ((x Symbol)) Bool true)
        (defn caller ((x Symbol)) Bool (is-admin x))
    "#;
    let program = parse_program(src).expect("parse");
    assert!(check_program(&program).is_ok());
}

#[test]
fn typecheck_rejects_domain_as_symbol_argument_without_constructor() {
    let src = r#"
        (sort Subject)
        (defn takes-symbol ((x Symbol)) Bool true)
        (defn caller ((u Subject)) Bool (takes-symbol u))
    "#;
    let program = parse_program(src).expect("parse");
    let errs = check_program(&program).expect_err("typecheck should fail");
    assert!(errs.iter().any(|d| d.code == "E-TYPE"));
}

#[test]
fn typecheck_accepts_relation_bool_literal_argument() {
    let src = r#"
        (relation rb (Bool))
        (defn f ((x Symbol)) Bool (rb true))
    "#;
    let program = parse_program(src).expect("parse");
    assert!(check_program(&program).is_ok());
}

#[test]
fn typecheck_handles_refinement_not_formula() {
    let src = r#"
        (relation p (Symbol))
        (defn f ((x Symbol))
          (Refine b Bool (not (p x)))
          true)
    "#;
    let program = parse_program(src).expect("parse");
    let report = check_program(&program).expect("should pass under CWA");
    assert_eq!(report.errors, 0);
}
