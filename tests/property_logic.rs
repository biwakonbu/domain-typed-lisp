use dtl::{KnowledgeBase, parse_program, solve_facts};
use proptest::prelude::*;

fn program_from_facts(facts: &[String]) -> String {
    let mut src =
        String::from("(sort X)\n(relation p (X))\n(relation q (X))\n(rule (q ?x) (p ?x))\n");
    for f in facts {
        src.push_str(&format!("(fact p {})\n", f));
    }
    src
}

proptest! {
    #[test]
    fn fixedpoint_is_idempotent(values in prop::collection::vec("[a-z]{1,3}", 1..20)) {
        let src = program_from_facts(&values);
        let p = parse_program(&src).expect("parse");
        let kb = KnowledgeBase::from_program(&p).expect("kb");
        let d1 = solve_facts(&kb).expect("solve1");
        let kb2 = kb.with_extra_facts(d1.all_facts());
        let d2 = solve_facts(&kb2).expect("solve2");

        prop_assert_eq!(d1.relation_facts("q"), d2.relation_facts("q"));
    }

    #[test]
    fn adding_facts_is_monotonic(a in prop::collection::vec("[a-z]{1,3}", 1..10), b in prop::collection::vec("[a-z]{1,3}", 1..10)) {
        let src_a = program_from_facts(&a);
        let mut ab = a.clone();
        ab.extend(b);
        let src_ab = program_from_facts(&ab);

        let p_a = parse_program(&src_a).expect("parse A");
        let p_ab = parse_program(&src_ab).expect("parse AB");

        let d_a = solve_facts(&KnowledgeBase::from_program(&p_a).expect("kb A")).expect("solve A");
        let d_ab = solve_facts(&KnowledgeBase::from_program(&p_ab).expect("kb AB")).expect("solve AB");

        let qa = d_a.relation_facts("q");
        let qab = d_ab.relation_facts("q");
        prop_assert!(qa.is_subset(&qab));
    }

    #[test]
    fn alpha_renaming_keeps_typecheck_result(name in "[a-z]{1,3}") {
        prop_assume!(name != "r");
        let src1 = "(sort Subject)\n(sort Resource)\n(sort Action)\n(relation can-access (Subject Resource Action))\n(defn f ((u Subject) (r Resource)) (Refine b Bool (can-access u r read)) (can-access u r read))".to_string();
        let src2 = format!(
            "(sort Subject)\n(sort Resource)\n(sort Action)\n(relation can-access (Subject Resource Action))\n(defn f (({} Subject) (r Resource)) (Refine b Bool (can-access {} r read)) (can-access {} r read))",
            name, name, name
        );

        let p1 = parse_program(&src1).expect("parse1");
        let p2 = parse_program(&src2).expect("parse2");

        prop_assert!(dtl::check_program(&p1).is_ok());
        prop_assert!(dtl::check_program(&p2).is_ok());
    }
}
