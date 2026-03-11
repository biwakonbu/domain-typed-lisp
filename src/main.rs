use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;
use std::{collections::HashSet, fmt::Write};

use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use dtl::{
    Diagnostic, DocBundleFormat, DocBundleOptions, FormatOptions, LintDiagnostic, LintOptions,
    Program, ProofTrace, Span, check_program, format_source, generate_doc_bundle_with_options,
    has_failed_obligation, has_full_claim_coverage, lint_program, parse_program_with_source,
    prove_program, prove_program_reference, write_proof_trace,
};
use serde::Serialize;

mod selfdoc;

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
        #[arg(long, value_enum, default_value_t = ProveEngine::Native)]
        engine: ProveEngine,
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
        #[arg(long, value_enum, default_value_t = ProveEngine::Native)]
        engine: ProveEngine,
        #[arg(long, default_value_t = false)]
        pdf: bool,
    },
    Lint {
        #[arg(required = true, num_args = 1..)]
        files: Vec<PathBuf>,
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
        #[arg(long, default_value_t = false)]
        deny_warnings: bool,
        #[arg(long, default_value_t = false)]
        semantic_dup: bool,
    },
    Fmt {
        #[arg(required = true, num_args = 1..)]
        files: Vec<PathBuf>,
        #[arg(long, default_value_t = false)]
        check: bool,
        #[arg(long, default_value_t = false)]
        stdout: bool,
    },
    Selfdoc {
        #[arg(long, default_value = ".")]
        repo: PathBuf,
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, value_enum, default_value_t = DocFormat::Markdown)]
        format: DocFormat,
        #[arg(long, value_enum, default_value_t = ProveEngine::Native)]
        engine: ProveEngine,
        #[arg(long, default_value_t = false)]
        pdf: bool,
    },
    Selfcheck {
        #[arg(long, default_value = ".")]
        repo: PathBuf,
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
        #[arg(long, value_enum, default_value_t = DocFormat::Json)]
        doc_format: DocFormat,
        #[arg(long, value_enum, default_value_t = ProveEngine::Native)]
        engine: ProveEngine,
        #[arg(long, default_value_t = false)]
        pdf: bool,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum ProveEngine {
    Native,
    Reference,
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
    reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    arg_indices: Option<Vec<usize>>,
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

#[derive(Debug, Serialize)]
struct LintJsonResponse {
    status: &'static str,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    diagnostics: Vec<LintJsonDiagnostic>,
}

#[derive(Debug, Serialize)]
struct LintJsonDiagnostic {
    severity: &'static str,
    lint_code: &'static str,
    category: &'static str,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    confidence: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    span: Option<JsonSpan>,
}

fn main() {
    let cli = Cli::parse();
    let exit_code = match cli.command {
        Command::Check { files, format } => run_check(&files, format),
        Command::Prove {
            files,
            format,
            engine,
            out,
        } => run_prove(&files, format, engine, out.as_deref()),
        Command::Doc {
            files,
            out,
            format,
            engine,
            pdf,
        } => run_doc(&files, &out, format, engine, pdf),
        Command::Lint {
            files,
            format,
            deny_warnings,
            semantic_dup,
        } => run_lint(&files, format, deny_warnings, semantic_dup),
        Command::Fmt {
            files,
            check,
            stdout,
        } => run_fmt(&files, check, stdout),
        Command::Selfdoc {
            repo,
            config,
            out,
            format,
            engine,
            pdf,
        } => run_selfdoc(&repo, config.as_deref(), &out, format, engine, pdf),
        Command::Selfcheck {
            repo,
            config,
            out,
            format,
            doc_format,
            engine,
            pdf,
        } => run_selfcheck(
            &repo,
            config.as_deref(),
            &out,
            format,
            doc_format,
            engine,
            pdf,
        ),
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

fn run_prove(
    files: &[PathBuf],
    format: OutputFormat,
    engine: ProveEngine,
    out: Option<&Path>,
) -> i32 {
    let program = match load_program(files) {
        Ok(program) => program,
        Err(diags) => {
            emit_error(&diags, format);
            return 1;
        }
    };

    let trace = match prove_with_engine(&program, engine) {
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

fn run_doc(
    files: &[PathBuf],
    out: &Path,
    format: DocFormat,
    engine: ProveEngine,
    pdf: bool,
) -> i32 {
    let program = match load_program(files) {
        Ok(program) => program,
        Err(diags) => {
            for d in diags {
                eprintln!("{d}");
            }
            return 1;
        }
    };

    let trace = match prove_with_engine(&program, engine) {
        Ok(trace) => trace,
        Err(diags) => {
            for d in attach_source_if_missing(diags, files) {
                eprintln!("{d}");
            }
            return 1;
        }
    };

    if let Err(diags) = generate_doc_bundle_with_options(
        &program,
        &trace,
        out,
        as_doc_bundle_format(format),
        DocBundleOptions::default(),
    ) {
        for d in diags {
            eprintln!("{d}");
        }
        return 1;
    }

    if pdf {
        if format == DocFormat::Markdown {
            if let Err(message) = try_generate_pdf(out) {
                eprintln!("warning: {message}");
            }
        } else {
            let message = "JSON 形式では PDF 生成をスキップしました".to_string();
            let _ = update_doc_index_pdf(out, true, false, Some(message.clone()));
            eprintln!("warning: {message}");
        }
    } else {
        let _ = update_doc_index_pdf(out, false, false, None);
    }

    println!("ok");
    0
}

fn run_selfdoc(
    repo: &Path,
    config: Option<&Path>,
    out: &Path,
    format: DocFormat,
    engine: ProveEngine,
    pdf: bool,
) -> i32 {
    let subcommands = Cli::command()
        .get_subcommands()
        .map(|cmd| cmd.get_name().to_string())
        .collect::<Vec<_>>();

    let prepared = match selfdoc::prepare_selfdoc(repo, config, out, &subcommands) {
        Ok(prepared) => prepared,
        Err(selfdoc::PrepareError::MissingConfig { path, template }) => {
            eprintln!(
                "E-SELFDOC-CONFIG: 設定ファイルが見つかりません: {}",
                path.display()
            );
            eprintln!("以下を {} に保存してください:", path.display());
            eprintln!("{template}");
            return 2;
        }
        Err(selfdoc::PrepareError::Diagnostics(diags)) => {
            for diag in diags {
                eprintln!("{diag}");
            }
            return 1;
        }
    };

    let files = vec![prepared.generated_file.clone()];
    let program = match load_program(&files) {
        Ok(program) => program,
        Err(diags) => {
            for d in diags {
                eprintln!("{d}");
            }
            return 1;
        }
    };

    let mut trace = match prove_with_engine(&program, engine) {
        Ok(trace) => trace,
        Err(diags) => {
            for d in attach_source_if_missing(diags, &files) {
                eprintln!("{d}");
            }
            return 1;
        }
    };
    trace.profile = "selfdoc".to_string();
    trace.claim_coverage = prepared.claim_coverage;

    let options = DocBundleOptions {
        profile: Some("selfdoc".to_string()),
        self_description: Some(prepared.self_description),
        intermediate_dsl: Some(prepared.generated_relative),
    };
    if let Err(diags) = generate_doc_bundle_with_options(
        &program,
        &trace,
        out,
        as_doc_bundle_format(format),
        options,
    ) {
        for d in diags {
            eprintln!("{d}");
        }
        return 1;
    }

    if pdf {
        if format == DocFormat::Markdown {
            if let Err(message) = try_generate_pdf(out) {
                eprintln!("warning: {message}");
            }
        } else {
            let message = "JSON 形式では PDF 生成をスキップしました".to_string();
            let _ = update_doc_index_pdf(out, true, false, Some(message.clone()));
            eprintln!("warning: {message}");
        }
    } else {
        let _ = update_doc_index_pdf(out, false, false, None);
    }

    println!("ok");
    0
}

fn run_selfcheck(
    repo: &Path,
    config: Option<&Path>,
    out: &Path,
    format: OutputFormat,
    doc_format: DocFormat,
    engine: ProveEngine,
    pdf: bool,
) -> i32 {
    let subcommands = Cli::command()
        .get_subcommands()
        .map(|cmd| cmd.get_name().to_string())
        .collect::<Vec<_>>();

    let prepared = match selfdoc::prepare_selfdoc(repo, config, out, &subcommands) {
        Ok(prepared) => prepared,
        Err(selfdoc::PrepareError::MissingConfig { path, template }) => {
            let diag = Diagnostic::new(
                "E-SELFDOC-CONFIG",
                format!("設定ファイルが見つかりません: {}", path.display()),
                None,
            )
            .with_source(path.display().to_string());
            match format {
                OutputFormat::Text => {
                    eprintln!("{diag}");
                    eprintln!("以下を {} に保存してください:", path.display());
                    eprintln!("{template}");
                }
                OutputFormat::Json => {
                    emit_json(ProveJsonResponse {
                        status: "error",
                        proof: None,
                        diagnostics: vec![as_json_diagnostic(&diag)],
                    });
                }
            }
            return 2;
        }
        Err(selfdoc::PrepareError::Diagnostics(diags)) => {
            match format {
                OutputFormat::Text => {
                    for diag in &diags {
                        eprintln!("{diag}");
                    }
                }
                OutputFormat::Json => {
                    emit_json(ProveJsonResponse {
                        status: "error",
                        proof: None,
                        diagnostics: diags.iter().map(as_json_diagnostic).collect(),
                    });
                }
            }
            return 1;
        }
    };

    let files = vec![prepared.generated_file.clone()];
    let program = match load_program(&files) {
        Ok(program) => program,
        Err(diags) => {
            match format {
                OutputFormat::Text => {
                    for d in &diags {
                        eprintln!("{d}");
                    }
                }
                OutputFormat::Json => {
                    emit_json(ProveJsonResponse {
                        status: "error",
                        proof: None,
                        diagnostics: diags.iter().map(as_json_diagnostic).collect(),
                    });
                }
            }
            return 1;
        }
    };

    let mut trace = match prove_with_engine(&program, engine) {
        Ok(trace) => trace,
        Err(diags) => {
            let diags = attach_source_if_missing(diags, &files);
            match format {
                OutputFormat::Text => {
                    for d in &diags {
                        eprintln!("{d}");
                    }
                }
                OutputFormat::Json => {
                    emit_json(ProveJsonResponse {
                        status: "error",
                        proof: None,
                        diagnostics: diags.iter().map(as_json_diagnostic).collect(),
                    });
                }
            }
            return 1;
        }
    };
    trace.profile = "selfdoc".to_string();
    trace.claim_coverage = prepared.claim_coverage;

    if let Err(err) = fs::create_dir_all(out) {
        let diag = Diagnostic::new(
            "E-IO",
            format!("failed to create output directory {}: {err}", out.display()),
            None,
        );
        match format {
            OutputFormat::Text => eprintln!("{diag}"),
            OutputFormat::Json => {
                emit_json(ProveJsonResponse {
                    status: "error",
                    proof: Some(trace),
                    diagnostics: vec![as_json_diagnostic(&diag)],
                });
            }
        }
        return 1;
    }
    if let Err(diag) = write_proof_trace(&out.join("proof-trace.json"), &trace) {
        match format {
            OutputFormat::Text => eprintln!("{diag}"),
            OutputFormat::Json => {
                emit_json(ProveJsonResponse {
                    status: "error",
                    proof: Some(trace),
                    diagnostics: vec![as_json_diagnostic(&diag)],
                });
            }
        }
        return 1;
    }

    let has_failed = has_failed_obligation(&trace);
    let has_full_coverage =
        has_full_claim_coverage(&trace) && trace.claim_coverage.total_claims > 0;
    if has_failed || !has_full_coverage {
        let mut diagnostics = Vec::new();
        if !has_full_coverage {
            diagnostics.push(Diagnostic::new(
                "E-SELFCHECK",
                format!(
                    "claim coverage が不足しています: {}/{}",
                    trace.claim_coverage.proved_claims, trace.claim_coverage.total_claims
                ),
                None,
            ));
        }
        match format {
            OutputFormat::Text => {
                if has_failed {
                    eprintln!("selfcheck proof failed");
                    for obligation in &trace.obligations {
                        if obligation.result != "proved" {
                            eprintln!("- {}", obligation.id);
                        }
                    }
                }
                for diag in diagnostics {
                    eprintln!("{diag}");
                }
            }
            OutputFormat::Json => {
                emit_json(ProveJsonResponse {
                    status: "error",
                    proof: Some(trace),
                    diagnostics: diagnostics.iter().map(as_json_diagnostic).collect(),
                });
            }
        }
        return 1;
    }

    let options = DocBundleOptions {
        profile: Some("selfdoc".to_string()),
        self_description: Some(prepared.self_description),
        intermediate_dsl: Some(prepared.generated_relative),
    };
    if let Err(diags) = generate_doc_bundle_with_options(
        &program,
        &trace,
        out,
        as_doc_bundle_format(doc_format),
        options,
    ) {
        match format {
            OutputFormat::Text => {
                for d in &diags {
                    eprintln!("{d}");
                }
            }
            OutputFormat::Json => {
                emit_json(ProveJsonResponse {
                    status: "error",
                    proof: Some(trace),
                    diagnostics: diags.iter().map(as_json_diagnostic).collect(),
                });
            }
        }
        return 1;
    }

    if pdf {
        if doc_format == DocFormat::Markdown {
            if let Err(message) = try_generate_pdf(out) {
                eprintln!("warning: {message}");
            }
        } else {
            let message = "JSON 形式では PDF 生成をスキップしました".to_string();
            let _ = update_doc_index_pdf(out, true, false, Some(message.clone()));
            eprintln!("warning: {message}");
        }
    } else {
        let _ = update_doc_index_pdf(out, false, false, None);
    }

    match format {
        OutputFormat::Text => println!("ok"),
        OutputFormat::Json => emit_json(ProveJsonResponse {
            status: "ok",
            proof: Some(trace),
            diagnostics: Vec::new(),
        }),
    }
    0
}

fn run_lint(
    files: &[PathBuf],
    format: OutputFormat,
    deny_warnings: bool,
    semantic_dup: bool,
) -> i32 {
    let program = match load_program(files) {
        Ok(program) => program,
        Err(diags) => {
            emit_error(&diags, format);
            return 1;
        }
    };

    let mut diagnostics = lint_program(&program, LintOptions { semantic_dup });
    diagnostics = attach_lint_source_if_missing(diagnostics, files);

    match format {
        OutputFormat::Text => {
            for diag in &diagnostics {
                if let Some(source) = &diag.source {
                    eprintln!(
                        "{}: {} {} [{}]{}",
                        source,
                        diag.severity.as_str(),
                        diag.lint_code,
                        diag.category,
                        format_span(diag.span.as_ref())
                    );
                } else {
                    eprintln!(
                        "{} {} [{}]{}",
                        diag.severity.as_str(),
                        diag.lint_code,
                        diag.category,
                        format_span(diag.span.as_ref())
                    );
                }
                eprintln!("  {}", diag.message);
            }
            if diagnostics.is_empty() {
                println!("ok");
            }
        }
        OutputFormat::Json => {
            emit_json(LintJsonResponse {
                status: if diagnostics.is_empty() {
                    "ok"
                } else if deny_warnings {
                    "error"
                } else {
                    "ok"
                },
                diagnostics: diagnostics.iter().map(as_json_lint_diagnostic).collect(),
            });
        }
    }

    if deny_warnings && !diagnostics.is_empty() {
        1
    } else {
        0
    }
}

fn prove_with_engine(
    program: &Program,
    engine: ProveEngine,
) -> Result<ProofTrace, Vec<Diagnostic>> {
    match engine {
        ProveEngine::Native => prove_program(program),
        ProveEngine::Reference => prove_program_reference(program),
    }
}

fn run_fmt(files: &[PathBuf], check: bool, stdout: bool) -> i32 {
    if stdout && files.len() != 1 {
        eprintln!("E-IO: --stdout requires exactly one input file");
        return 1;
    }

    let mut has_diff = false;
    for file in files {
        let src = match fs::read_to_string(file) {
            Ok(src) => src,
            Err(err) => {
                eprintln!("{}: E-IO: failed to read file: {err}", file.display());
                return 1;
            }
        };
        let formatted = match format_source(&src, FormatOptions::default()) {
            Ok(rendered) => rendered,
            Err(diags) => {
                for diag in diags {
                    eprintln!("{}: {}", file.display(), diag);
                }
                return 1;
            }
        };
        if formatted != src {
            has_diff = true;
            if !check
                && !stdout
                && let Err(err) = fs::write(file, formatted.as_bytes())
            {
                eprintln!("{}: E-IO: failed to write file: {err}", file.display());
                return 1;
            }
        }
        if stdout {
            print!("{formatted}");
        }
    }

    if check && has_diff { 1 } else { 0 }
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
    dst.aliases.extend(src.aliases);
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
        reason: diag.reason().map(ToOwned::to_owned),
        arg_indices: diag.arg_indices().map(ToOwned::to_owned),
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

fn as_json_lint_diagnostic(diag: &LintDiagnostic) -> LintJsonDiagnostic {
    LintJsonDiagnostic {
        severity: diag.severity.as_str(),
        lint_code: diag.lint_code,
        category: diag.category,
        message: diag.message.clone(),
        source: diag.source.clone(),
        confidence: diag.confidence,
        span: diag.span.as_ref().map(as_json_span),
    }
}

fn attach_lint_source_if_missing(
    diags: Vec<LintDiagnostic>,
    files: &[PathBuf],
) -> Vec<LintDiagnostic> {
    let single_source = if files.len() == 1 {
        Some(files[0].display().to_string())
    } else {
        None
    };

    diags
        .into_iter()
        .map(|mut d| {
            if d.source.is_none() {
                if let Some(file_id) = d.span.as_ref().and_then(|span| span.file_id.clone()) {
                    d.source = Some(file_id);
                } else if let Some(source) = &single_source {
                    d.source = Some(source.clone());
                }
            }
            d
        })
        .collect()
}

fn format_span(span: Option<&Span>) -> String {
    match span {
        Some(span) => format!(" at {}:{}", span.line, span.column),
        None => String::new(),
    }
}

fn try_generate_pdf(out_dir: &Path) -> Result<(), String> {
    let md_path = out_dir.join("spec.md");
    let pdf_path = out_dir.join("spec.pdf");
    if !md_path.exists() {
        return Err("spec.md が見つからないため PDF 生成をスキップしました".to_string());
    }

    let output = ProcessCommand::new("pandoc")
        .arg(md_path.as_os_str())
        .arg("-o")
        .arg(pdf_path.as_os_str())
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                let _ = update_doc_index_pdf(out_dir, true, true, None);
                Ok(())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                let message = if stderr.is_empty() {
                    "pandoc 実行に失敗しました".to_string()
                } else {
                    format!("pandoc 実行に失敗しました: {stderr}")
                };
                let _ = update_doc_index_pdf(out_dir, true, false, Some(message.clone()));
                Err(message)
            }
        }
        Err(err) => {
            let message = format!("pandoc が利用できないため PDF を生成できません: {err}");
            let _ = update_doc_index_pdf(out_dir, true, false, Some(message.clone()));
            Err(message)
        }
    }
}

fn update_doc_index_pdf(
    out_dir: &Path,
    requested: bool,
    generated: bool,
    message: Option<String>,
) -> Result<(), String> {
    let index_path = out_dir.join("doc-index.json");
    if !index_path.exists() {
        return Ok(());
    }
    let body =
        fs::read_to_string(&index_path).map_err(|e| format!("doc-index.json 読み込み失敗: {e}"))?;
    let mut value: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("doc-index.json JSON 解析失敗: {e}"))?;
    value["pdf"] = serde_json::json!({
        "requested": requested,
        "generated": generated,
        "message": message,
    });
    fs::write(
        &index_path,
        serde_json::to_string_pretty(&value)
            .map_err(|e| format!("doc-index.json JSON 生成失敗: {e}"))?,
    )
    .map_err(|e| format!("doc-index.json 書き込み失敗: {e}"))?;
    Ok(())
}
