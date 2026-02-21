use dtl::parse_program;

#[test]
fn parser_accepts_complex_constructs() {
    let src = r#"
        (sort Subject)
        (relation p (Symbol))
        (relation q (Symbol))
        (rule (q ?x) true)

        (defn use-fn ((f (-> (Symbol) Bool)) (x Symbol))
          Bool
          (if (f x)
              (let ((y x)) y)
              x))

        (defn refined ((x Symbol))
          (Refine b Bool (and (q x) (not (p x))))
          (q x))
    "#;

    let program = parse_program(src).expect("parse should succeed");
    assert_eq!(program.rules.len(), 1);
    assert_eq!(program.defns.len(), 2);
}

#[test]
fn parser_rejects_bad_function_type_shape() {
    let src = "(defn f ((x (-> Symbol Bool))) Bool true)";
    let errs = parse_program(src).expect_err("parse should fail");
    assert!(
        errs.iter()
            .any(|d| d.message.contains("function arguments must be a list"))
    );
}

#[test]
fn parser_rejects_refinement_atom_literal() {
    let src = "(defn f ((x Symbol)) (Refine b Bool x) true)";
    let errs = parse_program(src).expect_err("parse should fail");
    assert!(errs.iter().any(|d| {
        d.message
            .contains("formula atom must be true or predicate call")
    }));
}
