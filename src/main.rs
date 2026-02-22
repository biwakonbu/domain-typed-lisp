use std::fs;
use std::path::{Path, PathBuf};
use std::{collections::HashSet, fmt::Write};

use clap::{Parser, Subcommand, ValueEnum};
use dtl::{
    Diagnostic, DocBundleFormat, Program, ProofTrace, Span, check_program, generate_doc_bundle,
    has_failed_obligation, parse_program_with_source, prove_program, write_proof_trace,
};
use serde::Serialize;

#[derive(Debug, Parser)]
#[command(name = "dtl")]
#[command(about = "Domain Typed Lisp checker/prover")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Check {
        #[arg(required = true, num_args = 1..)]
        files: Vec<PathBuf>,
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
    Prove {
        #[arg(required = true, num_args = 1..)]
        files: Vec<PathBuf>,
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Doc {
        #[arg(required = true, num_args = 1..)]
        files: Vec<PathBuf>,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, value_enum, default_value_t = DocFormat::Markdown)]
        format: DocFormat,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum DocFormat {
    Markdown,
    Json,
}

#[derive(Debug, Serialize)]
struct JsonResponse {
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    report: Option<JsonReport>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    diagnostics: Vec<JsonDiagnostic>,
}

#[derive(Debug, Serialize)]
struct JsonReport {
    functions_checked: usize,
    errors: usize,
}

#[derive(Debug, Serialize)]
struct JsonDiagnostic {
    code: &'static str,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    hint: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    span: Option<JsonSpan>,
}

#[derive(Debug, Serialize)]
struct JsonSpan {
    start: usize,
    end: usize,
    line: usize,
    column: usize,
}

#[derive(Debug, Serialize)]
struct ProveJsonResponse {
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    proof: Option<ProofTrace>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    diagnostics: Vec<JsonDiagnostic>,
}

fn main() {
    let cli = Cli::parse();
    let exit_code = match cli.command {
        Command::Check { files, format } => run_check(&files, format),
        Command::Prove { files, format, out } => run_prove(&files, format, out.as_deref()),
        Command::Doc { files, out, format } => run_doc(&files, &out, format),
    };
    std::process::exit(exit_code);
}

fn run_check(files: &[PathBuf], format: OutputFormat) -> i32 {
    let program = match load_program(files) {
        Ok(program) => program,
        Err(diags) => {
            emit_error(&diags, format);
            return 1;
        }
    };

    match check_program(&program) {
        Ok(report) => {
            emit_ok(report.functions_checked, report.errors, format);
            0
        }
        Err(diags) => {
            let diags = attach_source_if_missing(diags, files);
            emit_error(&diags, format);
            1
        }
    }
}

fn run_prove(files: &[PathBuf], format: OutputFormat, out: Option<&Path>) -> i32 {
    let program = match load_program(files) {
        Ok(program) => program,
        Err(diags) => {
            emit_error(&diags, format);
            return 1;
        }
    };

    let trace = match prove_program(&program) {
        Ok(trace) => trace,
        Err(diags) => {
            let diags = attach_source_if_missing(diags, files);
            match format {
                OutputFormat::Text => emit_error(&diags, OutputFormat::Text),
                OutputFormat::Json => emit_json(ProveJsonResponse {
                    status: "error",
                    proof: None,
                    diagnostics: diags.iter().map(as_json_diagnostic).collect(),
                }),
            }
            return 1;
        }
    };

    if let Some(out_dir) = out {
        if let Err(err) = fs::create_dir_all(out_dir) {
            let diag = Diagnostic::new(
                "E-IO",
                format!(
                    "failed to create output directory {}: {err}",
                    out_dir.display()
                ),
                None,
            );
            emit_error(&[diag], format);
            return 1;
        }
        let path = out_dir.join("proof-trace.json");
        if let Err(diag) = write_proof_trace(&path, &trace) {
            emit_error(&[diag], format);
            return 1;
        }
    }

    let failed = has_failed_obligation(&trace);
    match format {
        OutputFormat::Text => {
            if failed {
                eprintln!("proof failed");
                for obligation in &trace.obligations {
                    if obligation.result != "proved" {
                        eprintln!("- {}", obligation.id);
                    }
                }
            } else {
                println!("ok");
            }
        }
        OutputFormat::Json => {
            emit_json(ProveJsonResponse {
                status: if failed { "error" } else { "ok" },
                proof: Some(trace),
                diagnostics: Vec::new(),
            });
        }
    }

    if failed { 1 } else { 0 }
}

fn run_doc(files: &[PathBuf], out: &Path, format: DocFormat) -> i32 {
    let program = match load_program(files) {
        Ok(program) => program,
        Err(diags) => {
            for d in diags {
                eprintln!("{d}");
            }
            return 1;
        }
    };

    let trace = match prove_program(&program) {
        Ok(trace) => trace,
        Err(diags) => {
            for d in attach_source_if_missing(diags, files) {
                eprintln!("{d}");
            }
            return 1;
        }
    };

    if let Err(diags) = generate_doc_bundle(&program, &trace, out, as_doc_bundle_format(format)) {
        for d in diags {
            eprintln!("{d}");
        }
        return 1;
    }

    println!("ok");
    0
}

fn as_doc_bundle_format(format: DocFormat) -> DocBundleFormat {
    match format {
        DocFormat::Markdown => DocBundleFormat::Markdown,
        DocFormat::Json => DocBundleFormat::Json,
    }
}

fn load_program(files: &[PathBuf]) -> Result<Program, Vec<Diagnostic>> {
    let mut state = LoadState::new();
    for file in files {
        load_program_file(file, &mut state);
    }
    if state.errors.is_empty() {
        Ok(state.merged)
    } else {
        Err(state.errors)
    }
}

#[derive(Debug)]
struct LoadState {
    merged: Program,
    errors: Vec<Diagnostic>,
    loaded: HashSet<PathBuf>,
    stack: Vec<PathBuf>,
}

impl LoadState {
    fn new() -> Self {
        Self {
            merged: Program::new(),
            errors: Vec::new(),
            loaded: HashSet::new(),
            stack: Vec::new(),
        }
    }
}

fn load_program_file(file: &Path, state: &mut LoadState) {
    let normalized = normalize_path(file);
    if state.loaded.contains(&normalized) {
        return;
    }
    if state.stack.contains(&normalized) {
        state.errors.push(
            Diagnostic::new(
                "E-IMPORT",
                format!(
                    "import cycle detected: {}",
                    render_cycle(&state.stack, &normalized)
                ),
                None,
            )
            .with_source(file.display().to_string()),
        );
        return;
    }
    state.stack.push(normalized.clone());

    let src = match fs::read_to_string(file) {
        Ok(src) => src,
        Err(err) => {
            state.errors.push(
                Diagnostic::new("E-IO", format!("failed to read file: {err}"), None)
                    .with_source(file.display().to_string()),
            );
            state.stack.pop();
            return;
        }
    };

    let source = file.display().to_string();
    let program = match parse_program_with_source(&src, &source) {
        Ok(program) => program,
        Err(diags) => {
            state
                .errors
                .extend(diags.into_iter().map(|d| d.with_source(source.clone())));
            state.stack.pop();
            return;
        }
    };

    for import in &program.imports {
        let path = resolve_import_path(file, &import.path);
        let norm = normalize_path(&path);
        if state.stack.contains(&norm) {
            state.errors.push(
                Diagnostic::new(
                    "E-IMPORT",
                    format!(
                        "import cycle detected: {}",
                        render_cycle(&state.stack, &norm)
                    ),
                    Some(import.span.clone()),
                )
                .with_source(source.clone()),
            );
            continue;
        }
        load_program_file(&path, state);
    }

    merge_program(&mut state.merged, program);
    state.loaded.insert(normalized);
    state.stack.pop();
}

fn normalize_path(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn resolve_import_path(base: &Path, import_path: &str) -> PathBuf {
    let imported = PathBuf::from(import_path);
    if imported.is_absolute() {
        imported
    } else {
        base.parent().unwrap_or(Path::new(".")).join(imported)
    }
}

fn render_cycle(stack: &[PathBuf], target: &Path) -> String {
    let start_idx = stack.iter().position(|p| p == target).unwrap_or(0);
    let mut rendered = String::new();
    for (idx, path) in stack.iter().skip(start_idx).enumerate() {
        if idx > 0 {
            let _ = write!(rendered, " -> ");
        }
        let _ = write!(rendered, "{}", path.display());
    }
    if !rendered.is_empty() {
        let _ = write!(rendered, " -> ");
    }
    let _ = write!(rendered, "{}", target.display());
    rendered
}

fn merge_program(dst: &mut Program, src: Program) {
    dst.imports.extend(src.imports);
    dst.sorts.extend(src.sorts);
    dst.data_decls.extend(src.data_decls);
    dst.relations.extend(src.relations);
    dst.facts.extend(src.facts);
    dst.rules.extend(src.rules);
    dst.asserts.extend(src.asserts);
    dst.universes.extend(src.universes);
    dst.defns.extend(src.defns);
}

fn attach_source_if_missing(diags: Vec<Diagnostic>, files: &[PathBuf]) -> Vec<Diagnostic> {
    let single_source = if files.len() == 1 {
        Some(files[0].display().to_string())
    } else {
        None
    };

    diags
        .into_iter()
        .map(|d| {
            if d.source().is_none() {
                if let Some(file_id) = d.span.as_ref().and_then(|span| span.file_id.clone()) {
                    d.with_source(file_id)
                } else if let Some(source) = &single_source {
                    d.with_source(source.clone())
                } else {
                    d
                }
            } else {
                d
            }
        })
        .collect()
}

fn emit_ok(functions_checked: usize, errors: usize, format: OutputFormat) {
    match format {
        OutputFormat::Text => println!("ok"),
        OutputFormat::Json => emit_json(JsonResponse {
            status: "ok",
            report: Some(JsonReport {
                functions_checked,
                errors,
            }),
            diagnostics: Vec::new(),
        }),
    }
}

fn emit_error(diags: &[Diagnostic], format: OutputFormat) {
    match format {
        OutputFormat::Text => {
            for d in diags {
                eprintln!("{d}");
            }
        }
        OutputFormat::Json => {
            let diagnostics = diags.iter().map(as_json_diagnostic).collect::<Vec<_>>();
            emit_json(JsonResponse {
                status: "error",
                report: None,
                diagnostics,
            });
        }
    }
}

fn emit_json<T: Serialize>(output: T) {
    let rendered = serde_json::to_string(&output).expect("serialize JSON output");
    println!("{rendered}");
}

fn as_json_diagnostic(diag: &Diagnostic) -> JsonDiagnostic {
    JsonDiagnostic {
        code: diag.code,
        message: diag.message.clone(),
        source: diag.source().map(ToOwned::to_owned),
        hint: diag.hint(),
        span: diag.span.as_ref().map(as_json_span),
    }
}

fn as_json_span(span: &Span) -> JsonSpan {
    JsonSpan {
        start: span.start,
        end: span.end,
        line: span.line,
        column: span.column,
    }
}
