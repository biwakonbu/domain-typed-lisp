use std::collections::HashSet;

use crate::ast::{
    AssertDecl, ConstructorDecl, DataDecl, Defn, Expr, Fact, ImportDecl, MatchArm, Param, Pattern,
    Program, RelationDecl, Rule, SortDecl, UniverseDecl,
};
use crate::diagnostics::{Diagnostic, make_span};
use crate::types::{Atom, Formula, LogicTerm, Type};

#[derive(Debug, Clone)]
struct Token {
    kind: TokenKind,
    start: usize,
    end: usize,
}

#[derive(Debug, Clone)]
enum TokenKind {
    LParen,
    RParen,
    Atom(String),
}

#[derive(Debug, Clone)]
enum SExpr {
    Atom(String, usize, usize),
    List(Vec<SExpr>, usize, usize),
}

impl SExpr {
    fn span_bounds(&self) -> (usize, usize) {
        match self {
            SExpr::Atom(_, s, e) | SExpr::List(_, s, e) => (*s, *e),
        }
    }

    fn as_atom(&self) -> Option<&str> {
        match self {
            SExpr::Atom(s, _, _) => Some(s),
            SExpr::List(_, _, _) => None,
        }
    }
}

pub fn parse_program(src: &str) -> Result<Program, Vec<Diagnostic>> {
    let tokens = lex(src)?;
    let sexprs = parse_sexprs(src, &tokens)?;
    let mut program = Program::new();
    let mut errors = Vec::new();

    for form in &sexprs {
        match parse_toplevel(src, form) {
            Ok(TopLevel::Import(i)) => program.imports.push(i),
            Ok(TopLevel::Sort(s)) => program.sorts.push(s),
            Ok(TopLevel::Data(d)) => program.data_decls.push(d),
            Ok(TopLevel::Relation(r)) => program.relations.push(r),
            Ok(TopLevel::Fact(f)) => program.facts.push(f),
            Ok(TopLevel::Rule(r)) => program.rules.push(r),
            Ok(TopLevel::Assert(a)) => program.asserts.push(a),
            Ok(TopLevel::Universe(u)) => program.universes.push(u),
            Ok(TopLevel::Defn(d)) => program.defns.push(d),
            Err(e) => errors.push(e),
        }
    }

    if errors.is_empty() {
        Ok(program)
    } else {
        Err(errors)
    }
}

fn lex(src: &str) -> Result<Vec<Token>, Vec<Diagnostic>> {
    let mut tokens = Vec::new();
    let bytes = src.as_bytes();
    let mut i = 0usize;

    while i < bytes.len() {
        let ch = bytes[i] as char;
        if ch.is_whitespace() {
            i += 1;
            continue;
        }
        if ch == ';' {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if ch == '(' {
            tokens.push(Token {
                kind: TokenKind::LParen,
                start: i,
                end: i + 1,
            });
            i += 1;
            continue;
        }
        if ch == ')' {
            tokens.push(Token {
                kind: TokenKind::RParen,
                start: i,
                end: i + 1,
            });
            i += 1;
            continue;
        }

        let start = i;
        while i < bytes.len() {
            let c = bytes[i] as char;
            if c.is_whitespace() || c == '(' || c == ')' || c == ';' {
                break;
            }
            i += 1;
        }
        let text = &src[start..i];
        tokens.push(Token {
            kind: TokenKind::Atom(text.to_string()),
            start,
            end: i,
        });
    }

    Ok(tokens)
}

fn parse_sexprs(src: &str, tokens: &[Token]) -> Result<Vec<SExpr>, Vec<Diagnostic>> {
    let mut idx = 0usize;
    let mut forms = Vec::new();
    let mut errors = Vec::new();

    while idx < tokens.len() {
        match parse_one(src, tokens, &mut idx) {
            Ok(form) => forms.push(form),
            Err(e) => {
                errors.push(e);
                break;
            }
        }
    }

    if errors.is_empty() {
        Ok(forms)
    } else {
        Err(errors)
    }
}

fn parse_one(src: &str, tokens: &[Token], idx: &mut usize) -> Result<SExpr, Diagnostic> {
    if *idx >= tokens.len() {
        return Err(Diagnostic::new(
            "E-PARSE",
            "unexpected EOF",
            Some(make_span(src, src.len(), src.len())),
        ));
    }

    let t = &tokens[*idx];
    match &t.kind {
        TokenKind::Atom(s) => {
            *idx += 1;
            Ok(SExpr::Atom(s.clone(), t.start, t.end))
        }
        TokenKind::RParen => Err(Diagnostic::new(
            "E-PARSE",
            "unexpected ')'",
            Some(make_span(src, t.start, t.end)),
        )),
        TokenKind::LParen => {
            let start = t.start;
            *idx += 1;
            let mut items = Vec::new();
            loop {
                if *idx >= tokens.len() {
                    return Err(Diagnostic::new(
                        "E-PARSE",
                        "unbalanced parentheses",
                        Some(make_span(src, start, start + 1)),
                    ));
                }
                let cur = &tokens[*idx];
                if matches!(cur.kind, TokenKind::RParen) {
                    let end = cur.end;
                    *idx += 1;
                    return Ok(SExpr::List(items, start, end));
                }
                let node = parse_one(src, tokens, idx)?;
                items.push(node);
            }
        }
    }
}

enum TopLevel {
    Import(ImportDecl),
    Sort(SortDecl),
    Data(DataDecl),
    Relation(RelationDecl),
    Fact(Fact),
    Rule(Rule),
    Assert(AssertDecl),
    Universe(UniverseDecl),
    Defn(Defn),
}

fn parse_toplevel(src: &str, form: &SExpr) -> Result<TopLevel, Diagnostic> {
    let (start, end) = form.span_bounds();
    let list = match form {
        SExpr::List(items, _, _) => items,
        SExpr::Atom(_, _, _) => {
            return Err(Diagnostic::new(
                "E-PARSE",
                "top-level form must be a list",
                Some(make_span(src, start, end)),
            ));
        }
    };
    if list.is_empty() {
        return Err(Diagnostic::new(
            "E-PARSE",
            "empty top-level form",
            Some(make_span(src, start, end)),
        ));
    }

    let head = list[0].as_atom().ok_or_else(|| {
        Diagnostic::new(
            "E-PARSE",
            "top-level head must be symbol",
            Some(make_span(src, start, end)),
        )
    })?;

    match head {
        "import" => parse_import(src, list),
        "sort" => parse_sort(src, list),
        "data" => parse_data(src, list),
        "relation" => parse_relation(src, list),
        "fact" => parse_fact(src, list),
        "rule" => parse_rule(src, list),
        "assert" => parse_assert(src, list),
        "universe" => parse_universe(src, list),
        "defn" => parse_defn(src, list),
        _ => Err(Diagnostic::new(
            "E-PARSE",
            format!("unknown top-level form: {head}"),
            Some(make_span(src, start, end)),
        )),
    }
}

fn parse_import(src: &str, list: &[SExpr]) -> Result<TopLevel, Diagnostic> {
    if list.len() != 2 {
        let (s, e) = list[0].span_bounds();
        return Err(Diagnostic::new(
            "E-PARSE",
            "import expects exactly 1 path argument",
            Some(make_span(src, s, e)),
        ));
    }

    let path = atom_required(src, &list[1], "import path")?;
    let path = path.trim_matches('"').to_string();
    let (s, e) = list[0].span_bounds();
    Ok(TopLevel::Import(ImportDecl {
        path,
        span: make_span(src, s, e),
    }))
}

fn parse_sort(src: &str, list: &[SExpr]) -> Result<TopLevel, Diagnostic> {
    if list.len() != 2 {
        let (s, e) = list[0].span_bounds();
        return Err(Diagnostic::new(
            "E-PARSE",
            "sort expects exactly 1 argument",
            Some(make_span(src, s, e)),
        ));
    }
    let name = atom_required(src, &list[1], "sort name")?;
    let (s, e) = list[0].span_bounds();
    Ok(TopLevel::Sort(SortDecl {
        name,
        span: make_span(src, s, e),
    }))
}

fn parse_data(src: &str, list: &[SExpr]) -> Result<TopLevel, Diagnostic> {
    if list.len() < 3 {
        let (s, e) = list[0].span_bounds();
        return Err(Diagnostic::new(
            "E-PARSE",
            "data expects type name and at least one constructor",
            Some(make_span(src, s, e)),
        ));
    }
    let name = atom_required(src, &list[1], "data name")?;

    let mut ctors = Vec::new();
    for node in list.iter().skip(2) {
        let ctor_list = match node {
            SExpr::List(items, _, _) => items,
            _ => {
                let (s, e) = node.span_bounds();
                return Err(Diagnostic::new(
                    "E-PARSE",
                    "constructor must be list: (Ctor fields...)",
                    Some(make_span(src, s, e)),
                ));
            }
        };
        if ctor_list.is_empty() {
            let (s, e) = node.span_bounds();
            return Err(Diagnostic::new(
                "E-PARSE",
                "constructor cannot be empty",
                Some(make_span(src, s, e)),
            ));
        }
        let ctor_name = atom_required(src, &ctor_list[0], "constructor name")?;
        let mut fields = Vec::new();
        for field in ctor_list.iter().skip(1) {
            fields.push(parse_type(src, field, &HashSet::new())?);
        }
        let (s, e) = node.span_bounds();
        ctors.push(ConstructorDecl {
            name: ctor_name,
            fields,
            span: make_span(src, s, e),
        });
    }

    let (s, e) = list[0].span_bounds();
    Ok(TopLevel::Data(DataDecl {
        name,
        constructors: ctors,
        span: make_span(src, s, e),
    }))
}

fn parse_relation(src: &str, list: &[SExpr]) -> Result<TopLevel, Diagnostic> {
    if list.len() != 3 {
        let (s, e) = list[0].span_bounds();
        return Err(Diagnostic::new(
            "E-PARSE",
            "relation expects name and sort list",
            Some(make_span(src, s, e)),
        ));
    }
    let name = atom_required(src, &list[1], "relation name")?;
    let sort_list = match &list[2] {
        SExpr::List(items, _, _) => items,
        node => {
            let (s, e) = node.span_bounds();
            return Err(Diagnostic::new(
                "E-PARSE",
                "relation argument sorts must be a list",
                Some(make_span(src, s, e)),
            ));
        }
    };
    let mut arg_sorts = Vec::new();
    for item in sort_list {
        arg_sorts.push(atom_required(src, item, "sort name")?);
    }
    let (s, e) = list[0].span_bounds();
    Ok(TopLevel::Relation(RelationDecl {
        name,
        arg_sorts,
        span: make_span(src, s, e),
    }))
}

fn parse_fact(src: &str, list: &[SExpr]) -> Result<TopLevel, Diagnostic> {
    if list.len() < 2 {
        let (s, e) = list[0].span_bounds();
        return Err(Diagnostic::new(
            "E-PARSE",
            "fact expects predicate and terms",
            Some(make_span(src, s, e)),
        ));
    }
    let name = atom_required(src, &list[1], "fact predicate")?;
    let mut terms = Vec::new();
    for item in list.iter().skip(2) {
        let term = parse_const_term(src, item)?;
        terms.push(term);
    }
    let (s, e) = list[0].span_bounds();
    Ok(TopLevel::Fact(Fact {
        name,
        terms,
        span: make_span(src, s, e),
    }))
}

fn parse_rule(src: &str, list: &[SExpr]) -> Result<TopLevel, Diagnostic> {
    if list.len() != 3 {
        let (s, e) = list[0].span_bounds();
        return Err(Diagnostic::new(
            "E-PARSE",
            "rule expects head and body",
            Some(make_span(src, s, e)),
        ));
    }
    let head = parse_rule_atom(src, &list[1])?;
    let body = parse_rule_formula(src, &list[2])?;
    let (s, e) = list[0].span_bounds();
    Ok(TopLevel::Rule(Rule {
        head,
        body,
        span: make_span(src, s, e),
    }))
}

fn parse_assert(src: &str, list: &[SExpr]) -> Result<TopLevel, Diagnostic> {
    if list.len() != 4 {
        let (s, e) = list[0].span_bounds();
        return Err(Diagnostic::new(
            "E-PARSE",
            "assert expects name, params and formula",
            Some(make_span(src, s, e)),
        ));
    }
    let name = atom_required(src, &list[1], "assert name")?;
    let params_list = match &list[2] {
        SExpr::List(items, _, _) => items,
        node => {
            let (s, e) = node.span_bounds();
            return Err(Diagnostic::new(
                "E-PARSE",
                "assert params must be a list",
                Some(make_span(src, s, e)),
            ));
        }
    };

    let mut params = Vec::new();
    let mut scope = HashSet::new();
    for p in params_list {
        let item = match p {
            SExpr::List(items, _, _) => items,
            node => {
                let (s, e) = node.span_bounds();
                return Err(Diagnostic::new(
                    "E-PARSE",
                    "assert parameter must be (name type)",
                    Some(make_span(src, s, e)),
                ));
            }
        };
        if item.len() != 2 {
            let (s, e) = p.span_bounds();
            return Err(Diagnostic::new(
                "E-PARSE",
                "assert parameter must contain exactly name and type",
                Some(make_span(src, s, e)),
            ));
        }
        let pname = atom_required(src, &item[0], "assert parameter name")?;
        let ty = parse_type(src, &item[1], &HashSet::new())?;
        let (s, e) = p.span_bounds();
        params.push(Param {
            name: pname.clone(),
            ty,
            span: make_span(src, s, e),
        });
        scope.insert(pname);
    }

    let formula = parse_refine_formula(src, &list[3], &scope)?;
    let (s, e) = list[0].span_bounds();
    Ok(TopLevel::Assert(AssertDecl {
        name,
        params,
        formula,
        span: make_span(src, s, e),
    }))
}

fn parse_universe(src: &str, list: &[SExpr]) -> Result<TopLevel, Diagnostic> {
    if list.len() != 3 {
        let (s, e) = list[0].span_bounds();
        return Err(Diagnostic::new(
            "E-PARSE",
            "universe expects type name and value list",
            Some(make_span(src, s, e)),
        ));
    }
    let ty_name = atom_required(src, &list[1], "universe type")?;
    let values_node = match &list[2] {
        SExpr::List(items, _, _) => items,
        node => {
            let (s, e) = node.span_bounds();
            return Err(Diagnostic::new(
                "E-PARSE",
                "universe values must be a list",
                Some(make_span(src, s, e)),
            ));
        }
    };

    let mut values = Vec::new();
    for node in values_node {
        values.push(parse_const_term(src, node)?);
    }

    let (s, e) = list[0].span_bounds();
    Ok(TopLevel::Universe(UniverseDecl {
        ty_name,
        values,
        span: make_span(src, s, e),
    }))
}

fn parse_defn(src: &str, list: &[SExpr]) -> Result<TopLevel, Diagnostic> {
    if list.len() != 5 {
        let (s, e) = list[0].span_bounds();
        return Err(Diagnostic::new(
            "E-PARSE",
            "defn expects name, params, return type and body",
            Some(make_span(src, s, e)),
        ));
    }

    let name = atom_required(src, &list[1], "function name")?;
    let params_list = match &list[2] {
        SExpr::List(items, _, _) => items,
        node => {
            let (s, e) = node.span_bounds();
            return Err(Diagnostic::new(
                "E-PARSE",
                "defn params must be a list",
                Some(make_span(src, s, e)),
            ));
        }
    };

    let mut params = Vec::new();
    let mut param_scope = HashSet::new();
    for p in params_list {
        let item = match p {
            SExpr::List(items, _, _) => items,
            node => {
                let (s, e) = node.span_bounds();
                return Err(Diagnostic::new(
                    "E-PARSE",
                    "parameter must be (name type)",
                    Some(make_span(src, s, e)),
                ));
            }
        };
        if item.len() != 2 {
            let (s, e) = p.span_bounds();
            return Err(Diagnostic::new(
                "E-PARSE",
                "parameter must contain exactly name and type",
                Some(make_span(src, s, e)),
            ));
        }

        let pname = atom_required(src, &item[0], "parameter name")?;
        let ty = parse_type(src, &item[1], &HashSet::new())?;
        let (s, e) = p.span_bounds();
        params.push(Param {
            name: pname.clone(),
            ty,
            span: make_span(src, s, e),
        });
        param_scope.insert(pname);
    }

    let ret_type = parse_type(src, &list[3], &param_scope)?;
    let body = parse_expr(src, &list[4], &param_scope)?;
    let (s, e) = list[0].span_bounds();
    Ok(TopLevel::Defn(Defn {
        name,
        params,
        ret_type,
        body,
        span: make_span(src, s, e),
    }))
}

fn parse_type(src: &str, node: &SExpr, scope: &HashSet<String>) -> Result<Type, Diagnostic> {
    if let Some(atom) = node.as_atom() {
        return Ok(match atom {
            "Bool" => Type::Bool,
            "Int" => Type::Int,
            "Symbol" => Type::Symbol,
            other => Type::Domain(other.to_string()),
        });
    }

    let list = match node {
        SExpr::List(items, _, _) => items,
        SExpr::Atom(_, _, _) => unreachable!(),
    };
    if list.is_empty() {
        let (s, e) = node.span_bounds();
        return Err(Diagnostic::new(
            "E-PARSE",
            "type cannot be empty list",
            Some(make_span(src, s, e)),
        ));
    }

    let head = atom_required(src, &list[0], "type constructor")?;
    match head.as_str() {
        "Refine" => {
            if list.len() != 4 {
                let (s, e) = node.span_bounds();
                return Err(Diagnostic::new(
                    "E-PARSE",
                    "Refine expects (Refine var base formula)",
                    Some(make_span(src, s, e)),
                ));
            }
            let var = atom_required(src, &list[1], "refinement variable")?;
            let base = parse_type(src, &list[2], scope)?;
            let mut formula_scope = scope.clone();
            formula_scope.insert(var.clone());
            let formula = parse_refine_formula(src, &list[3], &formula_scope)?;
            Ok(Type::Refine {
                var,
                base: Box::new(base),
                formula,
            })
        }
        "->" => {
            if list.len() != 3 {
                let (s, e) = node.span_bounds();
                return Err(Diagnostic::new(
                    "E-PARSE",
                    "function type expects (-> (args...) ret)",
                    Some(make_span(src, s, e)),
                ));
            }
            let args = match &list[1] {
                SExpr::List(items, _, _) => items,
                n => {
                    let (s, e) = n.span_bounds();
                    return Err(Diagnostic::new(
                        "E-PARSE",
                        "function arguments must be a list",
                        Some(make_span(src, s, e)),
                    ));
                }
            };
            let mut parsed_args = Vec::new();
            for a in args {
                parsed_args.push(parse_type(src, a, scope)?);
            }
            let ret = parse_type(src, &list[2], scope)?;
            Ok(Type::Fun(parsed_args, Box::new(ret)))
        }
        "Adt" => {
            if list.len() != 2 {
                let (s, e) = node.span_bounds();
                return Err(Diagnostic::new(
                    "E-PARSE",
                    "Adt expects exactly one type name",
                    Some(make_span(src, s, e)),
                ));
            }
            let name = atom_required(src, &list[1], "ADT name")?;
            Ok(Type::Adt(name))
        }
        _ => {
            let (s, e) = node.span_bounds();
            Err(Diagnostic::new(
                "E-PARSE",
                format!("unknown type constructor: {head}"),
                Some(make_span(src, s, e)),
            ))
        }
    }
}

fn parse_rule_formula(src: &str, node: &SExpr) -> Result<Formula, Diagnostic> {
    if let Some(atom) = node.as_atom() {
        return if atom == "true" {
            Ok(Formula::True)
        } else {
            let (s, e) = node.span_bounds();
            Err(Diagnostic::new(
                "E-PARSE",
                "rule formula atom must be 'true' or a predicate application",
                Some(make_span(src, s, e)),
            ))
        };
    }

    let list = match node {
        SExpr::List(items, _, _) => items,
        SExpr::Atom(_, _, _) => unreachable!(),
    };
    if list.is_empty() {
        let (s, e) = node.span_bounds();
        return Err(Diagnostic::new(
            "E-PARSE",
            "empty formula",
            Some(make_span(src, s, e)),
        ));
    }
    let head = atom_required(src, &list[0], "formula head")?;
    match head.as_str() {
        "and" => {
            if list.len() < 2 {
                let (s, e) = node.span_bounds();
                return Err(Diagnostic::new(
                    "E-PARSE",
                    "and requires at least one operand",
                    Some(make_span(src, s, e)),
                ));
            }
            let mut items = Vec::new();
            for it in list.iter().skip(1) {
                items.push(parse_rule_formula(src, it)?);
            }
            Ok(Formula::And(items))
        }
        "not" => {
            if list.len() != 2 {
                let (s, e) = node.span_bounds();
                return Err(Diagnostic::new(
                    "E-PARSE",
                    "not requires exactly one operand",
                    Some(make_span(src, s, e)),
                ));
            }
            Ok(Formula::Not(Box::new(parse_rule_formula(src, &list[1])?)))
        }
        _ => parse_rule_atom(src, node).map(Formula::Atom),
    }
}

fn parse_refine_formula(
    src: &str,
    node: &SExpr,
    var_scope: &HashSet<String>,
) -> Result<Formula, Diagnostic> {
    if let Some(atom) = node.as_atom() {
        return if atom == "true" {
            Ok(Formula::True)
        } else {
            let (s, e) = node.span_bounds();
            Err(Diagnostic::new(
                "E-PARSE",
                "formula atom must be true or predicate call",
                Some(make_span(src, s, e)),
            ))
        };
    }

    let list = match node {
        SExpr::List(items, _, _) => items,
        SExpr::Atom(_, _, _) => unreachable!(),
    };
    if list.is_empty() {
        let (s, e) = node.span_bounds();
        return Err(Diagnostic::new(
            "E-PARSE",
            "empty formula",
            Some(make_span(src, s, e)),
        ));
    }

    let head = atom_required(src, &list[0], "formula head")?;
    match head.as_str() {
        "and" => {
            if list.len() < 2 {
                let (s, e) = node.span_bounds();
                return Err(Diagnostic::new(
                    "E-PARSE",
                    "and requires at least one operand",
                    Some(make_span(src, s, e)),
                ));
            }
            let mut items = Vec::new();
            for it in list.iter().skip(1) {
                items.push(parse_refine_formula(src, it, var_scope)?);
            }
            Ok(Formula::And(items))
        }
        "not" => {
            if list.len() != 2 {
                let (s, e) = node.span_bounds();
                return Err(Diagnostic::new(
                    "E-PARSE",
                    "not requires exactly one operand",
                    Some(make_span(src, s, e)),
                ));
            }
            Ok(Formula::Not(Box::new(parse_refine_formula(
                src, &list[1], var_scope,
            )?)))
        }
        _ => {
            let pred = head;
            let mut terms = Vec::new();
            for t in list.iter().skip(1) {
                terms.push(parse_formula_term(src, t, var_scope)?);
            }
            Ok(Formula::Atom(Atom { pred, terms }))
        }
    }
}

fn parse_rule_atom(src: &str, node: &SExpr) -> Result<Atom, Diagnostic> {
    let list = match node {
        SExpr::List(items, _, _) => items,
        SExpr::Atom(_, s, e) => {
            return Err(Diagnostic::new(
                "E-PARSE",
                "atom must be list form: (pred args...)",
                Some(make_span(src, *s, *e)),
            ));
        }
    };
    if list.is_empty() {
        let (s, e) = node.span_bounds();
        return Err(Diagnostic::new(
            "E-PARSE",
            "predicate list cannot be empty",
            Some(make_span(src, s, e)),
        ));
    }
    let pred = atom_required(src, &list[0], "predicate name")?;
    let mut terms = Vec::new();
    for t in list.iter().skip(1) {
        terms.push(parse_rule_term(src, t)?);
    }
    Ok(Atom { pred, terms })
}

fn parse_expr(src: &str, node: &SExpr, scope: &HashSet<String>) -> Result<Expr, Diagnostic> {
    if let Some(atom) = node.as_atom() {
        let (s, e) = node.span_bounds();
        return if atom == "true" || atom == "false" {
            Ok(Expr::Bool {
                value: atom == "true",
                span: make_span(src, s, e),
            })
        } else if let Ok(i) = atom.parse::<i64>() {
            Ok(Expr::Int {
                value: i,
                span: make_span(src, s, e),
            })
        } else if scope.contains(atom) {
            Ok(Expr::Var {
                name: atom.to_string(),
                span: make_span(src, s, e),
            })
        } else {
            Ok(Expr::Symbol {
                value: atom.to_string(),
                span: make_span(src, s, e),
            })
        };
    }

    let list = match node {
        SExpr::List(items, _, _) => items,
        SExpr::Atom(_, _, _) => unreachable!(),
    };
    if list.is_empty() {
        let (s, e) = node.span_bounds();
        return Err(Diagnostic::new(
            "E-PARSE",
            "expression list cannot be empty",
            Some(make_span(src, s, e)),
        ));
    }

    let head = atom_required(src, &list[0], "expression head")?;
    let (s, e) = node.span_bounds();

    match head.as_str() {
        "let" => {
            if list.len() != 3 {
                return Err(Diagnostic::new(
                    "E-PARSE",
                    "let expects bindings and body",
                    Some(make_span(src, s, e)),
                ));
            }

            let bindings_list = match &list[1] {
                SExpr::List(items, _, _) => items,
                n => {
                    let (bs, be) = n.span_bounds();
                    return Err(Diagnostic::new(
                        "E-PARSE",
                        "let bindings must be a list",
                        Some(make_span(src, bs, be)),
                    ));
                }
            };

            let mut local_scope = scope.clone();
            let mut bindings = Vec::new();
            for b in bindings_list {
                let pair = match b {
                    SExpr::List(items, _, _) => items,
                    n => {
                        let (bs, be) = n.span_bounds();
                        return Err(Diagnostic::new(
                            "E-PARSE",
                            "binding must be (name expr)",
                            Some(make_span(src, bs, be)),
                        ));
                    }
                };
                if pair.len() != 2 {
                    let (bs, be) = b.span_bounds();
                    return Err(Diagnostic::new(
                        "E-PARSE",
                        "binding must have exactly name and value",
                        Some(make_span(src, bs, be)),
                    ));
                }
                let bname = atom_required(src, &pair[0], "binding name")?;
                let bexpr = parse_expr(src, &pair[1], &local_scope)?;
                let (bs, be) = b.span_bounds();
                bindings.push((bname.clone(), bexpr, make_span(src, bs, be)));
                local_scope.insert(bname);
            }

            let body = parse_expr(src, &list[2], &local_scope)?;
            Ok(Expr::Let {
                bindings,
                body: Box::new(body),
                span: make_span(src, s, e),
            })
        }
        "if" => {
            if list.len() != 4 {
                return Err(Diagnostic::new(
                    "E-PARSE",
                    "if expects cond, then, else",
                    Some(make_span(src, s, e)),
                ));
            }
            let cond = parse_expr(src, &list[1], scope)?;
            let then_branch = parse_expr(src, &list[2], scope)?;
            let else_branch = parse_expr(src, &list[3], scope)?;
            Ok(Expr::If {
                cond: Box::new(cond),
                then_branch: Box::new(then_branch),
                else_branch: Box::new(else_branch),
                span: make_span(src, s, e),
            })
        }
        "match" => parse_match_expr(src, node, list, scope),
        _ => {
            let mut args = Vec::new();
            for a in list.iter().skip(1) {
                args.push(parse_expr(src, a, scope)?);
            }
            Ok(Expr::Call {
                name: head,
                args,
                span: make_span(src, s, e),
            })
        }
    }
}

fn parse_match_expr(
    src: &str,
    node: &SExpr,
    list: &[SExpr],
    scope: &HashSet<String>,
) -> Result<Expr, Diagnostic> {
    let (s, e) = node.span_bounds();
    if list.len() < 3 {
        return Err(Diagnostic::new(
            "E-PARSE",
            "match expects scrutinee and at least one arm",
            Some(make_span(src, s, e)),
        ));
    }

    let scrutinee = parse_expr(src, &list[1], scope)?;
    let mut arms = Vec::new();

    for arm_node in list.iter().skip(2) {
        let arm_items = match arm_node {
            SExpr::List(items, _, _) => items,
            _ => {
                let (as_, ae) = arm_node.span_bounds();
                return Err(Diagnostic::new(
                    "E-PARSE",
                    "match arm must be (pattern expr)",
                    Some(make_span(src, as_, ae)),
                ));
            }
        };
        if arm_items.len() != 2 {
            let (as_, ae) = arm_node.span_bounds();
            return Err(Diagnostic::new(
                "E-PARSE",
                "match arm must contain exactly pattern and expression",
                Some(make_span(src, as_, ae)),
            ));
        }

        let mut pattern_bindings = HashSet::new();
        let pattern = parse_pattern(src, &arm_items[0], &mut pattern_bindings)?;
        let mut local_scope = scope.clone();
        local_scope.extend(pattern_bindings);
        let body = parse_expr(src, &arm_items[1], &local_scope)?;
        let (as_, ae) = arm_node.span_bounds();
        arms.push(MatchArm {
            pattern,
            body,
            span: make_span(src, as_, ae),
        });
    }

    Ok(Expr::Match {
        scrutinee: Box::new(scrutinee),
        arms,
        span: make_span(src, s, e),
    })
}

fn parse_pattern(
    src: &str,
    node: &SExpr,
    bindings: &mut HashSet<String>,
) -> Result<Pattern, Diagnostic> {
    if let Some(atom) = node.as_atom() {
        let (s, e) = node.span_bounds();
        if atom == "_" {
            return Ok(Pattern::Wildcard {
                span: make_span(src, s, e),
            });
        }
        if atom == "true" || atom == "false" {
            return Ok(Pattern::Bool {
                value: atom == "true",
                span: make_span(src, s, e),
            });
        }
        if let Ok(i) = atom.parse::<i64>() {
            return Ok(Pattern::Int {
                value: i,
                span: make_span(src, s, e),
            });
        }
        bindings.insert(atom.to_string());
        return Ok(Pattern::Var {
            name: atom.to_string(),
            span: make_span(src, s, e),
        });
    }

    let list = match node {
        SExpr::List(items, _, _) => items,
        SExpr::Atom(_, _, _) => unreachable!(),
    };
    if list.is_empty() {
        let (s, e) = node.span_bounds();
        return Err(Diagnostic::new(
            "E-PARSE",
            "pattern list cannot be empty",
            Some(make_span(src, s, e)),
        ));
    }

    let ctor = atom_required(src, &list[0], "pattern constructor")?;
    let mut args = Vec::new();
    for child in list.iter().skip(1) {
        args.push(parse_pattern(src, child, bindings)?);
    }
    let (s, e) = node.span_bounds();
    Ok(Pattern::Ctor {
        name: ctor,
        args,
        span: make_span(src, s, e),
    })
}

fn parse_rule_term(src: &str, node: &SExpr) -> Result<LogicTerm, Diagnostic> {
    match node {
        SExpr::Atom(atom, s, e) => parse_rule_atom_term(src, atom, *s, *e),
        SExpr::List(items, s, e) => {
            if items.is_empty() {
                return Err(Diagnostic::new(
                    "E-PARSE",
                    "constructor term cannot be empty",
                    Some(make_span(src, *s, *e)),
                ));
            }
            let name = atom_required(src, &items[0], "constructor name")?;
            let mut args = Vec::new();
            for child in items.iter().skip(1) {
                args.push(parse_rule_term(src, child)?);
            }
            Ok(LogicTerm::Ctor { name, args })
        }
    }
}

fn parse_rule_atom_term(
    src: &str,
    atom: &str,
    start: usize,
    end: usize,
) -> Result<LogicTerm, Diagnostic> {
    if atom == "true" {
        return Ok(LogicTerm::Bool(true));
    }
    if atom == "false" {
        return Ok(LogicTerm::Bool(false));
    }
    if let Ok(i) = atom.parse::<i64>() {
        return Ok(LogicTerm::Int(i));
    }
    if let Some(rest) = atom.strip_prefix('?') {
        if rest.is_empty() {
            return Err(Diagnostic::new(
                "E-PARSE",
                "variable name cannot be empty",
                Some(make_span(src, start, end)),
            ));
        }
        return Ok(LogicTerm::Var(rest.to_string()));
    }
    Ok(LogicTerm::Symbol(atom.to_string()))
}

fn parse_formula_term(
    src: &str,
    node: &SExpr,
    scope: &HashSet<String>,
) -> Result<LogicTerm, Diagnostic> {
    match node {
        SExpr::Atom(atom, _, _) => {
            if atom == "true" {
                return Ok(LogicTerm::Bool(true));
            }
            if atom == "false" {
                return Ok(LogicTerm::Bool(false));
            }
            if let Ok(i) = atom.parse::<i64>() {
                return Ok(LogicTerm::Int(i));
            }
            if scope.contains(atom) {
                return Ok(LogicTerm::Var(atom.clone()));
            }
            Ok(LogicTerm::Symbol(atom.clone()))
        }
        SExpr::List(items, s, e) => {
            if items.is_empty() {
                return Err(Diagnostic::new(
                    "E-PARSE",
                    "constructor term cannot be empty",
                    Some(make_span(src, *s, *e)),
                ));
            }
            let name = atom_required(src, &items[0], "constructor name")?;
            let mut args = Vec::new();
            for child in items.iter().skip(1) {
                args.push(parse_formula_term(src, child, scope)?);
            }
            Ok(LogicTerm::Ctor { name, args })
        }
    }
}

fn parse_const_term(src: &str, node: &SExpr) -> Result<LogicTerm, Diagnostic> {
    match node {
        SExpr::Atom(atom, s, e) => {
            if atom == "true" {
                return Ok(LogicTerm::Bool(true));
            }
            if atom == "false" {
                return Ok(LogicTerm::Bool(false));
            }
            if let Ok(i) = atom.parse::<i64>() {
                return Ok(LogicTerm::Int(i));
            }
            if atom.starts_with('?') {
                return Err(Diagnostic::new(
                    "E-PARSE",
                    "fact/universe cannot contain rule variables",
                    Some(make_span(src, *s, *e)),
                ));
            }
            Ok(LogicTerm::Symbol(atom.clone()))
        }
        SExpr::List(items, s, e) => {
            if items.is_empty() {
                return Err(Diagnostic::new(
                    "E-PARSE",
                    "constructor term cannot be empty",
                    Some(make_span(src, *s, *e)),
                ));
            }
            let name = atom_required(src, &items[0], "constructor name")?;
            let mut args = Vec::new();
            for child in items.iter().skip(1) {
                args.push(parse_const_term(src, child)?);
            }
            Ok(LogicTerm::Ctor { name, args })
        }
    }
}

fn atom_required(src: &str, node: &SExpr, expected: &str) -> Result<String, Diagnostic> {
    match node {
        SExpr::Atom(s, _, _) => Ok(s.clone()),
        SExpr::List(_, s, e) => Err(Diagnostic::new(
            "E-PARSE",
            format!("expected atom for {expected}"),
            Some(make_span(src, *s, *e)),
        )),
    }
}
