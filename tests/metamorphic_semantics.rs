mod support;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;

use assert_cmd::cargo::cargo_bin_cmd;
use dtl::{FormatOptions, format_source};
use serde_json::Value;
use tempfile::tempdir;

use support::reference_semantics::{
    ReferenceObligationResult, reference_prove_program, reference_value_to_string,
};
use support::{
    prepare_program_from_source, production_derived_fact_map, production_trace, read_fixture,
    valuation_to_map,
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ComparableObligation {
    id: String,
    kind: String,
    result: String,
    valuation: BTreeMap<String, String>,
}

fn proof_signature(src: &str) -> Vec<ComparableObligation> {
    let program = prepare_program_from_source(src);
    let trace = production_trace(&program);
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

fn reference_signature(src: &str) -> Vec<ComparableObligation> {
    let program = prepare_program_from_source(src);
    let reference = reference_prove_program(&program).expect("reference prove should succeed");
    comparable_reference(&reference)
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

fn derived_signature(src: &str) -> BTreeMap<String, BTreeSet<Vec<String>>> {
    let program = prepare_program_from_source(src);
    production_derived_fact_map(&program)
}

fn assert_semantics_equivalent(label: &str, left: &str, right: &str) {
    assert_eq!(
        proof_signature(left),
        proof_signature(right),
        "production proof mismatch: {label}"
    );
    assert_eq!(
        reference_signature(left),
        reference_signature(right),
        "reference proof mismatch: {label}"
    );
    assert_eq!(
        derived_signature(left),
        derived_signature(right),
        "derived fact mismatch: {label}"
    );
}

#[test]
fn fact_order_does_not_change_semantics() {
    let left = r#"
        (sort X)
        (relation p (X))
        (relation q (X))
        (fact p a)
        (fact p b)
        (rule (q ?x) (p ?x))
    "#;
    let right = r#"
        (sort X)
        (relation p (X))
        (relation q (X))
        (fact p b)
        (fact p a)
        (rule (q ?x) (p ?x))
    "#;
    assert_semantics_equivalent("fact-order", left, right);
}

#[test]
fn rule_order_does_not_change_semantics() {
    let left = r#"
        (sort X)
        (relation seed (X))
        (relation mid (X))
        (relation out (X))
        (fact seed a)
        (universe X (a))
        (assert stable ((u X)) (not (and (out u) (not (seed u)))))
        (rule (mid ?x) (seed ?x))
        (rule (out ?x) (mid ?x))
    "#;
    let right = r#"
        (sort X)
        (relation seed (X))
        (relation mid (X))
        (relation out (X))
        (fact seed a)
        (universe X (a))
        (assert stable ((u X)) (not (and (out u) (not (seed u)))))
        (rule (out ?x) (mid ?x))
        (rule (mid ?x) (seed ?x))
    "#;
    assert_semantics_equivalent("rule-order", left, right);
}

#[test]
fn alias_and_canonical_names_have_same_semantics() {
    let left = read_fixture("semantics/alias-canonicalization/with_alias.dtl");
    let right = r#"
        (data Action
          (read)
          (write))
        (relation allowed (Action))
        (fact allowed (read))
        (universe Action ((read) (write)))
        (defn alias-check ((a Action))
          (Refine b Bool (allowed a))
          (allowed a))
        (assert canonical ((a Action))
          (not (and (allowed a)
                    (not (allowed a)))))
    "#;
    assert_semantics_equivalent("alias-canonicalization", &left, right);
}

#[test]
fn universe_order_does_not_change_semantics() {
    let left = read_fixture("semantics/match-pattern-sensitive/branch_sensitive.dtl");
    let right = left.replace(
        "(universe Subject ((alice) (bob)))",
        "(universe Subject ((bob) (alice)))",
    );
    assert_semantics_equivalent("universe-order", &left, &right);
}

#[test]
fn alpha_renaming_does_not_change_semantics() {
    let left = r#"
        (data Subject (alice) (bob))
        (relation allowed (Subject))
        (fact allowed (alice))
        (universe Subject ((alice) (bob)))
        (assert consistency ((u Subject))
          (not (and (allowed u)
                    (not (allowed u)))))
        (defn witness ((u Subject))
          (Refine b Bool (allowed u))
          (allowed u))
    "#;
    let right = r#"
        (data Subject (alice) (bob))
        (relation allowed (Subject))
        (fact allowed (alice))
        (universe Subject ((alice) (bob)))
        (assert consistency ((subject Subject))
          (not (and (allowed subject)
                    (not (allowed subject)))))
        (defn witness ((subject Subject))
          (Refine b Bool (allowed subject))
          (allowed subject))
    "#;
    assert_semantics_equivalent("alpha-renaming", left, right);
}

#[test]
fn formatter_preserves_prove_result() {
    let src = read_fixture("semantics/match-pattern-sensitive/branch_sensitive.dtl");
    let formatted = format_source(&src, FormatOptions::default()).expect("format should succeed");
    assert_semantics_equivalent("fmt-preserves-prove", &src, &formatted);
}

#[test]
fn import_split_and_single_file_have_same_proof_trace() {
    let dir = tempdir().expect("tempdir");
    let split = dir.path().join("split.dtl");
    let common = dir.path().join("common.dtl");
    let single = dir.path().join("single.dtl");

    fs::write(
        &common,
        r#"
        (data Subject (alice) (bob))
        (relation allowed (Subject))
        (fact allowed (alice))
        (universe Subject ((alice) (bob)))
        "#,
    )
    .expect("write common");
    fs::write(
        &split,
        r#"
        (import "common.dtl")
        (assert consistency ((u Subject))
          (not (and (allowed u)
                    (not (allowed u)))))
        (defn witness ((u Subject))
          (Refine b Bool (allowed u))
          (allowed u))
        "#,
    )
    .expect("write split");
    fs::write(
        &single,
        r#"
        (data Subject (alice) (bob))
        (relation allowed (Subject))
        (fact allowed (alice))
        (universe Subject ((alice) (bob)))
        (assert consistency ((u Subject))
          (not (and (allowed u)
                    (not (allowed u)))))
        (defn witness ((u Subject))
          (Refine b Bool (allowed u))
          (allowed u))
        "#,
    )
    .expect("write single");

    let split_json = run_cli_prove_json(&split);
    let single_json = run_cli_prove_json(&single);
    assert_eq!(split_json["proof"], single_json["proof"]);
}

fn run_cli_prove_json(path: &std::path::Path) -> Value {
    let mut cmd = cargo_bin_cmd!("dtl");
    let output = cmd
        .arg("prove")
        .arg(path)
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .get_output()
        .clone();
    assert!(output.stderr.is_empty(), "json mode should not emit stderr");
    serde_json::from_slice(&output.stdout).expect("valid json output")
}
