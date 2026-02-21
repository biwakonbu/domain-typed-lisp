pub mod ast;
pub mod diagnostics;
pub mod logic_engine;
pub mod name_resolve;
pub mod parser;
pub mod stratify;
pub mod typecheck;
pub mod types;

pub use ast::Program;
pub use diagnostics::{Diagnostic, Span};
pub use logic_engine::{DerivedFacts, GroundFact, KnowledgeBase, solve_facts};
pub use parser::parse_program;
pub use typecheck::{TypeReport, check_program};
