use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::Path;

use serde::Serialize;

use crate::ast::{Expr, Program};
use crate::diagnostics::{Diagnostic, Span};
use crate::logic_engine::{DerivedFacts, GroundFact, KnowledgeBase, Value, solve_facts};
use crate::name_resolve::resolve_program;
use crate::stratify::compute_strata;
use crate::typecheck::check_program;
use crate::types::{Atom, Formula, LogicTerm, Type};

pub const PROOF_TRACE_SCHEMA_VERSION: &str = "2.1.0";
pub const DOC_SPEC_SCHEMA_VERSION: &str = "2.0.0";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocBundleFormat {
    Markdown,
    Json,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProofTrace {
    pub schema_version: String,
    pub profile: String,
    pub summary: ProofSummary,
    pub claim_coverage: ClaimCoverage,
    pub obligations: Vec<ObligationTrace>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProofSummary {
    pub total: usize,
    pub proved: usize,
    pub failed: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClaimCoverage {
    pub total_claims: usize,
    pub proved_claims: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ObligationTrace {
    pub id: String,
    pub kind: String,
    pub result: String,
    pub valuation: Vec<NameValue>,
    pub premises: Vec<String>,
    pub derived: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub counterexample: Option<CounterexampleTrace>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NameValue {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CounterexampleTrace {
    pub valuation: Vec<NameValue>,
    pub premises: Vec<String>,
    pub missing_goals: Vec<String>,
}

#[derive(Debug, Clone)]
struct ObligationSpec {
    id: String,
    kind: String,
    lhs: Formula,
    rhs: Formula,
    vars: Vec<QuantifiedVarSpec>,
}

#[derive(Debug, Clone)]
struct QuantifiedVarSpec {
    name: String,
    ty: Type,
    span: Span,
}

#[derive(Debug, Serialize)]
struct JsonSpec {
    schema_version: String,
    profile: String,
    summary: ProofSummary,
    self_description: DocSelfDescription,
    sorts: Vec<JsonSpecSort>,
    data_declarations: Vec<JsonSpecDataDecl>,
    relations: Vec<JsonSpecRelation>,
    assertions: Vec<JsonSpecAssertion>,
    proof_status: Vec<JsonSpecProofStatus>,
}

#[derive(Debug, Serialize)]
struct JsonSpecSort {
    name: String,
}

#[derive(Debug, Serialize)]
struct JsonSpecDataDecl {
    name: String,
    constructors: Vec<JsonSpecConstructor>,
}

#[derive(Debug, Serialize)]
struct JsonSpecConstructor {
    name: String,
    fields: Vec<String>,
}

#[derive(Debug, Serialize)]
struct JsonSpecRelation {
    name: String,
    arg_sorts: Vec<String>,
}

#[derive(Debug, Serialize)]
struct JsonSpecAssertion {
    name: String,
}

#[derive(Debug, Serialize)]
struct JsonSpecProofStatus {
    id: String,
    kind: String,
    result: String,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct DocSelfDescription {
    pub project: Option<DocProject>,
    pub modules: Vec<DocModule>,
    pub references: Vec<DocReference>,
    pub contracts: Vec<DocContract>,
    pub quality_gates: Vec<DocQualityGate>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DocProject {
    pub name: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DocModule {
    pub name: String,
    pub path: String,
    pub category: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DocReference {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DocContract {
    pub name: String,
    pub source: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DocQualityGate {
    pub name: String,
    pub command: String,
    pub source: String,
    pub required: bool,
}

#[derive(Debug, Clone, Default)]
pub struct DocBundleOptions {
    pub profile: Option<String>,
    pub self_description: Option<DocSelfDescription>,
    pub intermediate_dsl: Option<String>,
}

pub fn prove_program(program: &Program) -> Result<ProofTrace, Vec<Diagnostic>> {
    let mut errors = resolve_program(program);
    if !errors.is_empty() {
        return Err(errors);
    }
    if let Err(mut e) = compute_strata(program) {
        errors.append(&mut e);
        return Err(errors);
    }
    if let Err(mut e) = check_program(program) {
        errors.append(&mut e);
        return Err(errors);
    }

    let kb = KnowledgeBase::from_program(program)?;
    let universe_map = build_universe_map(program)?;
    let obligations = build_obligations(program);

    let mut traces = Vec::new();
    for obligation in obligations {
        let valuations = enumerate_valuations(&obligation.vars, &universe_map)?;

        let mut failed = None;
        for valuation in valuations {
            let lhs = substitute_formula_values(&obligation.lhs, &valuation);
            let rhs = substitute_formula_values(&obligation.rhs, &valuation);

            let assumptions = positive_atoms(&lhs)
                .into_iter()
                .filter_map(atom_to_ground_fact)
                .collect::<Vec<_>>();
            let trial = kb.with_extra_facts(assumptions.clone());
            let derived = solve_facts(&trial).map_err(wrap_as_prove_error)?;

            if eval_formula(&lhs, &derived) && !eval_formula(&rhs, &derived) {
                let minimized = minimize_premises(&kb, &lhs, &rhs, &assumptions)?;
                let derived_for_min = solve_facts(&kb.with_extra_facts(minimized.clone()))
                    .map_err(wrap_as_prove_error)?;
                failed = Some((valuation, minimized, derived_for_min, rhs));
                break;
            }
        }

        if let Some((valuation, premises, derived, rhs)) = failed {
            traces.push(ObligationTrace {
                id: obligation.id,
                kind: obligation.kind,
                result: "failed".to_string(),
                valuation: render_valuation(&valuation),
                premises: render_premises(&premises),
                derived: render_derived(&derived),
                counterexample: Some(CounterexampleTrace {
                    valuation: render_valuation(&valuation),
                    premises: render_premises(&premises),
                    missing_goals: render_missing_goals(&rhs, &derived),
                }),
            });
        } else {
            traces.push(ObligationTrace {
                id: obligation.id,
                kind: obligation.kind,
                result: "proved".to_string(),
                valuation: Vec::new(),
                premises: Vec::new(),
                derived: Vec::new(),
                counterexample: None,
            });
        }
    }

    let proved = traces.iter().filter(|o| o.result == "proved").count();
    let total = traces.len();
    Ok(ProofTrace {
        schema_version: PROOF_TRACE_SCHEMA_VERSION.to_string(),
        profile: "standard".to_string(),
        summary: ProofSummary {
            total,
            proved,
            failed: total.saturating_sub(proved),
        },
        claim_coverage: ClaimCoverage {
            total_claims: total,
            proved_claims: proved,
        },
        obligations: traces,
    })
}

pub fn has_failed_obligation(trace: &ProofTrace) -> bool {
    trace.obligations.iter().any(|o| o.result != "proved")
}

pub fn has_full_claim_coverage(trace: &ProofTrace) -> bool {
    trace.claim_coverage.proved_claims == trace.claim_coverage.total_claims
}

pub fn write_proof_trace(path: &Path, trace: &ProofTrace) -> Result<(), Diagnostic> {
    let rendered = serde_json::to_string_pretty(trace).map_err(|e| {
        Diagnostic::new(
            "E-IO",
            format!("failed to serialize proof trace: {e}"),
            None,
        )
    })?;
    fs::write(path, rendered).map_err(|e| {
        Diagnostic::new(
            "E-IO",
            format!("failed to write {}: {e}", path.display()),
            None,
        )
    })
}

pub fn generate_doc_bundle(
    program: &Program,
    trace: &ProofTrace,
    out_dir: &Path,
    format: DocBundleFormat,
) -> Result<(), Vec<Diagnostic>> {
    generate_doc_bundle_with_options(program, trace, out_dir, format, DocBundleOptions::default())
}

pub fn generate_doc_bundle_with_options(
    program: &Program,
    trace: &ProofTrace,
    out_dir: &Path,
    format: DocBundleFormat,
    options: DocBundleOptions,
) -> Result<(), Vec<Diagnostic>> {
    if has_failed_obligation(trace) {
        return Err(vec![Diagnostic::new(
            "E-PROVE",
            "cannot generate documentation because there are unproved obligations",
            None,
        )]);
    }

    fs::create_dir_all(out_dir).map_err(|e| {
        vec![Diagnostic::new(
            "E-IO",
            format!(
                "failed to create output directory {}: {e}",
                out_dir.display()
            ),
            None,
        )]
    })?;

    let proof_path = out_dir.join("proof-trace.json");
    write_proof_trace(&proof_path, trace).map_err(|d| vec![d])?;

    let profile = options
        .profile
        .clone()
        .unwrap_or_else(|| trace.profile.clone());
    let self_description = options.self_description.unwrap_or_default();
    let (spec_filename, spec_content) =
        render_spec_content(program, trace, format, &profile, &self_description)?;
    let spec_path = out_dir.join(spec_filename);
    fs::write(&spec_path, spec_content).map_err(|e| {
        vec![Diagnostic::new(
            "E-IO",
            format!("failed to write {}: {e}", spec_path.display()),
            None,
        )]
    })?;

    let index = serde_json::json!({
        "schema_version": DOC_SPEC_SCHEMA_VERSION,
        "profile": profile,
        "files": [spec_filename, "proof-trace.json"],
        "status": "ok",
        "intermediate": {
            "dsl": options.intermediate_dsl
        }
    });
    let index_path = out_dir.join("doc-index.json");
    fs::write(
        &index_path,
        serde_json::to_string_pretty(&index).expect("serialize doc index"),
    )
    .map_err(|e| {
        vec![Diagnostic::new(
            "E-IO",
            format!("failed to write {}: {e}", index_path.display()),
            None,
        )]
    })?;

    Ok(())
}

fn render_spec_content(
    program: &Program,
    trace: &ProofTrace,
    format: DocBundleFormat,
    profile: &str,
    self_description: &DocSelfDescription,
) -> Result<(&'static str, String), Vec<Diagnostic>> {
    match format {
        DocBundleFormat::Markdown => Ok((
            "spec.md",
            render_spec_markdown(program, trace, profile, self_description),
        )),
        DocBundleFormat::Json => {
            let spec = render_spec_json(program, trace, profile, self_description.clone());
            let rendered = serde_json::to_string_pretty(&spec).map_err(|e| {
                vec![Diagnostic::new(
                    "E-IO",
                    format!("failed to serialize spec.json: {e}"),
                    None,
                )]
            })?;
            Ok(("spec.json", rendered))
        }
    }
}

fn render_spec_markdown(
    program: &Program,
    trace: &ProofTrace,
    profile: &str,
    self_description: &DocSelfDescription,
) -> String {
    let mut out = String::new();
    let proved = trace
        .obligations
        .iter()
        .filter(|o| o.result == "proved")
        .count();
    let failed = trace.obligations.len().saturating_sub(proved);

    out.push_str("# ドメイン仕様書\n\n");
    out.push_str("この文書は `dtl doc` により自動生成された検証済み仕様です。");
    out.push_str("記述内容はプログラム定義と証明結果を同期したものです。\n\n");

    out.push_str("## 概要\n");
    out.push_str(&format!(
        "- sort: {} 件 / data: {} 件 / relation: {} 件 / defn: {} 件 / assert: {} 件\n",
        program.sorts.len(),
        program.data_decls.len(),
        program.relations.len(),
        program.defns.len(),
        program.asserts.len()
    ));
    out.push_str(&format!("- profile: `{}`\n", profile));
    out.push_str(&format!(
        "- schema_version: proof=`{}` / doc=`{}`\n",
        PROOF_TRACE_SCHEMA_VERSION, DOC_SPEC_SCHEMA_VERSION
    ));
    out.push_str(&format!(
        "- 証明義務: {} 件（proved: {} / failed: {}）\n\n",
        trace.obligations.len(),
        proved,
        failed
    ));

    out.push_str("## 型定義\n");
    if program.sorts.is_empty() && program.data_decls.is_empty() {
        out.push_str("- 定義なし\n");
    } else {
        for sort in &program.sorts {
            out.push_str(&format!("- sort `{}`\n", sort.name));
        }
        for data in &program.data_decls {
            out.push_str(&format!("- data `{}`\n", data.name));
            for ctor in &data.constructors {
                let fields = ctor
                    .fields
                    .iter()
                    .map(type_to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                if fields.is_empty() {
                    out.push_str(&format!("  - `{}`\n", ctor.name));
                } else {
                    out.push_str(&format!("  - `{}`({})\n", ctor.name, fields));
                }
            }
        }
    }
    out.push('\n');

    out.push_str("## 関係と仕様\n");
    if program.relations.is_empty() {
        out.push_str("- relation 定義なし\n");
    } else {
        for rel in &program.relations {
            let args = rel.arg_sorts.join(", ");
            out.push_str(&format!("- relation `{}`({})\n", rel.name, args));
        }
    }
    if !program.asserts.is_empty() {
        out.push_str("- assert\n");
        for a in &program.asserts {
            out.push_str(&format!("  - `{}`\n", a.name));
        }
    }
    out.push('\n');

    out.push_str("## 証明結果\n");
    for o in &trace.obligations {
        out.push_str(&format!("- `{}`: `{}`\n", o.id, o.result));
    }
    out.push('\n');

    if let Some(project) = &self_description.project {
        out.push_str("## 自己記述プロジェクト\n");
        out.push_str(&format!("- 名前: `{}`\n", project.name));
        out.push_str(&format!("- 概要: {}\n\n", project.summary));
    }

    out.push_str("## Mermaid: 型・関係図\n\n");
    out.push_str("```mermaid\n");
    out.push_str("erDiagram\n");
    for (idx, sort) in program.sorts.iter().enumerate() {
        out.push_str(&format!(
            "  S{idx} {{\n    string name \"{}\"\n  }}\n",
            sort.name
        ));
    }
    for (idx, data) in program.data_decls.iter().enumerate() {
        out.push_str(&format!(
            "  D{idx} {{\n    string name \"{}\"\n  }}\n",
            data.name
        ));
    }
    for (idx, rel) in program.relations.iter().enumerate() {
        out.push_str(&format!(
            "  R{idx} {{\n    string signature \"{}\"\n  }}\n",
            rel.name
        ));
        for (arg_idx, arg) in rel.arg_sorts.iter().enumerate() {
            out.push_str(&format!("  R{idx} ||--|| A{idx}_{arg_idx} : \"{}\"\n", arg));
            out.push_str(&format!(
                "  A{idx}_{arg_idx} {{\n    string type \"{}\"\n  }}\n",
                arg
            ));
        }
    }
    out.push_str("```\n\n");

    out.push_str("## Mermaid: 依存グラフ\n\n");
    out.push_str("```mermaid\n");
    out.push_str("flowchart TD\n");

    let mut relation_ids = HashMap::new();
    for (idx, rel) in program.relations.iter().enumerate() {
        let id = format!("R{idx}");
        relation_ids.insert(rel.name.clone(), id.clone());
        out.push_str(&format!("  {id}[\"relation: {}\"]\n", rel.name));
    }
    let mut defn_ids = HashMap::new();
    for (idx, defn) in program.defns.iter().enumerate() {
        let id = format!("D{idx}");
        defn_ids.insert(defn.name.clone(), id.clone());
        out.push_str(&format!("  {id}[\"defn: {}\"]\n", defn.name));
    }
    let mut assert_ids = HashMap::new();
    for (idx, assertion) in program.asserts.iter().enumerate() {
        let id = format!("A{idx}");
        assert_ids.insert(assertion.name.clone(), id.clone());
        out.push_str(&format!("  {id}[\"assert: {}\"]\n", assertion.name));
    }

    for rule in &program.rules {
        if let Some(head_id) = relation_ids.get(&rule.head.pred) {
            let mut refs = HashSet::new();
            collect_formula_preds(&rule.body, &mut refs);
            for pred in refs {
                if let Some(from_id) = relation_ids.get(&pred) {
                    out.push_str(&format!("  {from_id} --> {head_id}\n"));
                }
            }
        }
    }

    for defn in &program.defns {
        let Some(defn_id) = defn_ids.get(&defn.name) else {
            continue;
        };
        let mut refs = HashSet::new();
        collect_expr_call_names(&defn.body, &mut refs);
        for name in refs {
            if let Some(rel_id) = relation_ids.get(&name) {
                out.push_str(&format!("  {defn_id} --> {rel_id}\n"));
            } else if let Some(callee_id) = defn_ids.get(&name) {
                out.push_str(&format!("  {defn_id} --> {callee_id}\n"));
            }
        }
    }

    for assertion in &program.asserts {
        let Some(assert_id) = assert_ids.get(&assertion.name) else {
            continue;
        };
        let mut refs = HashSet::new();
        collect_formula_preds(&assertion.formula, &mut refs);
        for pred in refs {
            if let Some(rel_id) = relation_ids.get(&pred) {
                out.push_str(&format!("  {assert_id} --> {rel_id}\n"));
            }
        }
    }
    out.push_str("```\n\n");

    out.push_str("## Mermaid: 証明要約\n\n");
    out.push_str("```mermaid\n");
    out.push_str("graph LR\n");
    out.push_str(&format!(
        "  TOTAL[\"obligations: {}\"]\n",
        trace.obligations.len()
    ));
    out.push_str(&format!("  OK[\"proved: {proved}\"]\n"));
    out.push_str(&format!("  NG[\"failed: {failed}\"]\n"));
    out.push_str("  TOTAL --> OK\n");
    out.push_str("  TOTAL --> NG\n");
    out.push_str("```\n");

    out
}

fn collect_formula_preds(formula: &Formula, out: &mut HashSet<String>) {
    match formula {
        Formula::True => {}
        Formula::Atom(atom) => {
            out.insert(atom.pred.clone());
        }
        Formula::And(items) => {
            for item in items {
                collect_formula_preds(item, out);
            }
        }
        Formula::Not(inner) => collect_formula_preds(inner, out),
    }
}

fn collect_expr_call_names(expr: &Expr, out: &mut HashSet<String>) {
    match expr {
        Expr::Var { .. } | Expr::Symbol { .. } | Expr::Int { .. } | Expr::Bool { .. } => {}
        Expr::Call { name, args, .. } => {
            out.insert(name.clone());
            for arg in args {
                collect_expr_call_names(arg, out);
            }
        }
        Expr::Let { bindings, body, .. } => {
            for (_, bexpr, _) in bindings {
                collect_expr_call_names(bexpr, out);
            }
            collect_expr_call_names(body, out);
        }
        Expr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            collect_expr_call_names(cond, out);
            collect_expr_call_names(then_branch, out);
            collect_expr_call_names(else_branch, out);
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            collect_expr_call_names(scrutinee, out);
            for arm in arms {
                collect_expr_call_names(&arm.body, out);
            }
        }
    }
}

fn render_spec_json(
    program: &Program,
    trace: &ProofTrace,
    profile: &str,
    self_description: DocSelfDescription,
) -> JsonSpec {
    JsonSpec {
        schema_version: DOC_SPEC_SCHEMA_VERSION.to_string(),
        profile: profile.to_string(),
        summary: trace.summary.clone(),
        self_description,
        sorts: program
            .sorts
            .iter()
            .map(|sort| JsonSpecSort {
                name: sort.name.clone(),
            })
            .collect(),
        data_declarations: program
            .data_decls
            .iter()
            .map(|decl| JsonSpecDataDecl {
                name: decl.name.clone(),
                constructors: decl
                    .constructors
                    .iter()
                    .map(|ctor| JsonSpecConstructor {
                        name: ctor.name.clone(),
                        fields: ctor.fields.iter().map(type_to_string).collect(),
                    })
                    .collect(),
            })
            .collect(),
        relations: program
            .relations
            .iter()
            .map(|rel| JsonSpecRelation {
                name: rel.name.clone(),
                arg_sorts: rel.arg_sorts.clone(),
            })
            .collect(),
        assertions: program
            .asserts
            .iter()
            .map(|assertion| JsonSpecAssertion {
                name: assertion.name.clone(),
            })
            .collect(),
        proof_status: trace
            .obligations
            .iter()
            .map(|obligation| JsonSpecProofStatus {
                id: obligation.id.clone(),
                kind: obligation.kind.clone(),
                result: obligation.result.clone(),
            })
            .collect(),
    }
}

fn type_to_string(ty: &Type) -> String {
    match ty {
        Type::Bool => "Bool".to_string(),
        Type::Int => "Int".to_string(),
        Type::Symbol => "Symbol".to_string(),
        Type::Domain(s) => s.clone(),
        Type::Adt(s) => s.clone(),
        Type::Fun(args, ret) => format!(
            "(-> ({}) {})",
            args.iter()
                .map(type_to_string)
                .collect::<Vec<_>>()
                .join(" "),
            type_to_string(ret)
        ),
        Type::Refine { var, base, formula } => {
            format!(
                "(Refine {} {} {})",
                var,
                type_to_string(base),
                formula_to_string(formula)
            )
        }
    }
}

fn formula_to_string(formula: &Formula) -> String {
    match formula {
        Formula::True => "true".to_string(),
        Formula::Atom(atom) => {
            let args = atom
                .terms
                .iter()
                .map(logic_term_to_string)
                .collect::<Vec<_>>()
                .join(" ");
            format!("({} {})", atom.pred, args).trim_end().to_string()
        }
        Formula::And(items) => format!(
            "(and {})",
            items
                .iter()
                .map(formula_to_string)
                .collect::<Vec<_>>()
                .join(" ")
        ),
        Formula::Not(inner) => format!("(not {})", formula_to_string(inner)),
    }
}

fn build_universe_map(program: &Program) -> Result<HashMap<String, Vec<Value>>, Vec<Diagnostic>> {
    let mut map = HashMap::new();
    for u in &program.universes {
        let mut vals = Vec::new();
        for term in &u.values {
            let Some(v) = logic_term_to_const_value(term) else {
                return Err(vec![Diagnostic::new(
                    "E-PROVE",
                    format!("universe value contains variable: {}", u.ty_name),
                    Some(u.span.clone()),
                )]);
            };
            vals.push(v);
        }
        map.insert(u.ty_name.clone(), vals);
    }
    Ok(map)
}

fn build_obligations(program: &Program) -> Vec<ObligationSpec> {
    let relation_names: HashSet<String> =
        program.relations.iter().map(|r| r.name.clone()).collect();
    let constructor_names: HashSet<String> = program
        .data_decls
        .iter()
        .flat_map(|d| d.constructors.iter().map(|c| c.name.clone()))
        .collect();

    let mut obligations = Vec::new();

    for defn in &program.defns {
        if let Type::Refine { formula, .. } = &defn.ret_type {
            let lhs = formula_from_expr(&defn.body, &relation_names, &constructor_names)
                .unwrap_or(Formula::True);
            let vars = defn
                .params
                .iter()
                .map(|p| QuantifiedVarSpec {
                    name: p.name.clone(),
                    ty: p.ty.clone(),
                    span: p.span.clone(),
                })
                .collect::<Vec<_>>();
            obligations.push(ObligationSpec {
                id: format!("defn::{}", defn.name),
                kind: "defn".to_string(),
                lhs,
                rhs: formula.clone(),
                vars,
            });
        }
    }

    for assertion in &program.asserts {
        obligations.push(ObligationSpec {
            id: format!("assert::{}", assertion.name),
            kind: "assert".to_string(),
            lhs: Formula::True,
            rhs: assertion.formula.clone(),
            vars: assertion
                .params
                .iter()
                .map(|p| QuantifiedVarSpec {
                    name: p.name.clone(),
                    ty: p.ty.clone(),
                    span: p.span.clone(),
                })
                .collect(),
        });
    }

    obligations
}

fn formula_from_expr(
    expr: &Expr,
    relation_names: &HashSet<String>,
    constructor_names: &HashSet<String>,
) -> Option<Formula> {
    match expr {
        Expr::Bool { value, .. } => {
            if *value {
                Some(Formula::True)
            } else {
                Some(Formula::Not(Box::new(Formula::True)))
            }
        }
        Expr::Call { name, args, .. } if relation_names.contains(name) => {
            let mut terms = Vec::new();
            for arg in args {
                terms.push(expr_to_logic_term(arg, constructor_names)?);
            }
            Some(Formula::Atom(Atom {
                pred: name.clone(),
                terms,
            }))
        }
        Expr::Let { bindings, body, .. } => {
            let mut subst = HashMap::new();
            for (name, bexpr, _) in bindings {
                let term = expr_to_logic_term(bexpr, constructor_names)?;
                subst.insert(name.clone(), term);
            }
            let base = formula_from_expr(body, relation_names, constructor_names)?;
            Some(substitute_formula_terms(&base, &subst))
        }
        Expr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            let then_formula = formula_from_expr(then_branch, relation_names, constructor_names)?;
            let else_formula = formula_from_expr(else_branch, relation_names, constructor_names)?;
            if let Some(cond_formula) = formula_from_expr(cond, relation_names, constructor_names) {
                Some(formula_or(
                    formula_and(vec![cond_formula.clone(), then_formula]),
                    formula_and(vec![formula_not(cond_formula), else_formula]),
                ))
            } else {
                Some(formula_or(then_formula, else_formula))
            }
        }
        Expr::Match { arms, .. } => {
            if arms.is_empty() {
                return Some(formula_false());
            }
            let mut acc = formula_false();
            for arm in arms {
                let branch = formula_from_expr(&arm.body, relation_names, constructor_names)?;
                acc = formula_or(acc, branch);
            }
            Some(acc)
        }
        Expr::Var { .. } | Expr::Symbol { .. } | Expr::Int { .. } | Expr::Call { .. } => None,
    }
}

fn formula_false() -> Formula {
    Formula::Not(Box::new(Formula::True))
}

fn formula_not(formula: Formula) -> Formula {
    match formula {
        Formula::Not(inner) => *inner,
        other => Formula::Not(Box::new(other)),
    }
}

fn formula_and(items: Vec<Formula>) -> Formula {
    let mut flattened = Vec::new();
    for item in items {
        match item {
            Formula::True => {}
            Formula::And(parts) => flattened.extend(parts),
            other => flattened.push(other),
        }
    }
    match flattened.len() {
        0 => Formula::True,
        1 => flattened.into_iter().next().expect("single formula"),
        _ => Formula::And(flattened),
    }
}

fn formula_or(left: Formula, right: Formula) -> Formula {
    formula_not(formula_and(vec![formula_not(left), formula_not(right)]))
}

fn expr_to_logic_term(expr: &Expr, constructor_names: &HashSet<String>) -> Option<LogicTerm> {
    match expr {
        Expr::Var { name, .. } => Some(LogicTerm::Var(name.clone())),
        Expr::Symbol { value, .. } => Some(LogicTerm::Symbol(value.clone())),
        Expr::Int { value, .. } => Some(LogicTerm::Int(*value)),
        Expr::Bool { value, .. } => Some(LogicTerm::Bool(*value)),
        Expr::Call { name, args, .. } if constructor_names.contains(name) => {
            let mut parsed_args = Vec::new();
            for arg in args {
                parsed_args.push(expr_to_logic_term(arg, constructor_names)?);
            }
            Some(LogicTerm::Ctor {
                name: name.clone(),
                args: parsed_args,
            })
        }
        Expr::Call { .. } | Expr::Let { .. } | Expr::If { .. } | Expr::Match { .. } => None,
    }
}

fn substitute_formula_terms(formula: &Formula, subst: &HashMap<String, LogicTerm>) -> Formula {
    match formula {
        Formula::True => Formula::True,
        Formula::Atom(atom) => Formula::Atom(Atom {
            pred: atom.pred.clone(),
            terms: atom
                .terms
                .iter()
                .map(|t| substitute_term(t, subst))
                .collect(),
        }),
        Formula::And(items) => Formula::And(
            items
                .iter()
                .map(|item| substitute_formula_terms(item, subst))
                .collect(),
        ),
        Formula::Not(inner) => Formula::Not(Box::new(substitute_formula_terms(inner, subst))),
    }
}

fn substitute_term(term: &LogicTerm, subst: &HashMap<String, LogicTerm>) -> LogicTerm {
    match term {
        LogicTerm::Var(v) => subst
            .get(v)
            .cloned()
            .unwrap_or_else(|| LogicTerm::Var(v.clone())),
        LogicTerm::Ctor { name, args } => LogicTerm::Ctor {
            name: name.clone(),
            args: args.iter().map(|a| substitute_term(a, subst)).collect(),
        },
        other => other.clone(),
    }
}

fn enumerate_valuations(
    vars: &[QuantifiedVarSpec],
    universe_map: &HashMap<String, Vec<Value>>,
) -> Result<Vec<HashMap<String, Value>>, Vec<Diagnostic>> {
    let mut domains = Vec::new();
    for var in vars {
        let key = type_key(&var.ty)?;
        let Some(values) = universe_map.get(&key) else {
            return Err(vec![Diagnostic::new(
                "E-PROVE",
                format!("missing universe declaration for type: {key}"),
                Some(var.span.clone()),
            )]);
        };
        if values.is_empty() {
            return Err(vec![Diagnostic::new(
                "E-PROVE",
                format!("universe for type {key} must not be empty"),
                Some(var.span.clone()),
            )]);
        }
        domains.push((var.name.clone(), values.clone()));
    }

    let mut out = Vec::new();
    let mut current = HashMap::new();
    enumerate_cartesian(&domains, 0, &mut current, &mut out);
    Ok(out)
}

fn enumerate_cartesian(
    domains: &[(String, Vec<Value>)],
    idx: usize,
    current: &mut HashMap<String, Value>,
    out: &mut Vec<HashMap<String, Value>>,
) {
    if idx == domains.len() {
        out.push(current.clone());
        return;
    }

    let (name, values) = &domains[idx];
    for value in values {
        current.insert(name.clone(), value.clone());
        enumerate_cartesian(domains, idx + 1, current, out);
    }
}

fn type_key(ty: &Type) -> Result<String, Vec<Diagnostic>> {
    match ty {
        Type::Bool => Ok("Bool".to_string()),
        Type::Int => Ok("Int".to_string()),
        Type::Symbol => Ok("Symbol".to_string()),
        Type::Domain(name) | Type::Adt(name) => Ok(name.clone()),
        Type::Refine { base, .. } => type_key(base),
        Type::Fun(_, _) => Err(vec![Diagnostic::new(
            "E-PROVE",
            "function-typed quantified variables are not supported in prove",
            None,
        )]),
    }
}

fn substitute_formula_values(formula: &Formula, valuation: &HashMap<String, Value>) -> Formula {
    match formula {
        Formula::True => Formula::True,
        Formula::Atom(atom) => Formula::Atom(Atom {
            pred: atom.pred.clone(),
            terms: atom
                .terms
                .iter()
                .map(|t| substitute_term_value(t, valuation))
                .collect(),
        }),
        Formula::And(items) => Formula::And(
            items
                .iter()
                .map(|item| substitute_formula_values(item, valuation))
                .collect(),
        ),
        Formula::Not(inner) => Formula::Not(Box::new(substitute_formula_values(inner, valuation))),
    }
}

fn substitute_term_value(term: &LogicTerm, valuation: &HashMap<String, Value>) -> LogicTerm {
    match term {
        LogicTerm::Var(name) => valuation
            .get(name)
            .map(value_to_logic_term)
            .unwrap_or_else(|| LogicTerm::Var(name.clone())),
        LogicTerm::Ctor { name, args } => LogicTerm::Ctor {
            name: name.clone(),
            args: args
                .iter()
                .map(|arg| substitute_term_value(arg, valuation))
                .collect(),
        },
        other => other.clone(),
    }
}

fn value_to_logic_term(value: &Value) -> LogicTerm {
    match value {
        Value::Symbol(s) => LogicTerm::Symbol(s.clone()),
        Value::Int(i) => LogicTerm::Int(*i),
        Value::Bool(b) => LogicTerm::Bool(*b),
        Value::Adt { ctor, fields } => LogicTerm::Ctor {
            name: ctor.clone(),
            args: fields.iter().map(value_to_logic_term).collect(),
        },
    }
}

fn positive_atoms(formula: &Formula) -> Vec<Atom> {
    let mut out = Vec::new();
    collect_positive_atoms(formula, false, &mut out);
    out
}

fn collect_positive_atoms(formula: &Formula, neg: bool, out: &mut Vec<Atom>) {
    match formula {
        Formula::True => {}
        Formula::Atom(atom) => {
            if !neg {
                out.push(atom.clone());
            }
        }
        Formula::And(items) => {
            for item in items {
                collect_positive_atoms(item, neg, out);
            }
        }
        Formula::Not(inner) => collect_positive_atoms(inner, !neg, out),
    }
}

fn atom_to_ground_fact(atom: Atom) -> Option<GroundFact> {
    let mut terms = Vec::new();
    for term in atom.terms {
        terms.push(logic_term_to_const_value(&term)?);
    }
    Some(GroundFact {
        pred: atom.pred,
        terms,
    })
}

fn logic_term_to_const_value(term: &LogicTerm) -> Option<Value> {
    match term {
        LogicTerm::Var(_) => None,
        LogicTerm::Symbol(s) => Some(Value::Symbol(s.clone())),
        LogicTerm::Int(i) => Some(Value::Int(*i)),
        LogicTerm::Bool(b) => Some(Value::Bool(*b)),
        LogicTerm::Ctor { name, args } => {
            let mut fields = Vec::new();
            for arg in args {
                fields.push(logic_term_to_const_value(arg)?);
            }
            Some(Value::Adt {
                ctor: name.clone(),
                fields,
            })
        }
    }
}

fn eval_formula(formula: &Formula, derived: &DerivedFacts) -> bool {
    match formula {
        Formula::True => true,
        Formula::Atom(atom) => {
            let Some(tuple) = atom
                .terms
                .iter()
                .map(logic_term_to_const_value)
                .collect::<Option<Vec<_>>>()
            else {
                return false;
            };
            derived
                .facts
                .get(&atom.pred)
                .map(|set| set.contains(&tuple))
                .unwrap_or(false)
        }
        Formula::And(items) => items.iter().all(|item| eval_formula(item, derived)),
        Formula::Not(inner) => !eval_formula(inner, derived),
    }
}

fn minimize_premises(
    kb: &KnowledgeBase,
    lhs: &Formula,
    rhs: &Formula,
    assumptions: &[GroundFact],
) -> Result<Vec<GroundFact>, Vec<Diagnostic>> {
    let mut sorted = assumptions.to_vec();
    sorted.sort_by_key(ground_fact_key);

    for size in 0..=sorted.len() {
        let mut search = SubsetSearch {
            kb,
            lhs,
            rhs,
            sorted: &sorted,
            found: None,
        };
        search.search(size, 0, &mut Vec::new())?;
        if let Some(premises) = search.found {
            return Ok(premises);
        }
    }

    Ok(sorted)
}

struct SubsetSearch<'a> {
    kb: &'a KnowledgeBase,
    lhs: &'a Formula,
    rhs: &'a Formula,
    sorted: &'a [GroundFact],
    found: Option<Vec<GroundFact>>,
}

impl<'a> SubsetSearch<'a> {
    fn search(
        &mut self,
        target: usize,
        start: usize,
        picked: &mut Vec<usize>,
    ) -> Result<(), Vec<Diagnostic>> {
        if self.found.is_some() {
            return Ok(());
        }
        if picked.len() == target {
            let subset = picked
                .iter()
                .map(|i| self.sorted[*i].clone())
                .collect::<Vec<_>>();
            let derived = solve_facts(&self.kb.with_extra_facts(subset.clone()))
                .map_err(wrap_as_prove_error)?;
            if eval_formula(self.lhs, &derived) && !eval_formula(self.rhs, &derived) {
                self.found = Some(subset);
            }
            return Ok(());
        }

        for i in start..self.sorted.len() {
            picked.push(i);
            self.search(target, i + 1, picked)?;
            picked.pop();
            if self.found.is_some() {
                return Ok(());
            }
        }

        Ok(())
    }
}

fn ground_fact_key(f: &GroundFact) -> String {
    let args = f
        .terms
        .iter()
        .map(value_to_string)
        .collect::<Vec<_>>()
        .join(",");
    format!("{}({})", f.pred, args)
}

fn wrap_as_prove_error(diags: Vec<Diagnostic>) -> Vec<Diagnostic> {
    diags
        .into_iter()
        .map(|d| Diagnostic::new("E-PROVE", d.message, d.span))
        .collect()
}

fn render_valuation(valuation: &HashMap<String, Value>) -> Vec<NameValue> {
    let mut map = BTreeMap::new();
    for (k, v) in valuation {
        map.insert(k.clone(), value_to_string(v));
    }
    map.into_iter()
        .map(|(name, value)| NameValue { name, value })
        .collect()
}

fn render_premises(premises: &[GroundFact]) -> Vec<String> {
    let mut out = premises.iter().map(ground_fact_key).collect::<Vec<_>>();
    out.sort();
    out
}

fn render_derived(derived: &DerivedFacts) -> Vec<String> {
    let mut out = Vec::new();
    for (pred, tuples) in &derived.facts {
        for tuple in tuples {
            let args = tuple
                .iter()
                .map(value_to_string)
                .collect::<Vec<_>>()
                .join(",");
            out.push(format!("{}({})", pred, args));
        }
    }
    out.sort();
    out
}

fn render_missing_goals(rhs: &Formula, derived: &DerivedFacts) -> Vec<String> {
    let mut out = BTreeSet::new();
    collect_missing_goals(rhs, derived, &mut out);
    out.into_iter().collect()
}

fn collect_missing_goals(formula: &Formula, derived: &DerivedFacts, out: &mut BTreeSet<String>) {
    match formula {
        Formula::True => {}
        Formula::Atom(atom) => {
            let Some(tuple) = atom
                .terms
                .iter()
                .map(logic_term_to_const_value)
                .collect::<Option<Vec<_>>>()
            else {
                out.insert(format!("{}(non-ground)", atom.pred));
                return;
            };
            let exists = derived
                .facts
                .get(&atom.pred)
                .map(|set| set.contains(&tuple))
                .unwrap_or(false);
            if !exists {
                let args = tuple
                    .iter()
                    .map(value_to_string)
                    .collect::<Vec<_>>()
                    .join(",");
                out.insert(format!("{}({})", atom.pred, args));
            }
        }
        Formula::And(items) => {
            for item in items {
                collect_missing_goals(item, derived, out);
            }
        }
        Formula::Not(inner) => {
            if eval_formula(inner, derived) {
                out.insert(format!("not {}", formula_to_string(inner)));
            }
        }
    }
}

fn value_to_string(v: &Value) -> String {
    match v {
        Value::Symbol(s) => s.clone(),
        Value::Int(i) => i.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Adt { ctor, fields } => {
            let mut s = format!("({ctor}");
            for f in fields {
                s.push(' ');
                s.push_str(&value_to_string(f));
            }
            s.push(')');
            s
        }
    }
}

fn logic_term_to_string(t: &LogicTerm) -> String {
    match t {
        LogicTerm::Var(v) => v.clone(),
        LogicTerm::Symbol(s) => s.clone(),
        LogicTerm::Int(i) => i.to_string(),
        LogicTerm::Bool(b) => b.to_string(),
        LogicTerm::Ctor { name, args } => {
            let mut s = format!("({name}");
            for arg in args {
                s.push(' ');
                s.push_str(&logic_term_to_string(arg));
            }
            s.push(')');
            s
        }
    }
}
