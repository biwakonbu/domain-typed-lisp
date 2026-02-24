use dtl::parse_program;
use dtl::types::LogicTerm;

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
fn parser_accepts_import_form() {
    let src = r#"
        (import "schema.dtl")
        (sort Subject)
    "#;

    let program = parse_program(src).expect("parse should succeed");
    assert_eq!(program.imports.len(), 1);
    assert_eq!(program.imports[0].path, "schema.dtl");
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

#[test]
fn parser_accepts_japanese_identifiers() {
    let src = r#"
        (sort 主体)
        (sort 契約)
        (data 顧客種別 (法人) (個人))
        (relation 契約可能 (主体 契約 顧客種別))
        (defn 契約可能か ((u 主体) (k 契約) (種別 顧客種別)) Bool
          (契約可能 u k 種別))
    "#;

    let program = parse_program(src).expect("parse should succeed");
    assert_eq!(program.sorts[0].name, "主体");
    assert_eq!(program.relations[0].name, "契約可能");
    assert_eq!(program.defns[0].name, "契約可能か");
}

#[test]
fn parser_normalizes_identifiers_with_nfc() {
    let decomposed = "\u{30AB}\u{3099}";
    let src = format!("(sort {decomposed})");
    let program = parse_program(&src).expect("parse should succeed");
    assert_eq!(program.sorts[0].name, "ガ");
}

#[test]
fn parser_keeps_import_path_atom_without_normalization() {
    let decomposed = "\u{30AB}\u{3099}.dtl";
    let src = format!("(import \"{decomposed}\")");
    let program = parse_program(&src).expect("parse should succeed");
    assert_eq!(program.imports[0].path, decomposed);
}

#[test]
fn parser_keeps_quoted_atom_without_nfc_normalization() {
    let decomposed_quoted = "\"\u{30AB}\u{3099}\"";
    let src = format!("(relation label (Symbol)) (fact label {decomposed_quoted})");
    let program = parse_program(&src).expect("parse should succeed");
    assert_eq!(
        program.facts[0].terms,
        vec![LogicTerm::Symbol(decomposed_quoted.to_string())]
    );
}

#[test]
fn parser_unescapes_quoted_atom_in_import_path() {
    let src = r#"(import "a\"b\nc\t.dtl")"#;
    let program = parse_program(src).expect("parse should succeed");
    assert_eq!(program.imports[0].path, "a\"b\nc\t.dtl");
}

#[test]
fn parser_accepts_whitespace_inside_quoted_atom() {
    let src = r#"(sort "A B")"#;
    let program = parse_program(src).expect("parse should succeed");
    assert_eq!(program.sorts[0].name, "\"A B\"");
}

#[test]
fn parser_rejects_unknown_escape_sequence_in_quoted_atom() {
    let src = r#"(import "bad\q.dtl")"#;
    let errors = parse_program(src).expect_err("parse should fail");
    assert!(errors.iter().any(|d| d.code == "E-PARSE"));
    assert!(
        errors
            .iter()
            .any(|d| d.message.contains("unsupported escape sequence"))
    );
}
