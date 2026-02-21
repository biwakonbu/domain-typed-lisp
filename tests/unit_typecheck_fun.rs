use dtl::{check_program, parse_program};

#[test]
fn typecheck_rejects_function_type_argument_mismatch() {
    let src = r#"
        (defn make ((x Symbol)) (-> (Symbol) Bool) x)
    "#;
    let program = parse_program(src).expect("parse");
    let errs = check_program(&program).expect_err("should fail");
    assert!(errs.iter().any(|d| d.code == "E-TYPE"));
}

#[test]
fn typecheck_rejects_function_call_when_return_type_is_function() {
    let src = r#"
        (defn idf ((f (-> (Symbol) Bool))) (-> (Symbol) Bool) f)
        (defn bad ((x Symbol)) Bool (idf x))
    "#;
    let program = parse_program(src).expect("parse");
    let errs = check_program(&program).expect_err("should fail");
    assert!(errs.iter().any(|d| d.code == "E-TYPE"));
}
