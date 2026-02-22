use dtl::{KnowledgeBase, parse_program, solve_facts};

#[test]
fn logic_derives_expected_facts() {
    let src = r#"
        (sort Subject)
        (sort Role)
        (sort Resource)
        (data Action (read))
        (relation has-role (Subject Role))
        (relation resource-public (Resource))
        (relation can-access (Subject Resource Action))

        (fact has-role alice admin)
        (fact resource-public doc1)
        (rule (can-access ?u ?r (read))
              (and (has-role ?u admin)
                   (resource-public ?r)))
    "#;

    let program = parse_program(src).expect("parse should succeed");
    let kb = KnowledgeBase::from_program(&program).expect("kb should build");
    let derived = solve_facts(&kb).expect("solve should succeed");
    assert!(derived.relation_facts("can-access").contains(&vec![
        "alice".to_string(),
        "doc1".to_string(),
        "(read)".to_string()
    ]));
}

#[test]
fn logic_is_order_invariant_for_facts() {
    let src_a = r#"
        (sort X)
        (relation p (X))
        (relation q (X))
        (fact p a)
        (fact p b)
        (rule (q ?x) (p ?x))
    "#;

    let src_b = r#"
        (sort X)
        (relation p (X))
        (relation q (X))
        (fact p b)
        (fact p a)
        (rule (q ?x) (p ?x))
    "#;

    let p_a = parse_program(src_a).expect("parse A");
    let p_b = parse_program(src_b).expect("parse B");
    let d_a = solve_facts(&KnowledgeBase::from_program(&p_a).expect("kb A")).expect("solve A");
    let d_b = solve_facts(&KnowledgeBase::from_program(&p_b).expect("kb B")).expect("solve B");

    assert_eq!(d_a.relation_facts("q"), d_b.relation_facts("q"));
}

#[test]
fn logic_uses_closed_world_assumption() {
    let src = r#"
        (sort X)
        (relation p (X))
    "#;
    let program = parse_program(src).expect("parse should succeed");
    let kb = KnowledgeBase::from_program(&program).expect("kb should build");
    let derived = solve_facts(&kb).expect("solve should succeed");

    assert!(!derived.contains("p", &["unknown"]));
}
