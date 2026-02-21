use dtl::{check_program, parse_program};

#[test]
fn stratify_accepts_positive_cycle() {
    let src = r#"
        (sort Subject)
        (relation p (Subject))
        (relation q (Subject))
        (rule (p ?x) (q ?x))
        (rule (q ?x) (p ?x))
    "#;

    let program = parse_program(src).expect("parse should succeed");
    let report = check_program(&program).expect("stratify should succeed");
    assert_eq!(report.errors, 0);
}

#[test]
fn stratify_rejects_negative_cycle() {
    let src = r#"
        (sort Subject)
        (relation seed (Subject))
        (relation p (Subject))
        (relation q (Subject))
        (fact seed a)
        (rule (p ?x) (and (seed ?x) (not (q ?x))))
        (rule (q ?x) (and (seed ?x) (p ?x)))
    "#;

    let program = parse_program(src).expect("parse should succeed");
    let errors = check_program(&program).expect_err("stratify should fail");
    assert!(errors.iter().any(|d| d.code == "E-STRATIFY"));
}

#[test]
fn stratify_rejects_self_negation() {
    let src = r#"
        (sort Subject)
        (relation seed (Subject))
        (relation p (Subject))
        (fact seed a)
        (rule (p ?x) (and (seed ?x) (not (p ?x))))
    "#;

    let program = parse_program(src).expect("parse should succeed");
    let errors = check_program(&program).expect_err("stratify should fail");
    assert!(errors.iter().any(|d| d.code == "E-STRATIFY"));
}
