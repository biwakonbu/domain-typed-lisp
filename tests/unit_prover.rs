use dtl::{has_failed_obligation, parse_program, prove_program};

#[test]
fn prove_program_succeeds_and_emits_schema_version() {
    let src = r#"
        (data Subject (alice) (bob))
        (relation allowed (Subject))
        (fact allowed (alice))
        (universe Subject ((alice) (bob)))

        (assert consistency ((u Subject))
          (not (and (allowed u) (not (allowed u)))))

        (defn witness ((u Subject))
          (Refine b Bool (allowed u))
          (allowed u))
    "#;

    let program = parse_program(src).expect("parse");
    let trace = prove_program(&program).expect("prove should succeed");
    assert_eq!(trace.schema_version, "1.0.0");
    assert!(!has_failed_obligation(&trace));
    assert!(trace.obligations.iter().all(|o| o.result == "proved"));
}

#[test]
fn prove_program_reports_counterexample() {
    let src = r#"
        (data Subject (alice) (bob))
        (relation allowed (Subject))
        (fact allowed (alice))
        (universe Subject ((alice) (bob)))

        (assert everyone-allowed ((u Subject))
          (allowed u))
    "#;

    let program = parse_program(src).expect("parse");
    let trace = prove_program(&program).expect("prove should run");
    assert!(has_failed_obligation(&trace));

    let failed = trace
        .obligations
        .iter()
        .find(|o| o.result == "failed")
        .expect("failed obligation");
    assert!(failed.counterexample.is_some());
    assert!(
        failed
            .counterexample
            .as_ref()
            .expect("counterexample")
            .missing_goals
            .iter()
            .any(|g| g.contains("allowed"))
    );
}

#[test]
fn prove_program_requires_universe() {
    let src = r#"
        (data Subject (alice) (bob))
        (relation allowed (Subject))
        (assert everyone-allowed ((u Subject))
          (allowed u))
    "#;

    let program = parse_program(src).expect("parse");
    let errs = prove_program(&program).expect_err("prove should fail");
    assert!(errs.iter().any(|d| d.code == "E-PROVE"));
    assert!(
        errs.iter()
            .any(|d| d.message.contains("missing universe declaration"))
    );
}
