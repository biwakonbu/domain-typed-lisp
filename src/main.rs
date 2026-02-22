use std::fs;
use std::path::{Path, PathBuf};
use std::{collections::HashSet, fmt::Write};

use clap::{Parser, Subcommand, ValueEnum};
use dtl::{Diagnostic, Program, Span, check_program, parse_program};
use serde::Serialize;

#[derive(Debug, Parser)]
#[command(name = "dtl")]
#[command(about = "Domain Typed Lisp checker")]
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum OutputFormat {
    Text,
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

fn main() {
    let cli = Cli::parse();
    let exit_code = match cli.command {
        Command::Check { files, format } => run_check(&files, format),
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
            let diags = attach_single_source_if_missing(diags, files);
            emit_error(&diags, format);
            1
        }
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
    let program = match parse_program(&src) {
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
    dst.relations.extend(src.relations);
    dst.facts.extend(src.facts);
    dst.rules.extend(src.rules);
    dst.defns.extend(src.defns);
}

fn attach_single_source_if_missing(diags: Vec<Diagnostic>, files: &[PathBuf]) -> Vec<Diagnostic> {
    if files.len() != 1 {
        return diags;
    }
    let source = files[0].display().to_string();
    diags
        .into_iter()
        .map(|d| {
            if d.source().is_none() {
                d.with_source(source.clone())
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

fn emit_json(output: JsonResponse) {
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
