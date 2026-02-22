use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Bool,
    Int,
    Symbol,
    Domain(String),
    Adt(String),
    Fun(Vec<Type>, Box<Type>),
    Refine {
        var: String,
        base: Box<Type>,
        formula: Formula,
    },
}

impl Type {
    pub fn base(self) -> Type {
        match self {
            Type::Refine { base, .. } => *base,
            t => t,
        }
    }

    pub fn as_base(&self) -> &Type {
        match self {
            Type::Refine { base, .. } => base.as_ref(),
            t => t,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LogicTerm {
    Var(String),
    Symbol(String),
    Int(i64),
    Bool(bool),
    Ctor { name: String, args: Vec<LogicTerm> },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Atom {
    pub pred: String,
    pub terms: Vec<LogicTerm>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Formula {
    True,
    Atom(Atom),
    And(Vec<Formula>),
    Not(Box<Formula>),
}

impl Formula {
    pub fn atom(pred: impl Into<String>, terms: Vec<LogicTerm>) -> Self {
        Formula::Atom(Atom {
            pred: pred.into(),
            terms,
        })
    }
}

impl fmt::Display for LogicTerm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogicTerm::Var(v) => write!(f, "{v}"),
            LogicTerm::Symbol(s) => write!(f, "{s}"),
            LogicTerm::Int(i) => write!(f, "{i}"),
            LogicTerm::Bool(b) => write!(f, "{b}"),
            LogicTerm::Ctor { name, args } => {
                write!(f, "({name}")?;
                for arg in args {
                    write!(f, " {arg}")?;
                }
                write!(f, ")")
            }
        }
    }
}
