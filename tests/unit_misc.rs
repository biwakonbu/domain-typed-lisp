use dtl::ast::Program;
use dtl::diagnostics::{Diagnostic, line_col, make_span};
use dtl::parse_program_with_source;
use dtl::types::{Formula, LogicTerm, Type};

#[test]
fn diagnostics_line_col_and_display() {
    let src = "a\nbc\n";
    let (line, col) = line_col(src, 2);
    assert_eq!((line, col), (2, 1));

    let span = make_span(src, 2, 3);
    let d = Diagnostic::new("E-X", "msg", Some(span));
    assert!(d.to_string().contains("E-X: msg at 2:1"));
}

#[test]
fn diagnostics_line_col_counts_unicode_scalars() {
    let src = "aあb";
    let (line, col) = line_col(src, 4);
    assert_eq!((line, col), (1, 3));
}

#[test]
fn diagnostics_hint_is_attached_for_known_code() {
    let d = Diagnostic::new("E-TYPE", "msg", None);
    assert!(d.hint().is_some());
    assert!(d.to_string().contains("hint:"));
}

#[test]
fn diagnostics_can_hold_source_path() {
    let d = Diagnostic::new("E-IO", "msg", None).with_source("foo/bar.dtl");
    assert_eq!(d.source(), Some("foo/bar.dtl"));
    assert!(d.to_string().contains("foo/bar.dtl: E-IO: msg"));
}

#[test]
fn ast_program_default_and_new() {
    let p = Program::new();
    assert!(p.imports.is_empty());
    assert!(p.sorts.is_empty());
    let p2 = Program::default();
    assert!(p2.relations.is_empty());
}

#[test]
fn types_helpers_are_exercised() {
    let t = Type::Refine {
        var: "x".to_string(),
        base: Box::new(Type::Symbol),
        formula: Formula::atom("p", vec![LogicTerm::Var("x".to_string())]),
    };
    assert_eq!(t.as_base(), &Type::Symbol);
    assert_eq!(t.clone().base(), Type::Symbol);

    let term = LogicTerm::Int(42);
    assert_eq!(term.to_string(), "42");
}

#[test]
fn parse_program_with_source_sets_span_file_id() {
    let src = "(sort Subject)";
    let program = parse_program_with_source(src, "fixtures/schema.dtl").expect("parse");
    assert_eq!(
        program.sorts[0].span.file_id.as_deref(),
        Some("fixtures/schema.dtl")
    );
}
