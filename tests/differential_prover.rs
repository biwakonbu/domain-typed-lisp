mod support;

use std::collections::{BTreeMap, BTreeSet};

use dtl::name_resolve::resolve_program;
use dtl::{check_program, parse_program, prove_program};
use proptest::prelude::*;
use support::program_generators::prove_program_sources;
use support::reference_semantics::{
    ReferenceObligationResult, reference_prove_program, reference_value_to_string,
};
use support::{prepare_program_from_source, production_trace, read_fixture, valuation_to_map};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ComparableObligation {
    id: String,
    kind: String,
    result: String,
    valuation: BTreeMap<String, String>,
}

fn comparable_production(trace: &dtl::ProofTrace) -> Vec<ComparableObligation> {
    let mut out = trace
        .obligations
        .iter()
        .map(|obligation| ComparableObligation {
            id: obligation.id.clone(),
            kind: obligation.kind.clone(),
            result: obligation.result.clone(),
            valuation: valuation_to_map(&obligation.valuation),
        })
        .collect::<Vec<_>>();
    out.sort();
    out
}

fn comparable_reference(results: &[ReferenceObligationResult]) -> Vec<ComparableObligation> {
    let mut out = results
        .iter()
        .map(|obligation| ComparableObligation {
            id: obligation.id.clone(),
            kind: obligation.kind.clone(),
            result: obligation.result.clone(),
            valuation: obligation
                .valuation
                .iter()
                .map(|(name, value)| (name.clone(), reference_value_to_string(value)))
                .collect(),
        })
        .collect::<Vec<_>>();
    out.sort();
    out
}

fn failed_missing_goals_production(trace: &dtl::ProofTrace) -> BTreeMap<String, BTreeSet<String>> {
    trace
        .obligations
        .iter()
        .filter_map(|obligation| {
            obligation.counterexample.as_ref().map(|counterexample| {
                (
                    obligation.id.clone(),
                    counterexample.missing_goals.iter().cloned().collect(),
                )
            })
        })
        .collect()
}

fn failed_missing_goals_reference(
    results: &[ReferenceObligationResult],
) -> BTreeMap<String, BTreeSet<String>> {
    results
        .iter()
        .filter(|obligation| obligation.result == "failed")
        .map(|obligation| (obligation.id.clone(), obligation.missing_goals.clone()))
        .collect()
}

#[test]
fn prover_matches_reference_on_curated_fixtures() {
    for path in [
        "semantics/if-condition-sensitive/failing_refine.dtl",
        "semantics/match-pattern-sensitive/branch_sensitive.dtl",
        "semantics/alias-canonicalization/with_alias.dtl",
        "semantics/recursive-defn/list_allows.dtl",
        "semantics/assert-counterexample/everyone_allowed.dtl",
    ] {
        let src = read_fixture(path);
        let program = prepare_program_from_source(&src);
        let production = production_trace(&program);
        let reference = reference_prove_program(&program).expect("reference prove should succeed");
        assert_eq!(
            comparable_production(&production),
            comparable_reference(&reference),
            "prove/reference mismatch: {path}\nsource:\n{src}\nproduction:\n{production:#?}\nreference:\n{reference:#?}"
        );
        assert_eq!(
            failed_missing_goals_production(&production),
            failed_missing_goals_reference(&reference),
            "missing goals mismatch: {path}\nsource:\n{src}\nproduction:\n{production:#?}\nreference:\n{reference:#?}"
        );
    }
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 64,
        .. ProptestConfig::default()
    })]

    #[test]
    fn prover_matches_reference_on_generated_programs(src in prove_program_sources()) {
        let program = prepare_program_from_source(&src);
        let production = production_trace(&program);
        let reference = reference_prove_program(&program).expect("reference prove should succeed");
        eprintln!("generated prove source:\n{src}");
        prop_assert_eq!(comparable_production(&production), comparable_reference(&reference));
        prop_assert_eq!(
            failed_missing_goals_production(&production),
            failed_missing_goals_reference(&reference)
        );
    }
}

#[test]
fn prove_reports_inclusion_minimal_premises() {
    let src = read_fixture("semantics/assert-counterexample/everyone_allowed.dtl");
    let program = prepare_program_from_source(&src);
    let trace = production_trace(&program);
    let failed = trace
        .obligations
        .iter()
        .find(|obligation| obligation.result == "failed")
        .expect("failed obligation");

    let valuation = valuation_to_map(
        &failed
            .counterexample
            .as_ref()
            .expect("counterexample")
            .valuation,
    );
    let premises = failed.premises.clone();
    for subset in proper_subsets(&premises) {
        let extra_facts = subset
            .iter()
            .map(|premise| premise_to_fact_form(premise))
            .collect::<Vec<_>>();
        assert!(
            !fails_with_same_counterexample(&src, &failed.id, &valuation, &extra_facts),
            "non-minimal premises detected for {}: {:?}",
            failed.id,
            subset
        );
    }
}

#[test]
fn prove_rejects_missing_universe_fixture() {
    let src = read_fixture("semantics/errors/missing_universe.dtl");
    let program = parse_program(&src).expect("parse should succeed");
    let errs = prove_program(&program).expect_err("prove should fail");
    assert!(errs.iter().any(|diag| diag.code == "E-PROVE"));
    assert!(
        errs.iter()
            .any(|diag| diag.message.contains("missing universe declaration"))
    );
}

#[test]
fn check_rejects_type_mismatch_fixture() {
    let src = read_fixture("semantics/errors/type_mismatch.dtl");
    let program = parse_program(&src).expect("parse should succeed");
    let errs = check_program(&program).expect_err("check should fail");
    assert!(errs.iter().any(|diag| diag.code == "E-TYPE"));
}

#[test]
fn resolve_rejects_unsafe_rule_fixture() {
    let src = read_fixture("semantics/errors/unsafe_rule.dtl");
    let program = parse_program(&src).expect("parse should succeed");
    let errs = resolve_program(&program);
    assert!(!errs.is_empty(), "resolve should reject unsafe rule");
    assert!(errs.iter().any(|diag| diag.message.contains("unsafe rule")));
}

fn proper_subsets(items: &[String]) -> Vec<Vec<String>> {
    let mut out = Vec::new();
    for mask in 0..(1usize << items.len()) {
        if mask.count_ones() as usize == items.len() {
            continue;
        }
        let mut subset = Vec::new();
        for (index, item) in items.iter().enumerate() {
            if (mask & (1usize << index)) != 0 {
                subset.push(item.clone());
            }
        }
        out.push(subset);
    }
    out
}

fn fails_with_same_counterexample(
    src: &str,
    obligation_id: &str,
    valuation: &BTreeMap<String, String>,
    extra_fact_forms: &[String],
) -> bool {
    let mut augmented = String::from(src);
    if !augmented.ends_with('\n') {
        augmented.push('\n');
    }
    for fact in extra_fact_forms {
        augmented.push_str(fact);
        augmented.push('\n');
    }

    let program = prepare_program_from_source(&augmented);
    let trace = production_trace(&program);
    let Some(obligation) = trace
        .obligations
        .iter()
        .find(|item| item.id == obligation_id)
    else {
        return false;
    };
    obligation.result == "failed" && valuation_to_map(&obligation.valuation) == *valuation
}

fn premise_to_fact_form(premise: &str) -> String {
    let open = premise
        .find('(')
        .expect("premise should contain predicate arguments");
    let pred = &premise[..open];
    let inner = &premise[open + 1..premise.len() - 1];
    let args = split_top_level_csv(inner);
    if args.is_empty() {
        format!("(fact {pred})")
    } else {
        format!("(fact {pred} {})", args.join(" "))
    }
}

fn split_top_level_csv(input: &str) -> Vec<String> {
    if input.is_empty() {
        return Vec::new();
    }

    let mut parts = Vec::new();
    let mut start = 0;
    let mut depth = 0i32;
    for (idx, ch) in input.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(input[start..idx].trim().to_string());
                start = idx + 1;
            }
            _ => {}
        }
    }
    parts.push(input[start..].trim().to_string());
    parts
}
