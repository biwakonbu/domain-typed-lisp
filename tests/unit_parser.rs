use dtl::parse_program;

#[test]
fn parser_accepts_minimal_program() {
    let src = r#"
        (sort Subject)
        (sort Role)
        (relation has-role (Subject Role))
        (fact has-role alice admin)
    "#;

    let program = parse_program(src).expect("parse should succeed");
    assert_eq!(program.sorts.len(), 2);
    assert_eq!(program.relations.len(), 1);
    assert_eq!(program.facts.len(), 1);
}

#[test]
fn parser_reports_unbalanced_parentheses() {
    let src = r#"
        (sort Subject
        (relation has-role (Subject Role))
    "#;

    let errors = parse_program(src).expect_err("parse should fail");
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|d| d.code == "E-PARSE"));
    assert!(errors.iter().any(|d| d.span.is_some()));
}

#[test]
fn parser_rejects_unknown_toplevel_form() {
    let src = r#"
        (unknown abc)
    "#;

    let errors = parse_program(src).expect_err("parse should fail");
    assert!(
        errors
            .iter()
            .any(|d| d.message.contains("unknown top-level form"))
    );
}
