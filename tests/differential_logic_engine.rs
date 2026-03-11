mod support;

use std::collections::{BTreeMap, BTreeSet};

use proptest::prelude::*;
use support::program_generators::logic_program_sources;
use support::reference_semantics::reference_solve_facts;
use support::{prepare_program_from_source, production_derived_fact_map, read_fixture};

fn reference_fact_map(program: &dtl::Program) -> BTreeMap<String, BTreeSet<Vec<String>>> {
    let derived = reference_solve_facts(program).expect("reference solve should succeed");
    program
        .relations
        .iter()
        .map(|rel| (rel.name.clone(), derived.relation_facts(&rel.name)))
        .collect()
}

fn assert_fact_maps_eq(
    label: &str,
    src: &str,
    production: &BTreeMap<String, BTreeSet<Vec<String>>>,
    reference: &BTreeMap<String, BTreeSet<Vec<String>>>,
) {
    assert_eq!(
        production, reference,
        "logic_engine/reference mismatch: {label}\nsource:\n{src}\nproduction:\n{production:#?}\nreference:\n{reference:#?}"
    );
}

#[test]
fn logic_engine_matches_reference_on_curated_fixtures() {
    for path in [
        "semantics/negative-stratified/basic.dtl",
        "semantics/negative-stratified/adt_passthrough.dtl",
    ] {
        let src = read_fixture(path);
        let program = prepare_program_from_source(&src);
        let production = production_derived_fact_map(&program);
        let reference = reference_fact_map(&program);
        assert_fact_maps_eq(path, &src, &production, &reference);
    }
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 64,
        .. ProptestConfig::default()
    })]

    #[test]
    fn logic_engine_matches_reference_on_generated_programs(src in logic_program_sources()) {
        let program = prepare_program_from_source(&src);
        let production = production_derived_fact_map(&program);
        let reference = reference_fact_map(&program);
        eprintln!("generated logic source:\n{src}");
        prop_assert_eq!(production, reference);
    }
}
