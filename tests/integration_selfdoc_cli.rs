use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::Value;
use std::fs;
use tempfile::tempdir;

fn write_base_repo(dir: &std::path::Path) {
    fs::create_dir_all(dir.join("src")).expect("mkdir src");
    fs::create_dir_all(dir.join(".github/workflows")).expect("mkdir workflow");

    fs::write(
        dir.join("README.md"),
        r#"# sample

sample repository

<!-- selfdoc:cli-contracts:start -->
| subcommand | impl_path |
| --- | --- |
| check | src/main.rs |
| prove | src/main.rs |
| doc | src/main.rs |
| lint | src/main.rs |
| fmt | src/main.rs |
| selfdoc | src/main.rs |
| selfcheck | src/main.rs |
<!-- selfdoc:cli-contracts:end -->
"#,
    )
    .expect("write readme");

    fs::write(dir.join("src/main.rs"), "fn main() {}\n").expect("write main");

    fs::write(
        dir.join(".github/workflows/ci.yml"),
        r#"name: ci
on: [push]
jobs:
  quality:
    runs-on: ubuntu-latest
    steps:
      - run: cargo test
"#,
    )
    .expect("write workflow");

    fs::write(
        dir.join(".dtl-selfdoc.toml"),
        r#"version = 1

[scan]
include = ["README.md", "src/**", ".github/workflows/**", ".dtl-selfdoc.toml"]
exclude = []
use_gitignore = false

[[classify]]
category = "doc"
patterns = ["README.md"]

[[classify]]
category = "source"
patterns = ["src/**"]

[[classify]]
category = "ci"
patterns = [".github/workflows/**"]

[[classify]]
category = "config"
patterns = [".dtl-selfdoc.toml"]
"#,
    )
    .expect("write config");
}

#[test]
fn selfdoc_exits_with_code_2_when_config_missing() {
    let dir = tempdir().expect("tempdir");
    write_base_repo(dir.path());
    fs::remove_file(dir.path().join(".dtl-selfdoc.toml")).expect("remove config");

    let out = dir.path().join("out");
    let mut cmd = cargo_bin_cmd!("dtl");
    let output = cmd
        .arg("selfdoc")
        .arg("--repo")
        .arg(dir.path())
        .arg("--out")
        .arg(&out)
        .assert()
        .code(2)
        .get_output()
        .clone();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("E-SELFDOC-CONFIG"));
    assert!(stderr.contains("version = 1"));
}

#[test]
fn selfdoc_generates_bundle_and_intermediate_dsl() {
    let dir = tempdir().expect("tempdir");
    write_base_repo(dir.path());

    let out = dir.path().join("out");
    let mut cmd = cargo_bin_cmd!("dtl");
    cmd.arg("selfdoc")
        .arg("--repo")
        .arg(dir.path())
        .arg("--out")
        .arg(&out)
        .arg("--format")
        .arg("json")
        .assert()
        .success();

    assert!(out.join("selfdoc.generated.dtl").exists());
    assert!(out.join("spec.json").exists());
    assert!(out.join("proof-trace.json").exists());
    assert!(out.join("doc-index.json").exists());

    let spec: Value = serde_json::from_slice(&fs::read(out.join("spec.json")).expect("read spec"))
        .expect("valid spec");
    assert_eq!(spec["schema_version"], "2.0.0");
    assert_eq!(spec["profile"], "selfdoc");
    assert!(spec["self_description"]["project"].is_object());

    let trace: Value =
        serde_json::from_slice(&fs::read(out.join("proof-trace.json")).expect("read proof trace"))
            .expect("valid proof trace");
    assert_eq!(trace["schema_version"], "2.2.0");
    assert_eq!(trace["profile"], "selfdoc");
    assert_eq!(trace["engine"], "native");
    assert_eq!(trace["claim_coverage"]["total_claims"], 7);
    assert_eq!(trace["claim_coverage"]["proved_claims"], 7);

    let index: Value =
        serde_json::from_slice(&fs::read(out.join("doc-index.json")).expect("read index"))
            .expect("valid index");
    assert_eq!(index["schema_version"], "2.0.0");
    assert_eq!(index["profile"], "selfdoc");
    assert_eq!(index["intermediate"]["dsl"], "selfdoc.generated.dtl");
}

#[test]
fn selfdoc_reference_engine_writes_reference_trace() {
    let dir = tempdir().expect("tempdir");
    write_base_repo(dir.path());

    let out = dir.path().join("out_ref");
    let mut cmd = cargo_bin_cmd!("dtl");
    cmd.arg("selfdoc")
        .arg("--repo")
        .arg(dir.path())
        .arg("--out")
        .arg(&out)
        .arg("--format")
        .arg("json")
        .arg("--engine")
        .arg("reference")
        .assert()
        .success();

    let trace: Value =
        serde_json::from_slice(&fs::read(out.join("proof-trace.json")).expect("read proof trace"))
            .expect("valid proof trace");
    assert_eq!(trace["schema_version"], "2.2.0");
    assert_eq!(trace["profile"], "selfdoc");
    assert_eq!(trace["engine"], "reference");
    assert_eq!(trace["claim_coverage"]["total_claims"], 7);
    assert_eq!(trace["claim_coverage"]["proved_claims"], 7);
}

#[test]
fn selfdoc_rejects_plain_cli_strings_without_structured_contract_table() {
    let dir = tempdir().expect("tempdir");
    write_base_repo(dir.path());
    fs::write(
        dir.path().join("README.md"),
        r#"# sample

dtl check
dtl prove
dtl doc
dtl lint
dtl fmt
dtl selfdoc
dtl selfcheck
"#,
    )
    .expect("rewrite readme");

    let out = dir.path().join("out");
    let mut cmd = cargo_bin_cmd!("dtl");
    let output = cmd
        .arg("selfdoc")
        .arg("--repo")
        .arg(dir.path())
        .arg("--out")
        .arg(&out)
        .assert()
        .failure()
        .get_output()
        .clone();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("E-SELFDOC-CONTRACT"));
}

#[test]
fn selfdoc_fails_fast_when_reference_target_missing() {
    let dir = tempdir().expect("tempdir");
    write_base_repo(dir.path());
    fs::write(dir.path().join("broken.dtl"), "(import \"missing.dtl\")\n")
        .expect("write broken dtl");

    fs::write(
        dir.path().join(".dtl-selfdoc.toml"),
        r#"version = 1

[scan]
include = ["README.md", "src/**", ".github/workflows/**", ".dtl-selfdoc.toml", "broken.dtl"]
exclude = []
use_gitignore = false

[[classify]]
category = "doc"
patterns = ["README.md"]

[[classify]]
category = "source"
patterns = ["src/**", "broken.dtl"]

[[classify]]
category = "ci"
patterns = [".github/workflows/**"]

[[classify]]
category = "config"
patterns = [".dtl-selfdoc.toml"]
"#,
    )
    .expect("rewrite config");

    let out = dir.path().join("out");
    let mut cmd = cargo_bin_cmd!("dtl");
    let output = cmd
        .arg("selfdoc")
        .arg("--repo")
        .arg(dir.path())
        .arg("--out")
        .arg(&out)
        .assert()
        .failure()
        .get_output()
        .clone();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("E-SELFDOC-REF"));
    assert!(stderr.contains("missing.dtl"));

    assert!(!out.join("spec.json").exists());
    assert!(!out.join("spec.md").exists());
    assert!(!out.join("selfdoc.generated.dtl").exists());
}

#[test]
fn selfdoc_ignores_remote_action_output_paths() {
    let dir = tempdir().expect("tempdir");
    write_base_repo(dir.path());

    fs::write(
        dir.path().join(".github/workflows/ci.yml"),
        r#"name: ci
on: [push]
jobs:
  docs:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/upload-pages-artifact@v3
        with:
          path: docs-site/book
"#,
    )
    .expect("rewrite workflow");

    let out = dir.path().join("out");
    let mut cmd = cargo_bin_cmd!("dtl");
    cmd.arg("selfdoc")
        .arg("--repo")
        .arg(dir.path())
        .arg("--out")
        .arg(&out)
        .assert()
        .success();

    assert!(out.join("proof-trace.json").exists());
}
