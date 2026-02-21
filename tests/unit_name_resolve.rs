use dtl::{check_program, parse_program};

#[test]
fn resolve_rejects_undefined_relation_in_fact() {
    let src = r#"
        (sort Subject)
        (sort Role)
        (fact has-role alice admin)
    "#;

    let program = parse_program(src).expect("parse should succeed");
    let errors = check_program(&program).expect_err("resolve should fail");
    assert!(errors.iter().any(|d| d.code == "E-RESOLVE"));
    assert!(
        errors
            .iter()
            .any(|d| d.message.contains("undefined relation"))
    );
}

#[test]
fn resolve_rejects_duplicate_sort() {
    let src = r#"
        (sort Subject)
        (sort Subject)
    "#;

    let program = parse_program(src).expect("parse should succeed");
    let errors = check_program(&program).expect_err("resolve should fail");
    assert!(errors.iter().any(|d| d.code == "E-RESOLVE"));
    assert!(errors.iter().any(|d| d.message.contains("duplicate sort")));
}

#[test]
fn resolve_rejects_unknown_type_in_defn() {
    let src = r#"
        (defn f ((x UnknownSort)) Bool true)
    "#;

    let program = parse_program(src).expect("parse should succeed");
    let errors = check_program(&program).expect_err("resolve should fail");
    assert!(errors.iter().any(|d| d.message.contains("unknown type")));
}
