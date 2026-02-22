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

#[test]
fn typecheck_rejects_symbol_for_japanese_adt_argument() {
    let src = r#"
        (data 顧客種別 (法人) (個人))
        (defn 契約可能か ((種別 顧客種別)) Bool
          (match 種別
            ((法人) true)
            ((個人) false)))
        (defn 呼び出し側 ((x Symbol)) Bool (契約可能か x))
    "#;

    let program = parse_program(src).expect("parse");
    let errs = check_program(&program).expect_err("should fail");
    assert!(errs.iter().any(|d| d.code == "E-TYPE"));
    assert!(
        errs.iter()
            .any(|d| d.message.contains("function argument type mismatch"))
    );
}

#[test]
fn typecheck_treats_nfc_equivalent_sort_names_as_same() {
    let decomposed = "\u{30AB}\u{3099}";
    let src = format!(
        r#"
        (sort ガ)
        (sort {decomposed})
        "#
    );

    let program = parse_program(&src).expect("parse");
    let errs = check_program(&program).expect_err("should fail");
    assert!(errs.iter().any(|d| d.code == "E-RESOLVE"));
    assert!(
        errs.iter()
            .any(|d| d.message.contains("duplicate sort: ガ"))
    );
}
