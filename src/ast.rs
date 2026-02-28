use crate::diagnostics::Span;
use crate::types::{Atom, Formula, LogicTerm, Type};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    pub imports: Vec<ImportDecl>,
    pub aliases: Vec<AliasDecl>,
    pub sorts: Vec<SortDecl>,
    pub data_decls: Vec<DataDecl>,
    pub relations: Vec<RelationDecl>,
    pub facts: Vec<Fact>,
    pub rules: Vec<Rule>,
    pub asserts: Vec<AssertDecl>,
    pub universes: Vec<UniverseDecl>,
    pub defns: Vec<Defn>,
}

impl Program {
    pub fn new() -> Self {
        Self {
            imports: Vec::new(),
            aliases: Vec::new(),
            sorts: Vec::new(),
            data_decls: Vec::new(),
            relations: Vec::new(),
            facts: Vec::new(),
            rules: Vec::new(),
            asserts: Vec::new(),
            universes: Vec::new(),
            defns: Vec::new(),
        }
    }
}

impl Default for Program {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportDecl {
    pub path: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AliasDecl {
    pub alias: String,
    pub canonical: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SortDecl {
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationDecl {
    pub name: String,
    pub arg_sorts: Vec<String>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataDecl {
    pub name: String,
    pub constructors: Vec<ConstructorDecl>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstructorDecl {
    pub name: String,
    pub fields: Vec<Type>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fact {
    pub name: String,
    pub terms: Vec<LogicTerm>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    pub head: Atom,
    pub body: Formula,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssertDecl {
    pub name: String,
    pub params: Vec<Param>,
    pub formula: Formula,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UniverseDecl {
    pub ty_name: String,
    pub values: Vec<LogicTerm>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    pub name: String,
    pub ty: Type,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Defn {
    pub name: String,
    pub params: Vec<Param>,
    pub ret_type: Type,
    pub body: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    Var {
        name: String,
        span: Span,
    },
    Symbol {
        value: String,
        span: Span,
    },
    Int {
        value: i64,
        span: Span,
    },
    Bool {
        value: bool,
        span: Span,
    },
    Call {
        name: String,
        args: Vec<Expr>,
        span: Span,
    },
    Let {
        bindings: Vec<(String, Expr, Span)>,
        body: Box<Expr>,
        span: Span,
    },
    If {
        cond: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Box<Expr>,
        span: Span,
    },
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<MatchArm>,
        span: Span,
    },
}

impl Expr {
    pub fn span(&self) -> &Span {
        match self {
            Expr::Var { span, .. }
            | Expr::Symbol { span, .. }
            | Expr::Int { span, .. }
            | Expr::Bool { span, .. }
            | Expr::Call { span, .. }
            | Expr::Let { span, .. }
            | Expr::If { span, .. }
            | Expr::Match { span, .. } => span,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pattern {
    Wildcard {
        span: Span,
    },
    Var {
        name: String,
        span: Span,
    },
    Symbol {
        value: String,
        span: Span,
    },
    Int {
        value: i64,
        span: Span,
    },
    Bool {
        value: bool,
        span: Span,
    },
    Ctor {
        name: String,
        args: Vec<Pattern>,
        span: Span,
    },
}

impl Pattern {
    pub fn span(&self) -> &Span {
        match self {
            Pattern::Wildcard { span }
            | Pattern::Var { span, .. }
            | Pattern::Symbol { span, .. }
            | Pattern::Int { span, .. }
            | Pattern::Bool { span, .. }
            | Pattern::Ctor { span, .. } => span,
        }
    }
}
