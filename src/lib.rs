pub mod ast;
pub mod diagnostics;
pub mod logic_engine;
pub mod name_resolve;
pub mod parser;
pub mod prover;
pub mod stratify;
pub mod typecheck;
pub mod types;

pub use ast::Program;
pub use diagnostics::{Diagnostic, Span};
pub use logic_engine::{DerivedFacts, GroundFact, KnowledgeBase, solve_facts};
pub use parser::{parse_program, parse_program_with_source};
pub use prover::{
    DOC_SPEC_SCHEMA_VERSION, DocBundleFormat, PROOF_TRACE_SCHEMA_VERSION, ProofTrace,
    generate_doc_bundle, has_failed_obligation, prove_program, write_proof_trace,
};
pub use typecheck::{TypeReport, check_program};
