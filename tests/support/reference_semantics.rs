#![allow(dead_code)]
#![allow(unused_imports)]

pub use dtl::{
    ReferenceDerivedFacts, ReferenceEnv, ReferenceObligationResult, ReferenceValue,
    reference_value_to_string,
};

use dtl::ast::{AssertDecl, Defn};
use dtl::{
    Program, reference_prove_program as dtl_reference_prove_program,
    reference_solve_facts as dtl_reference_solve_facts,
};

fn render_diagnostics(diags: Vec<dtl::Diagnostic>) -> String {
    diags
        .into_iter()
        .map(|diag| diag.message)
        .collect::<Vec<_>>()
        .join("; ")
}

pub fn reference_solve_facts_wrapper(program: &Program) -> Result<ReferenceDerivedFacts, String> {
    dtl_reference_solve_facts(program).map_err(render_diagnostics)
}

pub fn reference_prove_program_wrapper(
    program: &Program,
) -> Result<Vec<ReferenceObligationResult>, String> {
    dtl_reference_prove_program(program).map_err(render_diagnostics)
}

pub fn reference_check_assert(
    program: &Program,
    assertion: &AssertDecl,
) -> Result<ReferenceObligationResult, String> {
    reference_prove_program_wrapper(program)?
        .into_iter()
        .find(|item| item.id == format!("assert::{}", assertion.name))
        .ok_or_else(|| format!("missing obligation for assert::{}", assertion.name))
}

pub fn reference_check_refine(
    program: &Program,
    defn: &Defn,
) -> Result<ReferenceObligationResult, String> {
    reference_prove_program_wrapper(program)?
        .into_iter()
        .find(|item| item.id == format!("defn::{}", defn.name))
        .ok_or_else(|| format!("missing obligation for defn::{}", defn.name))
}

pub use reference_prove_program_wrapper as reference_prove_program;
pub use reference_solve_facts_wrapper as reference_solve_facts;
