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
    assert_eq!(trace.schema_version, "2.0.0");
    assert_eq!(trace.profile, "standard");
    assert_eq!(trace.summary.total, trace.obligations.len());
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

#[test]
fn prove_program_extracts_if_body_obligation() {
    let src = r#"
        (data Subject (alice) (bob))
        (relation allowed (Subject))
        (relation admin (Subject))
        (fact admin (alice))
        (fact allowed (alice))
        (universe Subject ((alice) (bob)))

        (defn check-admin ((u Subject))
          (Refine b Bool (allowed u))
          (if (admin u)
              (allowed u)
              (allowed u)))
    "#;

    let program = parse_program(src).expect("parse");
    let trace = prove_program(&program).expect("prove should run");
    assert!(
        !has_failed_obligation(&trace),
        "if 本体の論理化がないと bob で偽反例になる"
    );
}

#[test]
fn prove_program_extracts_match_body_obligation() {
    let src = r#"
        (data Subject (alice) (bob))
        (relation allowed (Subject))
        (fact allowed (alice))
        (universe Subject ((alice) (bob)))

        (defn check-user ((u Subject))
          (Refine b Bool (allowed u))
          (match u
            ((alice) (allowed u))
            ((bob) (allowed u))))
    "#;

    let program = parse_program(src).expect("parse");
    let trace = prove_program(&program).expect("prove should run");
    assert!(
        !has_failed_obligation(&trace),
        "match 本体の論理化がないと bob で偽反例になる"
    );
}

#[test]
fn prove_program_reports_missing_universe_with_span() {
    let src = r#"
        (sort Subject)
        (relation allowed (Subject))
        (defn check ((u Subject))
          (Refine b Bool (allowed u))
          (allowed u))
    "#;

    let program = parse_program(src).expect("parse");
    let errs = prove_program(&program).expect_err("prove should fail");
    let diag = errs
        .iter()
        .find(|d| d.message.contains("missing universe declaration"))
        .expect("missing universe diagnostic");
    assert_eq!(diag.code, "E-PROVE");
    assert!(
        diag.span.is_some(),
        "missing universe diagnostic should have span"
    );
}
