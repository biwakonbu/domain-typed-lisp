#![allow(dead_code)]

pub mod program_generators;
pub mod reference_semantics;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

use dtl::name_resolve::{normalize_program_aliases, resolve_program};
use dtl::prover::NameValue;
use dtl::stratify::compute_strata;
use dtl::{
    DerivedFacts, Program, ProofTrace, check_program, parse_program, prove_program, solve_facts,
};

pub fn fixture_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(relative)
}

pub fn read_fixture(relative: &str) -> String {
    fs::read_to_string(fixture_path(relative)).expect("fixture should be readable")
}

pub fn prepare_program_from_source(src: &str) -> Program {
    let program = parse_program(src).expect("source should parse");
    prepare_program(&program)
}

pub fn prepare_program(program: &Program) -> Program {
    let normalized =
        normalize_program_aliases(program).expect("alias normalization should succeed");
    let resolve_errors = resolve_program(&normalized);
    assert!(
        resolve_errors.is_empty(),
        "resolve should succeed: {resolve_errors:?}"
    );
    compute_strata(&normalized).expect("stratification should succeed");
    check_program(&normalized).expect("typecheck should succeed");
    normalized
}

pub fn production_derived_facts(program: &Program) -> DerivedFacts {
    let kb = dtl::KnowledgeBase::from_program(program).expect("knowledge base should build");
    solve_facts(&kb).expect("logic engine should solve")
}

pub fn production_derived_fact_map(program: &Program) -> BTreeMap<String, BTreeSet<Vec<String>>> {
    let derived = production_derived_facts(program);
    program
        .relations
        .iter()
        .map(|rel| (rel.name.clone(), derived.relation_facts(&rel.name)))
        .collect()
}

pub fn production_trace(program: &Program) -> ProofTrace {
    prove_program(program).expect("prove should succeed")
}

pub fn valuation_to_map(items: &[NameValue]) -> BTreeMap<String, String> {
    items
        .iter()
        .map(|item| (item.name.clone(), item.value.clone()))
        .collect()
}
