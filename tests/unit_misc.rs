use dtl::ast::Program;
use dtl::diagnostics::{Diagnostic, line_col, make_span};
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
fn ast_program_default_and_new() {
    let p = Program::new();
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
