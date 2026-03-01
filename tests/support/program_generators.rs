#![allow(dead_code)]

use proptest::prelude::*;

const SUBJECTS: [&str; 3] = ["alice", "bob", "carol"];

fn render_subject_fact(pred: &str, enabled: [bool; 3]) -> String {
    SUBJECTS
        .iter()
        .zip(enabled)
        .filter(|(_, is_enabled)| *is_enabled)
        .map(|(subject, _)| format!("(fact {pred} ({subject}))\n"))
        .collect::<String>()
}

pub fn logic_program_sources() -> impl Strategy<Value = String> {
    (
        prop::array::uniform3(any::<bool>()),
        prop::array::uniform3(any::<bool>()),
    )
        .prop_map(|(p_enabled, blocked_enabled)| {
            let mut src = String::new();
            src.push_str("(sort X)\n");
            src.push_str("(relation p (X))\n");
            src.push_str("(relation blocked (X))\n");
            src.push_str("(relation q (X))\n");
            src.push_str(&render_sort_fact("p", p_enabled));
            src.push_str(&render_sort_fact("blocked", blocked_enabled));
            src.push_str("(rule (q ?x) (and (p ?x) (not (blocked ?x))))\n");
            src
        })
}

pub fn prove_program_sources() -> impl Strategy<Value = String> {
    (
        prop::array::uniform3(any::<bool>()),
        prop::array::uniform3(any::<bool>()),
        prop_oneof![
            Just("plain"),
            Just("let"),
            Just("if-same"),
            Just("match-same")
        ],
    )
        .prop_map(|(p_enabled, q_enabled, body_kind)| {
            let body = match body_kind {
                "plain" => "(q u)".to_string(),
                "let" => "(let ((ok (q u))) ok)".to_string(),
                "if-same" => "(if (p u) (q u) (q u))".to_string(),
                "match-same" => {
                    "(match u ((alice) (q u)) ((bob) (q u)) ((carol) (q u)))".to_string()
                }
                _ => unreachable!("unsupported body kind"),
            };

            let mut src = String::new();
            src.push_str("(data Subject (alice) (bob) (carol))\n");
            src.push_str("(relation p (Subject))\n");
            src.push_str("(relation q (Subject))\n");
            src.push_str(&render_subject_fact("p", p_enabled));
            src.push_str(&render_subject_fact("q", q_enabled));
            src.push_str("(universe Subject ((alice) (bob) (carol)))\n");
            src.push_str(
                "(assert q_implies_subject ((u Subject)) (not (and (q u) (not (p u)))))\n",
            );
            src.push_str("(defn witness ((u Subject))\n");
            src.push_str("  (Refine b Bool (q u))\n");
            src.push_str(&format!("  {body})\n"));
            src
        })
}

fn render_sort_fact(pred: &str, enabled: [bool; 3]) -> String {
    ["a", "b", "c"]
        .iter()
        .zip(enabled)
        .filter(|(_, is_enabled)| *is_enabled)
        .map(|(value, _)| format!("(fact {pred} {value})\n"))
        .collect::<String>()
}
