use dtl::{check_program, parse_program};

fn expect_error(src: &str, code: &str, needle: &str) {
    let program = parse_program(src).expect("parse should succeed");
    let errs = check_program(&program).expect_err("check should fail");
    assert!(errs.iter().any(|d| d.code == code), "errs={errs:?}");
    assert!(
        errs.iter().any(|d| d.message.contains(needle)),
        "errs={errs:?}"
    );
}

#[test]
fn resolve_accepts_recursive_data() {
    let src = "(data List (nil) (cons Symbol List))";
    let program = parse_program(src).expect("parse should succeed");
    let report = check_program(&program).expect("check should succeed");
    assert_eq!(report.errors, 0);
}

#[test]
fn resolve_rejects_duplicate_constructor() {
    expect_error(
        "(data Subject (alice)) (data Resource (alice))",
        "E-DATA",
        "duplicate constructor",
    );
}

#[test]
fn resolve_rejects_unknown_universe_type() {
    expect_error(
        "(universe Missing (x))",
        "E-RESOLVE",
        "unknown universe type",
    );
}

#[test]
fn resolve_rejects_universe_value_of_wrong_constructor_family() {
    expect_error(
        "(data Subject (alice)) (data Resource (doc1)) (universe Subject ((doc1)))",
        "E-DATA",
        "belongs to Resource, expected Subject",
    );
}

#[test]
fn resolve_rejects_unknown_constructor_in_pattern() {
    expect_error(
        "(data Subject (alice)) (defn f ((u Subject)) Bool (match u ((unknown) true)))",
        "E-RESOLVE",
        "unknown constructor in pattern",
    );
}

#[test]
fn resolve_accepts_alias_for_constructor_usage() {
    let src = r#"
        (data Action (read) (write))
        (alias 閲覧 read)
        (defn ok ((a Action)) Bool
          (match a
            ((閲覧) true)
            ((write) false)))
    "#;
    let program = parse_program(src).expect("parse should succeed");
    let report = check_program(&program).expect("check should succeed");
    assert_eq!(report.errors, 0);
}

#[test]
fn resolve_rejects_alias_to_unknown_canonical_constructor() {
    expect_error(
        "(data Action (read)) (alias 閲覧 unknown)",
        "E-RESOLVE",
        "alias canonical constructor is undefined",
    );
}

#[test]
fn resolve_rejects_alias_cycle() {
    expect_error(
        "(data Action (read)) (alias A B) (alias B A)",
        "E-RESOLVE",
        "alias cycle detected",
    );
}

#[test]
fn resolve_rejects_alias_conflict_with_constructor() {
    expect_error(
        "(data Action (read)) (alias read read)",
        "E-DATA",
        "alias conflicts with constructor",
    );
}
