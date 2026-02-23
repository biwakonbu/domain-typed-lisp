use dtl::{Diagnostic, check_program, parse_program};

fn first_totality_error(errors: &[Diagnostic]) -> &Diagnostic {
    errors
        .iter()
        .find(|d| d.code == "E-TOTAL")
        .expect("E-TOTAL should exist")
}

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
fn typecheck_accepts_structural_tail_recursion() {
    let src = r#"
        (data Nat (z) (s Nat))
        (defn is-zero-chain ((n Nat)) Bool
          (match n
            ((z) true)
            ((s m) (is-zero-chain m))))
    "#;

    let program = parse_program(src).expect("parse");
    let report = check_program(&program).expect("should pass");
    assert_eq!(report.errors, 0);
}

#[test]
fn typecheck_rejects_non_tail_recursive_call() {
    let src = r#"
        (data Nat (z) (s Nat))
        (defn bad ((n Nat)) Bool
          (match n
            ((z) true)
            ((s m) (if (bad m) true false))))
    "#;

    let program = parse_program(src).expect("parse");
    let errs = check_program(&program).expect_err("should fail");
    let diag = first_totality_error(&errs);
    assert!(diag.message.contains("tail position"));
    assert_eq!(diag.reason(), Some("non_tail_recursive_call"));
    assert_eq!(diag.arg_indices(), None);
}

#[test]
fn typecheck_rejects_non_decreasing_recursive_call() {
    let src = r#"
        (data Nat (z) (s Nat))
        (defn bad ((n Nat)) Bool
          (match n
            ((z) true)
            ((s m) (bad n))))
    "#;

    let program = parse_program(src).expect("parse");
    let errs = check_program(&program).expect_err("should fail");
    let diag = first_totality_error(&errs);
    assert!(diag.message.contains("argument index"));
    assert_eq!(diag.reason(), Some("non_decreasing_argument"));
    assert_eq!(diag.arg_indices(), Some(&[1][..]));
}

#[test]
fn typecheck_rejects_mutual_recursion() {
    let src = r#"
        (data Nat (z) (s Nat))
        (defn f ((n Nat)) Bool (g n))
        (defn g ((n Nat)) Bool (f n))
    "#;

    let program = parse_program(src).expect("parse");
    let errs = check_program(&program).expect_err("should fail");
    let diag = first_totality_error(&errs);
    assert!(diag.message.contains("mutual recursion"));
    assert_eq!(diag.reason(), Some("mutual_recursion"));
    assert_eq!(diag.arg_indices(), None);
}

#[test]
fn typecheck_accepts_structural_recursion_through_let_alias() {
    let src = r#"
        (data Nat (z) (s Nat))
        (defn count-down ((n Nat)) Bool
          (match n
            ((z) true)
            ((s m)
              (let ((next m))
                (count-down next)))))
    "#;

    let program = parse_program(src).expect("parse");
    let report = check_program(&program).expect("should pass");
    assert_eq!(report.errors, 0);
}

#[test]
fn typecheck_accepts_structural_recursion_under_nested_match() {
    let src = r#"
        (data Nat (z) (s Nat))
        (defn count-down ((n Nat)) Bool
          (match n
            ((z) true)
            ((s m)
              (match m
                ((z) true)
                ((s k) (count-down k))))))
    "#;

    let program = parse_program(src).expect("parse");
    let report = check_program(&program).expect("should pass");
    assert_eq!(report.errors, 0);
}

#[test]
fn typecheck_accepts_recursion_when_one_of_multiple_adt_args_decreases() {
    let src = r#"
        (data Nat (z) (s Nat))
        (defn fold-left ((a Nat) (b Nat)) Bool
          (match a
            ((z) true)
            ((s a1) (fold-left a1 b))))
    "#;

    let program = parse_program(src).expect("parse");
    let report = check_program(&program).expect("should pass");
    assert_eq!(report.errors, 0);
}

#[test]
fn typecheck_rejects_recursion_when_no_adt_arg_decreases_with_multiple_adt_args() {
    let src = r#"
        (data Nat (z) (s Nat))
        (defn bad ((a Nat) (b Nat)) Bool
          (match a
            ((z) true)
            ((s a1) (bad a b))))
    "#;

    let program = parse_program(src).expect("parse");
    let errs = check_program(&program).expect_err("should fail");
    let diag = first_totality_error(&errs);
    assert_eq!(diag.reason(), Some("non_decreasing_argument"));
    assert_eq!(diag.arg_indices(), Some(&[1, 2][..]));
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
