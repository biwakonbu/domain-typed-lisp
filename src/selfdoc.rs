use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

use dtl::{
    ClaimCoverage, Diagnostic, DocContract, DocModule, DocProject, DocQualityGate, DocReference,
    DocSelfDescription,
};
use globset::{Glob, GlobSet, GlobSetBuilder};
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_yaml::Value as YamlValue;

const DEFAULT_CONFIG_FILENAME: &str = ".dtl-selfdoc.toml";

const CONFIG_TEMPLATE: &str = r#"version = 1

[scan]
include = [
  "src/**",
  "tests/**",
  "benches/**",
  "docs/**",
  "docs-site/**",
  "examples/**",
  "scripts/**",
  "tooling/**",
  "editors/**",
  ".github/workflows/**",
  "README.md",
  "TODO.md",
  "LICENSE",
  "Cargo.toml",
  "Cargo.lock",
  "rust-toolchain.toml",
  ".gitignore",
  ".dtl-selfdoc.toml"
]
exclude = [
  "target/**"
]
use_gitignore = true

[[classify]]
category = "source"
patterns = ["src/**"]

[[classify]]
category = "test"
patterns = ["tests/**", "benches/**"]

[[classify]]
category = "doc"
patterns = ["docs/**", "docs-site/**", "README.md"]

[[classify]]
category = "ci"
patterns = [".github/workflows/**"]

[[classify]]
category = "script"
patterns = ["scripts/**"]

[[classify]]
category = "tooling"
patterns = ["tooling/**", "editors/**"]

[[classify]]
category = "example"
patterns = ["examples/**"]

[[classify]]
category = "config"
patterns = ["Cargo.toml", "Cargo.lock", "rust-toolchain.toml", ".gitignore", ".dtl-selfdoc.toml"]
"#;

#[derive(Debug, Clone)]
pub struct PreparedSelfdoc {
    pub generated_file: PathBuf,
    pub generated_relative: String,
    pub self_description: DocSelfDescription,
    pub claim_coverage: ClaimCoverage,
}

#[derive(Debug)]
pub enum PrepareError {
    MissingConfig { path: PathBuf, template: String },
    Diagnostics(Vec<Diagnostic>),
}

#[derive(Debug, Deserialize)]
struct SelfdocConfig {
    version: u32,
    scan: ScanConfig,
    classify: Vec<ClassifyRuleConfig>,
}

#[derive(Debug, Deserialize)]
struct ScanConfig {
    include: Vec<String>,
    exclude: Vec<String>,
    #[serde(default)]
    use_gitignore: bool,
}

#[derive(Debug, Deserialize)]
struct ClassifyRuleConfig {
    category: String,
    patterns: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
enum FileCategory {
    Source,
    Test,
    Doc,
    Ci,
    Script,
    Tooling,
    Example,
    Config,
    Asset,
    Other,
}

impl FileCategory {
    fn parse(raw: &str) -> Option<Self> {
        match raw {
            "source" => Some(Self::Source),
            "test" => Some(Self::Test),
            "doc" => Some(Self::Doc),
            "ci" => Some(Self::Ci),
            "script" => Some(Self::Script),
            "tooling" => Some(Self::Tooling),
            "example" => Some(Self::Example),
            "config" => Some(Self::Config),
            "asset" => Some(Self::Asset),
            "other" => Some(Self::Other),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::Test => "test",
            Self::Doc => "doc",
            Self::Ci => "ci",
            Self::Script => "script",
            Self::Tooling => "tooling",
            Self::Example => "example",
            Self::Config => "config",
            Self::Asset => "asset",
            Self::Other => "other",
        }
    }
}

#[derive(Debug, Clone)]
struct Artifact {
    path: String,
    category: FileCategory,
}

#[derive(Debug, Clone, Serialize)]
struct SelfdocProject {
    name: String,
    summary: String,
}

#[derive(Debug, Clone, Serialize)]
struct SelfdocModule {
    name: String,
    path: String,
    category: String,
}

#[derive(Debug, Clone, Serialize)]
struct SelfdocLink {
    from: String,
    to: String,
}

#[derive(Debug, Clone, Serialize)]
struct SelfdocContract {
    name: String,
    source: String,
    path: String,
}

#[derive(Debug, Clone, Serialize)]
struct SelfdocGate {
    name: String,
    command: String,
    source: String,
    required: bool,
}

#[derive(Debug)]
struct PreparedData {
    project: SelfdocProject,
    modules: Vec<SelfdocModule>,
    references: Vec<SelfdocLink>,
    contracts: Vec<SelfdocContract>,
    quality_gates: Vec<SelfdocGate>,
    extra_exists_paths: Vec<String>,
}

pub fn default_config_template() -> &'static str {
    CONFIG_TEMPLATE
}

pub fn default_config_path(repo: &Path) -> PathBuf {
    repo.join(DEFAULT_CONFIG_FILENAME)
}

pub fn prepare_selfdoc(
    repo: &Path,
    config_override: Option<&Path>,
    out_dir: &Path,
    cli_subcommands: &[String],
) -> Result<PreparedSelfdoc, PrepareError> {
    let repo = fs::canonicalize(repo).unwrap_or_else(|_| repo.to_path_buf());
    let config_path = config_override
        .map(PathBuf::from)
        .unwrap_or_else(|| default_config_path(&repo));

    if !config_path.exists() {
        return Err(PrepareError::MissingConfig {
            path: config_path,
            template: default_config_template().to_string(),
        });
    }

    let config_body = fs::read_to_string(&config_path).map_err(|err| {
        PrepareError::Diagnostics(vec![diag(
            "E-SELFDOC-CONFIG",
            format!("設定ファイルを読み込めません: {err}"),
            Some(config_path.display().to_string()),
        )])
    })?;

    let config: SelfdocConfig = toml::from_str(&config_body).map_err(|err| {
        PrepareError::Diagnostics(vec![diag(
            "E-SELFDOC-CONFIG",
            format!("設定ファイルが TOML として不正です: {err}"),
            Some(config_path.display().to_string()),
        )])
    })?;

    let mut errors = validate_config(&config, &config_path);
    if !errors.is_empty() {
        return Err(PrepareError::Diagnostics(errors));
    }

    let include = compile_globset(&config.scan.include, "E-SELFDOC-CONFIG", &config_path)
        .map_err(PrepareError::Diagnostics)?;
    let exclude = compile_globset(&config.scan.exclude, "E-SELFDOC-CONFIG", &config_path)
        .map_err(PrepareError::Diagnostics)?;
    let classify_rules = compile_classify_rules(&config.classify, &config_path)
        .map_err(PrepareError::Diagnostics)?;

    let gitignore = if config.scan.use_gitignore {
        Some(build_gitignore(&repo, &config_path).map_err(PrepareError::Diagnostics)?)
    } else {
        None
    };

    let paths = scan_paths(&repo, &include, &exclude, gitignore.as_ref());
    if paths.is_empty() {
        return Err(PrepareError::Diagnostics(vec![diag(
            "E-SELFDOC-SCAN",
            "走査対象ファイルが 1 件もありません".to_string(),
            Some(config_path.display().to_string()),
        )]));
    }

    let mut artifacts = Vec::new();
    for path in paths {
        let mut matched = HashSet::new();
        for (category, matcher) in &classify_rules {
            if matcher.is_match(&path) {
                matched.insert(*category);
            }
        }
        if matched.len() != 1 {
            let detail = if matched.is_empty() {
                "分類ルールに一致しません".to_string()
            } else {
                let mut cats = matched.iter().map(|c| c.as_str()).collect::<Vec<_>>();
                cats.sort_unstable();
                format!("複数カテゴリに一致しました: {}", cats.join(", "))
            };
            errors.push(diag(
                "E-SELFDOC-CLASSIFY",
                format!("{} ({detail})", path),
                Some(config_path.display().to_string()),
            ));
            continue;
        }
        let category = *matched.iter().next().expect("single match");
        artifacts.push(Artifact { path, category });
    }

    if !errors.is_empty() {
        return Err(PrepareError::Diagnostics(errors));
    }

    artifacts.sort_by(|a, b| a.path.cmp(&b.path));

    let reference_result = extract_references(&repo, &artifacts);
    if !reference_result.errors.is_empty() {
        return Err(PrepareError::Diagnostics(reference_result.errors));
    }

    let cli_contracts = extract_cli_contracts(&repo, cli_subcommands);
    if !cli_contracts.errors.is_empty() {
        return Err(PrepareError::Diagnostics(cli_contracts.errors));
    }

    let quality_gates = extract_quality_gates(&repo, &artifacts);
    if !quality_gates.errors.is_empty() {
        return Err(PrepareError::Diagnostics(quality_gates.errors));
    }

    let data = build_prepared_data(
        &repo,
        &artifacts,
        &reference_result.references,
        &reference_result.extra_exists,
        &cli_contracts.contracts,
        &quality_gates.gates,
    );

    let rendered = render_selfdoc_program(&data);
    fs::create_dir_all(out_dir).map_err(|err| {
        PrepareError::Diagnostics(vec![diag(
            "E-IO",
            format!("出力ディレクトリを作成できません: {err}"),
            Some(out_dir.display().to_string()),
        )])
    })?;

    let generated_file = out_dir.join("selfdoc.generated.dtl");
    fs::write(&generated_file, rendered.as_bytes()).map_err(|err| {
        PrepareError::Diagnostics(vec![diag(
            "E-IO",
            format!("自己記述 DSL を書き込めません: {err}"),
            Some(generated_file.display().to_string()),
        )])
    })?;

    Ok(PreparedSelfdoc {
        generated_file,
        generated_relative: "selfdoc.generated.dtl".to_string(),
        self_description: DocSelfDescription {
            project: Some(DocProject {
                name: data.project.name,
                summary: data.project.summary,
            }),
            modules: data
                .modules
                .iter()
                .map(|m| DocModule {
                    name: m.name.clone(),
                    path: m.path.clone(),
                    category: m.category.clone(),
                })
                .collect(),
            references: data
                .references
                .iter()
                .map(|r| DocReference {
                    from: r.from.clone(),
                    to: r.to.clone(),
                })
                .collect(),
            contracts: data
                .contracts
                .iter()
                .map(|c| DocContract {
                    name: c.name.clone(),
                    source: c.source.clone(),
                    path: c.path.clone(),
                })
                .collect(),
            quality_gates: data
                .quality_gates
                .iter()
                .map(|g| DocQualityGate {
                    name: g.name.clone(),
                    command: g.command.clone(),
                    source: g.source.clone(),
                    required: g.required,
                })
                .collect(),
        },
        claim_coverage: ClaimCoverage {
            total_claims: cli_contracts.total_claims,
            proved_claims: cli_contracts.proved_claims,
        },
    })
}

fn validate_config(config: &SelfdocConfig, source: &Path) -> Vec<Diagnostic> {
    let mut errors = Vec::new();
    if config.version != 1 {
        errors.push(diag(
            "E-SELFDOC-CONFIG",
            format!("version は 1 のみ許可です: {}", config.version),
            Some(source.display().to_string()),
        ));
    }
    if config.scan.include.is_empty() {
        errors.push(diag(
            "E-SELFDOC-CONFIG",
            "scan.include は 1 件以上必要です".to_string(),
            Some(source.display().to_string()),
        ));
    }
    if config.classify.is_empty() {
        errors.push(diag(
            "E-SELFDOC-CONFIG",
            "classify は 1 件以上必要です".to_string(),
            Some(source.display().to_string()),
        ));
    }
    for rule in &config.classify {
        if FileCategory::parse(&rule.category).is_none() {
            errors.push(diag(
                "E-SELFDOC-CONFIG",
                format!("無効な category: {}", rule.category),
                Some(source.display().to_string()),
            ));
        }
        if rule.patterns.is_empty() {
            errors.push(diag(
                "E-SELFDOC-CONFIG",
                format!("category={} の patterns は 1 件以上必要です", rule.category),
                Some(source.display().to_string()),
            ));
        }
    }
    errors
}

fn compile_globset(
    patterns: &[String],
    code: &'static str,
    source: &Path,
) -> Result<GlobSet, Vec<Diagnostic>> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        match Glob::new(pattern) {
            Ok(glob) => {
                builder.add(glob);
            }
            Err(err) => {
                return Err(vec![diag(
                    code,
                    format!("glob パターンが不正です: {pattern}: {err}"),
                    Some(source.display().to_string()),
                )]);
            }
        }
    }
    builder.build().map_err(|err| {
        vec![diag(
            code,
            format!("glob セットを構築できません: {err}"),
            Some(source.display().to_string()),
        )]
    })
}

fn compile_classify_rules(
    rules: &[ClassifyRuleConfig],
    source: &Path,
) -> Result<Vec<(FileCategory, GlobSet)>, Vec<Diagnostic>> {
    let mut out = Vec::new();
    for rule in rules {
        let Some(category) = FileCategory::parse(&rule.category) else {
            continue;
        };
        let matcher = compile_globset(&rule.patterns, "E-SELFDOC-CONFIG", source)?;
        out.push((category, matcher));
    }
    Ok(out)
}

fn build_gitignore(repo: &Path, source: &Path) -> Result<Gitignore, Vec<Diagnostic>> {
    let mut builder = GitignoreBuilder::new(repo);
    let gitignore = repo.join(".gitignore");
    if gitignore.exists() {
        builder.add(gitignore);
    }
    builder.build().map_err(|err| {
        vec![diag(
            "E-SELFDOC-CONFIG",
            format!(".gitignore を解析できません: {err}"),
            Some(source.display().to_string()),
        )]
    })
}

fn scan_paths(
    repo: &Path,
    include: &GlobSet,
    exclude: &GlobSet,
    gitignore: Option<&Gitignore>,
) -> Vec<String> {
    let mut files = Vec::new();
    collect_files_recursive(repo, &mut files);

    let mut out = Vec::new();
    for file in files {
        let rel = match file.strip_prefix(repo) {
            Ok(rel) => rel,
            Err(_) => continue,
        };
        let rel_posix = to_posix_path(rel);
        if !include.is_match(&rel_posix) {
            continue;
        }
        if exclude.is_match(&rel_posix) {
            continue;
        }
        if let Some(gitignore) = gitignore {
            let ignored = gitignore
                .matched_path_or_any_parents(rel, false)
                .is_ignore();
            if ignored {
                continue;
            }
        }
        out.push(rel_posix);
    }
    out.sort();
    out
}

fn collect_files_recursive(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        if meta.is_dir() {
            if path
                .file_name()
                .and_then(|s| s.to_str())
                .is_some_and(|s| s == ".git")
            {
                continue;
            }
            collect_files_recursive(&path, out);
            continue;
        }
        if meta.is_file() {
            out.push(path);
        }
    }
}

struct ReferenceExtraction {
    references: Vec<SelfdocLink>,
    extra_exists: Vec<String>,
    errors: Vec<Diagnostic>,
}

fn extract_references(repo: &Path, artifacts: &[Artifact]) -> ReferenceExtraction {
    let mut references = Vec::new();
    let mut extra_exists = BTreeSet::new();
    let mut errors = Vec::new();

    let import_re =
        Regex::new(r#"\(\s*import\s+\"((?:\\.|[^\"\\])*)\"\s*\)"#).expect("valid import regex");
    let link_re = Regex::new(r#"\[[^\]]+\]\(([^)]+)\)"#).expect("valid markdown link regex");
    let include_re =
        Regex::new(r#"\{\{#include\s+([^\s\}]+)\s*\}\}"#).expect("valid include regex");

    for artifact in artifacts {
        let abs = repo.join(&artifact.path);
        let Ok(body) = fs::read_to_string(&abs) else {
            continue;
        };
        let ext = abs
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        let mut candidates = Vec::new();
        let mut yaml_paths = false;
        match ext.as_str() {
            "dtl" => {
                for caps in import_re.captures_iter(&body) {
                    let Some(raw) = caps.get(1) else {
                        continue;
                    };
                    match decode_escaped(raw.as_str()) {
                        Ok(path) => candidates.push(path),
                        Err(message) => errors.push(diag(
                            "E-SELFDOC-REF",
                            format!("import パスのエスケープが不正です: {message}"),
                            Some(artifact.path.clone()),
                        )),
                    }
                }
            }
            "md" => {
                for caps in link_re.captures_iter(&body) {
                    let Some(raw) = caps.get(1) else {
                        continue;
                    };
                    if let Some(target) = normalize_markdown_target(raw.as_str()) {
                        candidates.push(target);
                    }
                }
                for caps in include_re.captures_iter(&body) {
                    let Some(raw) = caps.get(1) else {
                        continue;
                    };
                    candidates.push(raw.as_str().to_string());
                }
            }
            "yml" | "yaml" => {
                yaml_paths = true;
                match serde_yaml::from_str::<YamlValue>(&body) {
                    Ok(value) => collect_yaml_paths(&value, &mut candidates),
                    Err(err) => {
                        errors.push(diag(
                            "E-SELFDOC-REF",
                            format!("YAML 解析に失敗しました: {err}"),
                            Some(artifact.path.clone()),
                        ));
                    }
                }
            }
            _ => {}
        }

        for candidate in candidates {
            let Some(target_rel) = (if yaml_paths {
                normalize_yaml_reference_target(&candidate)
            } else {
                normalize_reference_target(&artifact.path, &candidate)
            }) else {
                continue;
            };
            let target_abs = repo.join(&target_rel);
            if !target_abs.exists() {
                errors.push(diag(
                    "E-SELFDOC-REF",
                    format!("参照先が存在しません: {} -> {}", artifact.path, target_rel),
                    Some(artifact.path.clone()),
                ));
                continue;
            }
            if target_abs.is_dir() {
                extra_exists.insert(target_rel.clone());
            }
            references.push(SelfdocLink {
                from: artifact.path.clone(),
                to: target_rel,
            });
        }
    }

    references.sort_by(|a, b| {
        if a.from == b.from {
            a.to.cmp(&b.to)
        } else {
            a.from.cmp(&b.from)
        }
    });
    references.dedup_by(|a, b| a.from == b.from && a.to == b.to);

    ReferenceExtraction {
        references,
        extra_exists: extra_exists.into_iter().collect(),
        errors,
    }
}

fn collect_yaml_paths(value: &YamlValue, out: &mut Vec<String>) {
    match value {
        YamlValue::Mapping(map) => {
            for (key, child) in map {
                if let YamlValue::String(k) = key {
                    if k == "uses" {
                        if let YamlValue::String(s) = child
                            && s.starts_with("./")
                        {
                            out.push(s.to_string());
                        }
                    } else if k == "path" {
                        collect_path_values(child, out);
                    }
                }
                collect_yaml_paths(child, out);
            }
        }
        YamlValue::Sequence(seq) => {
            for item in seq {
                collect_yaml_paths(item, out);
            }
        }
        _ => {}
    }
}

fn collect_path_values(value: &YamlValue, out: &mut Vec<String>) {
    match value {
        YamlValue::String(s) => {
            if is_local_path_value(s) {
                out.push(s.to_string());
            }
        }
        YamlValue::Sequence(seq) => {
            for item in seq {
                if let YamlValue::String(s) = item
                    && is_local_path_value(s)
                {
                    out.push(s.to_string());
                }
            }
        }
        _ => {}
    }
}

fn is_local_path_value(path: &str) -> bool {
    if path.is_empty() {
        return false;
    }
    if path.starts_with("http://")
        || path.starts_with("https://")
        || path.starts_with("mailto:")
        || path.contains("://")
        || path.starts_with("${{")
    {
        return false;
    }
    true
}

fn normalize_markdown_target(raw: &str) -> Option<String> {
    let mut s = raw.trim();
    if s.is_empty() {
        return None;
    }
    if s.starts_with('<') && s.ends_with('>') && s.len() > 2 {
        s = &s[1..s.len() - 1];
    }
    if s.starts_with('#') {
        return None;
    }
    if s.starts_with("http://")
        || s.starts_with("https://")
        || s.starts_with("mailto:")
        || s.contains("://")
    {
        return None;
    }

    // markdown のタイトル付きリンクを除外
    let target = s.split_whitespace().next().unwrap_or(s);
    Some(target.to_string())
}

fn normalize_yaml_reference_target(raw_target: &str) -> Option<String> {
    let mut target = raw_target.trim().to_string();
    if target.is_empty() {
        return None;
    }
    if let Some((prefix, _)) = target.split_once('#') {
        target = prefix.to_string();
    }
    if let Some((prefix, _)) = target.split_once('?') {
        target = prefix.to_string();
    }
    if target.is_empty() {
        return None;
    }
    if target.starts_with("http://")
        || target.starts_with("https://")
        || target.starts_with("mailto:")
        || target.contains("://")
    {
        return None;
    }

    let normalized = if target.starts_with('/') {
        target.trim_start_matches('/').to_string()
    } else if target.starts_with("./") {
        target.trim_start_matches("./").to_string()
    } else {
        target
    };
    normalize_path_relative(Path::new(&normalized))
}

fn normalize_reference_target(from: &str, raw_target: &str) -> Option<String> {
    let mut target = raw_target.trim().to_string();
    if target.is_empty() {
        return None;
    }
    if let Some((prefix, _)) = target.split_once('#') {
        target = prefix.to_string();
    }
    if let Some((prefix, _)) = target.split_once('?') {
        target = prefix.to_string();
    }
    if target.is_empty() {
        return None;
    }
    if target.starts_with("http://")
        || target.starts_with("https://")
        || target.starts_with("mailto:")
        || target.contains("://")
    {
        return None;
    }

    let base_dir = Path::new(from).parent().unwrap_or(Path::new(""));
    let joined = if target.starts_with('/') {
        PathBuf::from(target.trim_start_matches('/'))
    } else {
        base_dir.join(target)
    };
    normalize_path_relative(&joined)
}

fn normalize_path_relative(path: &Path) -> Option<String> {
    let mut stack: Vec<String> = Vec::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                stack.pop()?;
            }
            Component::Normal(part) => {
                stack.push(part.to_string_lossy().to_string());
            }
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    if stack.is_empty() {
        return None;
    }
    Some(stack.join("/"))
}

struct CliContractExtraction {
    contracts: Vec<SelfdocContract>,
    errors: Vec<Diagnostic>,
    total_claims: usize,
    proved_claims: usize,
}

fn extract_cli_contracts(repo: &Path, subcommands: &[String]) -> CliContractExtraction {
    const START_MARKER: &str = "<!-- selfdoc:cli-contracts:start -->";
    const END_MARKER: &str = "<!-- selfdoc:cli-contracts:end -->";

    let expected = subcommands.iter().cloned().collect::<BTreeSet<_>>();
    let mut contracts_by_subcommand: BTreeMap<String, SelfdocContract> = BTreeMap::new();
    let mut errors = Vec::new();
    let mut table_found = false;
    let mut docs_without_table = Vec::new();

    let docs = vec![
        (
            "README.md",
            fs::read_to_string(repo.join("README.md")).unwrap_or_default(),
        ),
        (
            "docs/language-spec.md",
            fs::read_to_string(repo.join("docs/language-spec.md")).unwrap_or_default(),
        ),
    ];

    for (source, body) in docs {
        let section = extract_tagged_section(&body, START_MARKER, END_MARKER);
        if let Some(section) = section {
            table_found = true;
            let parsed = parse_contract_table(section, source, &expected);
            errors.extend(parsed.errors);
            for (subcommand, impl_path) in parsed.entries {
                let key = subcommand.clone();
                let contract = SelfdocContract {
                    name: format!("cli::{subcommand}"),
                    source: source.to_string(),
                    path: impl_path,
                };
                if let Some(prev) = contracts_by_subcommand.insert(key.clone(), contract) {
                    if prev.source != source {
                        errors.push(diag(
                            "E-SELFDOC-CONTRACT",
                            format!(
                                "CLI 契約が複数文書で重複しています: cli::{key} ({}, {})",
                                prev.source, source
                            ),
                            Some(source.to_string()),
                        ));
                    }
                }
            }
            continue;
        }
        docs_without_table.push((source, body));
    }

    if !table_found {
        for (source, body) in docs_without_table {
            for name in &expected {
                let pattern = format!("dtl {name}");
                if body.contains(&pattern) {
                    errors.push(diag(
                        "E-SELFDOC-CONTRACT",
                        format!(
                            "構造化契約テーブルなしで CLI 文字列を検出しました: `{pattern}` (`{source}`)"
                        ),
                        Some(source.to_string()),
                    ));
                }
            }
        }
        errors.push(diag(
            "E-SELFDOC-CONTRACT",
            "CLI 契約テーブルが見つかりません。`<!-- selfdoc:cli-contracts:start -->` / `<!-- selfdoc:cli-contracts:end -->` で定義してください。".to_string(),
            None,
        ));
    }

    let contracts = contracts_by_subcommand.into_values().collect::<Vec<_>>();
    CliContractExtraction {
        total_claims: expected.len(),
        proved_claims: contracts.len(),
        contracts,
        errors,
    }
}

struct ParsedContractTable {
    entries: Vec<(String, String)>,
    errors: Vec<Diagnostic>,
}

fn extract_tagged_section<'a>(
    body: &'a str,
    start_marker: &str,
    end_marker: &str,
) -> Option<&'a str> {
    let start = body.find(start_marker)?;
    let rest = &body[start + start_marker.len()..];
    let end = rest.find(end_marker)?;
    Some(&rest[..end])
}

fn parse_contract_table(
    section: &str,
    source: &str,
    expected: &BTreeSet<String>,
) -> ParsedContractTable {
    let mut entries = Vec::new();
    let mut errors = Vec::new();
    let lines = section
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    let table_lines = lines
        .iter()
        .copied()
        .filter(|line| line.starts_with('|') && line.ends_with('|'))
        .collect::<Vec<_>>();

    if table_lines.len() < 3 {
        errors.push(diag(
            "E-SELFDOC-CONTRACT",
            "CLI 契約テーブルの行数が不足しています（header + separator + data が必要）"
                .to_string(),
            Some(source.to_string()),
        ));
        return ParsedContractTable { entries, errors };
    }

    let headers = parse_table_row(table_lines[0]);
    let Some(subcommand_idx) = headers.iter().position(|cell| {
        let name = normalize_header_cell(cell);
        name == "subcommand" || name == "サブコマンド"
    }) else {
        errors.push(diag(
            "E-SELFDOC-CONTRACT",
            "CLI 契約テーブルに `subcommand` 列がありません".to_string(),
            Some(source.to_string()),
        ));
        return ParsedContractTable { entries, errors };
    };
    let Some(path_idx) = headers.iter().position(|cell| {
        let name = normalize_header_cell(cell);
        name == "impl_path" || name == "path" || name == "実装パス"
    }) else {
        errors.push(diag(
            "E-SELFDOC-CONTRACT",
            "CLI 契約テーブルに `impl_path` 列がありません".to_string(),
            Some(source.to_string()),
        ));
        return ParsedContractTable { entries, errors };
    };

    let mut seen = BTreeSet::new();
    for line in table_lines.iter().skip(2) {
        let cells = parse_table_row(line);
        if cells.len() <= subcommand_idx || cells.len() <= path_idx {
            continue;
        }
        let subcommand = normalize_subcommand_cell(&cells[subcommand_idx]);
        let impl_path = normalize_table_cell(&cells[path_idx]);
        if subcommand.is_empty() || impl_path.is_empty() {
            continue;
        }
        if !expected.contains(&subcommand) {
            errors.push(diag(
                "E-SELFDOC-CONTRACT",
                format!("未知の subcommand です: `{subcommand}`"),
                Some(source.to_string()),
            ));
            continue;
        }
        if !seen.insert(subcommand.clone()) {
            errors.push(diag(
                "E-SELFDOC-CONTRACT",
                format!("重複した subcommand 定義です: `{subcommand}`"),
                Some(source.to_string()),
            ));
            continue;
        }
        entries.push((subcommand, impl_path));
    }

    ParsedContractTable { entries, errors }
}

fn parse_table_row(line: &str) -> Vec<String> {
    line.trim()
        .trim_start_matches('|')
        .trim_end_matches('|')
        .split('|')
        .map(|cell| cell.trim().to_string())
        .collect()
}

fn normalize_header_cell(cell: &str) -> String {
    normalize_table_cell(cell).to_ascii_lowercase()
}

fn normalize_subcommand_cell(cell: &str) -> String {
    let cell = normalize_table_cell(cell);
    if let Some(rest) = cell.strip_prefix("dtl ") {
        return rest.trim().to_string();
    }
    cell
}

fn normalize_table_cell(cell: &str) -> String {
    cell.trim()
        .trim_matches('`')
        .trim_matches('"')
        .trim_matches('\'')
        .to_string()
}

struct QualityGateExtraction {
    gates: Vec<SelfdocGate>,
    errors: Vec<Diagnostic>,
}

fn extract_quality_gates(repo: &Path, artifacts: &[Artifact]) -> QualityGateExtraction {
    let mut gates = Vec::new();
    let mut errors = Vec::new();

    for artifact in artifacts {
        if artifact.category != FileCategory::Ci {
            continue;
        }
        let path = Path::new(&artifact.path);
        let Some(ext) = path.extension().and_then(|s| s.to_str()) else {
            continue;
        };
        if ext != "yml" && ext != "yaml" {
            continue;
        }

        let abs = repo.join(&artifact.path);
        let Ok(body) = fs::read_to_string(&abs) else {
            continue;
        };
        let parsed = match serde_yaml::from_str::<YamlValue>(&body) {
            Ok(value) => value,
            Err(err) => {
                errors.push(diag(
                    "E-SELFDOC-GATE",
                    format!("workflow YAML を解析できません: {err}"),
                    Some(artifact.path.clone()),
                ));
                continue;
            }
        };

        let Some(jobs) = yaml_get_map(&parsed, "jobs") else {
            continue;
        };
        for (job_name_val, job_body) in jobs {
            let Some(job_name) = job_name_val.as_str() else {
                continue;
            };
            let Some(steps) = yaml_get_seq(job_body, "steps") else {
                continue;
            };
            for (idx, step) in steps.iter().enumerate() {
                let Some(run) = yaml_get_str(step, "run") else {
                    continue;
                };
                let required = !yaml_get_bool(step, "continue-on-error").unwrap_or(false);
                let workflow_stem = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("workflow");
                let gate_id = format!("{workflow_stem}:{job_name}:{}", idx + 1);
                gates.push(SelfdocGate {
                    name: gate_id,
                    command: run.trim().to_string(),
                    source: artifact.path.clone(),
                    required,
                });
            }
        }
    }

    gates.sort_by(|a, b| a.name.cmp(&b.name));
    gates.dedup_by(|a, b| a.name == b.name);

    QualityGateExtraction { gates, errors }
}

fn yaml_get_map<'a>(value: &'a YamlValue, key: &str) -> Option<&'a serde_yaml::Mapping> {
    let YamlValue::Mapping(map) = value else {
        return None;
    };
    let child = map.get(YamlValue::String(key.to_string()))?;
    child.as_mapping()
}

fn yaml_get_seq<'a>(value: &'a YamlValue, key: &str) -> Option<&'a Vec<YamlValue>> {
    let YamlValue::Mapping(map) = value else {
        return None;
    };
    let child = map.get(YamlValue::String(key.to_string()))?;
    child.as_sequence()
}

fn yaml_get_str<'a>(value: &'a YamlValue, key: &str) -> Option<&'a str> {
    let YamlValue::Mapping(map) = value else {
        return None;
    };
    let child = map.get(YamlValue::String(key.to_string()))?;
    child.as_str()
}

fn yaml_get_bool(value: &YamlValue, key: &str) -> Option<bool> {
    let YamlValue::Mapping(map) = value else {
        return None;
    };
    let child = map.get(YamlValue::String(key.to_string()))?;
    child.as_bool()
}

fn build_prepared_data(
    repo: &Path,
    artifacts: &[Artifact],
    references: &[SelfdocLink],
    extra_exists: &[String],
    contracts: &[SelfdocContract],
    gates: &[SelfdocGate],
) -> PreparedData {
    let readme = fs::read_to_string(repo.join("README.md")).unwrap_or_default();
    let (project_name, project_summary) = infer_project_info(repo, &readme);

    let modules = artifacts
        .iter()
        .map(|artifact| SelfdocModule {
            name: artifact.path.clone(),
            path: artifact.path.clone(),
            category: artifact.category.as_str().to_string(),
        })
        .collect::<Vec<_>>();

    PreparedData {
        project: SelfdocProject {
            name: project_name,
            summary: project_summary,
        },
        modules,
        references: references.to_vec(),
        contracts: contracts.to_vec(),
        quality_gates: gates.to_vec(),
        extra_exists_paths: extra_exists.to_vec(),
    }
}

fn infer_project_info(repo: &Path, readme: &str) -> (String, String) {
    let default_name = repo
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("project")
        .to_string();

    let mut name = default_name.clone();
    let mut summary = String::new();

    for line in readme.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("# ") {
            name = trimmed.trim_start_matches("# ").trim().to_string();
            break;
        }
    }

    for line in readme.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('!') {
            continue;
        }
        if trimmed.starts_with('[') && trimmed.contains("](http") {
            continue;
        }
        summary = trimmed.to_string();
        break;
    }

    if summary.is_empty() {
        summary = "自己記述生成により抽出したプロジェクト概要".to_string();
    }

    (name, summary)
}

fn render_selfdoc_program(data: &PreparedData) -> String {
    let mut out = String::new();
    out.push_str("; syntax: surface\n");
    out.push_str("; @context: selfdoc\n\n");

    out.push_str("(型 Path)\n");
    out.push_str("(型 Category)\n");
    out.push_str("(型 Ident)\n");
    out.push_str("(型 Flag)\n\n");

    out.push_str("(関係 exists :引数 (Path))\n");
    out.push_str("(関係 artifact :引数 (Path Category))\n");
    out.push_str("(関係 ref :引数 (Path Path))\n");
    out.push_str("(関係 contract-doc :引数 (Ident Path))\n");
    out.push_str("(関係 contract-impl :引数 (Ident Path))\n");
    out.push_str("(関係 gate-source :引数 (Ident Path))\n");
    out.push_str("(関係 gate-required :引数 (Ident Flag))\n\n");

    out.push_str("(関係 sd-project :引数 (Ident Symbol))\n");
    out.push_str("(関係 sd-module :引数 (Ident Path Category))\n");
    out.push_str("(関係 sd-reference :引数 (Path Path))\n");
    out.push_str("(関係 sd-contract :引数 (Ident Path Path))\n");
    out.push_str("(関係 sd-quality-gate :引数 (Ident Symbol Path Flag))\n\n");

    let ref_target_formula = build_ref_target_exists_formula(&data.references);
    let contract_doc_formula = build_contract_doc_exists_formula(&data.contracts);
    let contract_impl_formula = build_contract_impl_exists_formula(&data.contracts);
    let gate_source_formula = build_gate_source_exists_formula(&data.quality_gates);
    let module_artifact_formula = build_module_artifact_consistency_formula(&data.modules);
    let reference_consistency_formula = build_reference_fact_consistency_formula(&data.references);
    let contract_consistency_formula = build_contract_fact_consistency_formula(&data.contracts);
    let gate_consistency_formula = build_gate_fact_consistency_formula(&data.quality_gates);
    out.push_str(&format!(
        "(検証 ref_target_exists :引数 () :式 {ref_target_formula})\n"
    ));
    out.push_str(&format!(
        "(検証 contract_doc_exists :引数 () :式 {contract_doc_formula})\n"
    ));
    out.push_str(&format!(
        "(検証 contract_impl_exists :引数 () :式 {contract_impl_formula})\n"
    ));
    out.push_str(&format!(
        "(検証 gate_source_exists :引数 () :式 {gate_source_formula})\n"
    ));
    out.push_str(&format!(
        "(検証 module_artifact_consistency :引数 () :式 {module_artifact_formula})\n"
    ));
    out.push_str(&format!(
        "(検証 reference_fact_consistency :引数 () :式 {reference_consistency_formula})\n"
    ));
    out.push_str(&format!(
        "(検証 contract_fact_consistency :引数 () :式 {contract_consistency_formula})\n"
    ));
    out.push_str(&format!(
        "(検証 gate_fact_consistency :引数 () :式 {gate_consistency_formula})\n\n"
    ));

    out.push_str(&format!(
        "(プロジェクト :名前 {} :概要 {})\n",
        quote_atom(&data.project.name),
        quote_atom(&data.project.summary)
    ));

    for module in &data.modules {
        out.push_str(&format!(
            "(モジュール :名前 {} :パス {} :カテゴリ {})\n",
            quote_atom(&module.name),
            quote_atom(&module.path),
            module.category
        ));
    }

    for reference in &data.references {
        out.push_str(&format!(
            "(参照 :元 {} :先 {})\n",
            quote_atom(&reference.from),
            quote_atom(&reference.to)
        ));
    }

    for contract in &data.contracts {
        out.push_str(&format!(
            "(契約 :名前 {} :出典 {} :パス {})\n",
            quote_atom(&contract.name),
            quote_atom(&contract.source),
            quote_atom(&contract.path)
        ));
    }

    for gate in &data.quality_gates {
        out.push_str(&format!(
            "(品質ゲート :名前 {} :コマンド {} :出典 {} :必須 {})\n",
            quote_atom(&gate.name),
            quote_atom(&gate.command),
            quote_atom(&gate.source),
            if gate.required { "yes" } else { "no" }
        ));
    }

    for extra in &data.extra_exists_paths {
        out.push_str(&format!("(事実 exists :項 ({}))\n", quote_atom(extra)));
    }

    let path_values = collect_path_universe(data);
    let ident_values = collect_ident_universe(data);
    let category_values = collect_category_universe(data);

    out.push('\n');
    out.push_str(&format!(
        "(宇宙 Path :値 ({}))\n",
        path_values
            .iter()
            .map(|v| quote_atom(v))
            .collect::<Vec<_>>()
            .join(" ")
    ));
    out.push_str(&format!(
        "(宇宙 Ident :値 ({}))\n",
        ident_values
            .iter()
            .map(|v| quote_atom(v))
            .collect::<Vec<_>>()
            .join(" ")
    ));
    out.push_str(&format!(
        "(宇宙 Category :値 ({}))\n",
        category_values.join(" ")
    ));
    out.push_str("(宇宙 Flag :値 (yes no))\n");

    out
}

fn collect_path_universe(data: &PreparedData) -> Vec<String> {
    let mut values = BTreeSet::new();
    for module in &data.modules {
        values.insert(module.path.clone());
    }
    for reference in &data.references {
        values.insert(reference.from.clone());
        values.insert(reference.to.clone());
    }
    for contract in &data.contracts {
        values.insert(contract.source.clone());
        values.insert(contract.path.clone());
    }
    for gate in &data.quality_gates {
        values.insert(gate.source.clone());
    }
    for extra in &data.extra_exists_paths {
        values.insert(extra.clone());
    }
    values.into_iter().collect()
}

fn collect_ident_universe(data: &PreparedData) -> Vec<String> {
    let mut values = BTreeSet::new();
    values.insert(data.project.name.clone());
    for module in &data.modules {
        values.insert(module.name.clone());
    }
    for contract in &data.contracts {
        values.insert(contract.name.clone());
    }
    for gate in &data.quality_gates {
        values.insert(gate.name.clone());
    }
    values.into_iter().collect()
}

fn collect_category_universe(data: &PreparedData) -> Vec<String> {
    let mut values = BTreeSet::new();
    for module in &data.modules {
        values.insert(module.category.clone());
    }
    values.into_iter().collect()
}

fn quote_atom(value: &str) -> String {
    let mut escaped = String::new();
    for ch in value.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\t' => escaped.push_str("\\t"),
            '\r' => escaped.push_str("\\r"),
            _ => escaped.push(ch),
        }
    }
    format!("\"{escaped}\"")
}

fn build_ref_target_exists_formula(references: &[SelfdocLink]) -> String {
    let clauses = references.iter().map(|reference| {
        format!(
            "(not (and (ref {} {}) (not (exists {}))))",
            quote_atom(&reference.from),
            quote_atom(&reference.to),
            quote_atom(&reference.to)
        )
    });
    fold_and_formula(clauses)
}

fn build_contract_doc_exists_formula(contracts: &[SelfdocContract]) -> String {
    let clauses = contracts.iter().map(|contract| {
        format!(
            "(not (and (contract-doc {} {}) (not (exists {}))))",
            quote_atom(&contract.name),
            quote_atom(&contract.source),
            quote_atom(&contract.source)
        )
    });
    fold_and_formula(clauses)
}

fn build_contract_impl_exists_formula(contracts: &[SelfdocContract]) -> String {
    let clauses = contracts.iter().map(|contract| {
        format!(
            "(not (and (contract-impl {} {}) (not (exists {}))))",
            quote_atom(&contract.name),
            quote_atom(&contract.path),
            quote_atom(&contract.path)
        )
    });
    fold_and_formula(clauses)
}

fn build_gate_source_exists_formula(gates: &[SelfdocGate]) -> String {
    let clauses = gates.iter().map(|gate| {
        format!(
            "(not (and (gate-source {} {}) (not (exists {}))))",
            quote_atom(&gate.name),
            quote_atom(&gate.source),
            quote_atom(&gate.source)
        )
    });
    fold_and_formula(clauses)
}

fn build_module_artifact_consistency_formula(modules: &[SelfdocModule]) -> String {
    let clauses = modules.iter().map(|module| {
        format!(
            "(not (and (sd-module {} {} {}) (not (artifact {} {}))))",
            quote_atom(&module.name),
            quote_atom(&module.path),
            module.category,
            quote_atom(&module.path),
            module.category
        )
    });
    fold_and_formula(clauses)
}

fn build_reference_fact_consistency_formula(references: &[SelfdocLink]) -> String {
    let clauses = references.iter().map(|reference| {
        format!(
            "(not (and (sd-reference {} {}) (not (ref {} {}))))",
            quote_atom(&reference.from),
            quote_atom(&reference.to),
            quote_atom(&reference.from),
            quote_atom(&reference.to)
        )
    });
    fold_and_formula(clauses)
}

fn build_contract_fact_consistency_formula(contracts: &[SelfdocContract]) -> String {
    let clauses = contracts.iter().map(|contract| {
        format!(
            "(not (and (sd-contract {} {} {}) (not (and (contract-doc {} {}) (contract-impl {} {})))))",
            quote_atom(&contract.name),
            quote_atom(&contract.source),
            quote_atom(&contract.path),
            quote_atom(&contract.name),
            quote_atom(&contract.source),
            quote_atom(&contract.name),
            quote_atom(&contract.path)
        )
    });
    fold_and_formula(clauses)
}

fn build_gate_fact_consistency_formula(gates: &[SelfdocGate]) -> String {
    let clauses = gates.iter().map(|gate| {
        format!(
            "(not (and (sd-quality-gate {} {} {} {}) (not (and (gate-source {} {}) (gate-required {} {})))))",
            quote_atom(&gate.name),
            quote_atom(&gate.command),
            quote_atom(&gate.source),
            if gate.required { "yes" } else { "no" },
            quote_atom(&gate.name),
            quote_atom(&gate.source),
            quote_atom(&gate.name),
            if gate.required { "yes" } else { "no" }
        )
    });
    fold_and_formula(clauses)
}

fn fold_and_formula<I>(clauses: I) -> String
where
    I: IntoIterator<Item = String>,
{
    let mut clauses = clauses.into_iter().collect::<Vec<_>>();
    if clauses.is_empty() {
        return "true".to_string();
    }
    if clauses.len() == 1 {
        return clauses.remove(0);
    }
    format!("(and {})", clauses.join(" "))
}

fn decode_escaped(raw: &str) -> Result<String, String> {
    let mut out = String::new();
    let mut chars = raw.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        let Some(next) = chars.next() else {
            return Err("末尾のバックスラッシュは無効です".to_string());
        };
        match next {
            '\\' => out.push('\\'),
            '"' => out.push('"'),
            'n' => out.push('\n'),
            't' => out.push('\t'),
            'r' => out.push('\r'),
            _ => return Err(format!("未対応エスケープ: \\{next}")),
        }
    }
    Ok(out)
}

fn to_posix_path(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(part) => Some(part.to_string_lossy().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn diag(code: &'static str, message: String, source: Option<String>) -> Diagnostic {
    let mut d = Diagnostic::new(code, message, None);
    if let Some(source) = source {
        d = d.with_source(source);
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn config_validation_rejects_invalid_category() {
        let config = SelfdocConfig {
            version: 1,
            scan: ScanConfig {
                include: vec!["**".to_string()],
                exclude: vec![],
                use_gitignore: false,
            },
            classify: vec![ClassifyRuleConfig {
                category: "invalid".to_string(),
                patterns: vec!["**".to_string()],
            }],
        };
        let errs = validate_config(&config, Path::new(".dtl-selfdoc.toml"));
        assert!(errs.iter().any(|d| d.code == "E-SELFDOC-CONFIG"));
    }

    #[test]
    fn scan_paths_honors_include_exclude_and_gitignore() {
        let dir = tempdir().expect("tempdir");
        fs::write(dir.path().join("keep.txt"), "ok\n").expect("write keep");
        fs::write(dir.path().join("drop.txt"), "ng\n").expect("write drop");
        fs::write(dir.path().join(".gitignore"), "drop.txt\n").expect("write gitignore");

        let include = compile_globset(&["*.txt".to_string()], "E-TEST", Path::new("config"))
            .expect("include");
        let exclude = compile_globset(&[], "E-TEST", Path::new("config")).expect("exclude");
        let gitignore = build_gitignore(dir.path(), Path::new("config")).expect("gitignore");

        let scanned = scan_paths(dir.path(), &include, &exclude, Some(&gitignore));
        assert_eq!(scanned, vec!["keep.txt".to_string()]);
    }

    #[test]
    fn extract_references_reads_dtl_md_and_yaml() {
        let dir = tempdir().expect("tempdir");
        fs::create_dir_all(dir.path().join("docs")).expect("mkdir docs");
        fs::create_dir_all(dir.path().join(".github/workflows")).expect("mkdir workflows");

        fs::write(dir.path().join("main.dtl"), "(import \"docs/spec.dtl\")\n").expect("write dtl");
        fs::write(dir.path().join("docs/spec.dtl"), "(sort S)\n").expect("write spec");
        fs::write(
            dir.path().join("README.md"),
            "[spec](docs/spec.dtl)\n{{#include docs/spec.dtl}}\n",
        )
        .expect("write readme");
        fs::write(
            dir.path().join(".github/workflows/ci.yml"),
            "jobs:\n  q:\n    steps:\n      - uses: ./docs/spec.dtl\n        with:\n          path: docs/spec.dtl\n",
        )
        .expect("write workflow");

        let artifacts = vec![
            Artifact {
                path: "main.dtl".to_string(),
                category: FileCategory::Source,
            },
            Artifact {
                path: "README.md".to_string(),
                category: FileCategory::Doc,
            },
            Artifact {
                path: ".github/workflows/ci.yml".to_string(),
                category: FileCategory::Ci,
            },
        ];

        let extracted = extract_references(dir.path(), &artifacts);
        assert!(extracted.errors.is_empty());
        assert!(
            extracted
                .references
                .iter()
                .any(|r| r.from == "main.dtl" && r.to == "docs/spec.dtl")
        );
        assert!(
            extracted
                .references
                .iter()
                .any(|r| r.from == ".github/workflows/ci.yml" && r.to == "docs/spec.dtl")
        );
    }
}
