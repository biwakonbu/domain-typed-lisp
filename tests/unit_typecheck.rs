use dtl::{check_program, parse_program};

#[test]
fn typecheck_accepts_refinement_return() {
    let src = r#"
        (sort Subject)
        (sort Resource)
        (data Action (read))
        (relation can-access (Subject Resource Action))

        (defn can-read ((u Subject) (r Resource))
          (Refine b Bool (can-access u r (read)))
          (can-access u r (read)))
    "#;

    let program = parse_program(src).expect("parse should succeed");
    let report = check_program(&program).expect("typecheck should succeed");
    assert_eq!(report.errors, 0);
}

#[test]
fn typecheck_rejects_argument_type_mismatch() {
    let src = r#"
        (sort Subject)
        (sort Resource)
        (data Action (read))
        (relation can-access (Subject Resource Action))

        (defn expects-subject ((u Subject)) Bool true)
        (defn caller ((r Resource)) Bool (expects-subject r))
    "#;

    let program = parse_program(src).expect("parse should succeed");
    let errors = check_program(&program).expect_err("typecheck should fail");
    assert!(errors.iter().any(|d| d.code == "E-TYPE"));
}

#[test]
fn typecheck_rejects_unprovable_entailment() {
    let src = r#"
        (sort Subject)
        (sort Resource)
        (data Action (read))
        (relation can-access (Subject Resource Action))
        (relation has-role (Subject Symbol))

        (defn broken ((u Subject) (r Resource))
          (Refine b Bool (can-access u r (read)))
          (has-role u admin))
    "#;

    let program = parse_program(src).expect("parse should succeed");
    let errors = check_program(&program).expect_err("typecheck should fail");
    assert!(errors.iter().any(|d| d.code == "E-ENTAIL"));
}
