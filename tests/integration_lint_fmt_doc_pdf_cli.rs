use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use serde_json::Value;
use std::fs;
use tempfile::tempdir;

#[test]
fn cli_lint_reports_duplicate_fact_in_json() {
    let dir = tempdir().expect("tempdir");
    let src = dir.path().join("dup_fact.dtl");
    fs::write(
        &src,
        r#"
        (sort Subject)
        (relation allowed (Subject))
        (fact allowed alice)
        (fact allowed alice)
        "#,
    )
    .expect("write");

    let mut cmd = cargo_bin_cmd!("dtl");
    let output = cmd
        .arg("lint")
        .arg(&src)
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    let value: Value = serde_json::from_slice(&output).expect("json");
    assert_eq!(value["status"], "ok");
    assert!(
        value["diagnostics"]
            .as_array()
            .expect("array")
            .iter()
            .any(|d| d["lint_code"] == "L-DUP-EXACT" && d["severity"] == "warning")
    );
}

#[test]
fn cli_lint_deny_warnings_returns_failure() {
    let dir = tempdir().expect("tempdir");
    let src = dir.path().join("dup_fact.dtl");
    fs::write(
        &src,
        r#"
        (sort Subject)
        (relation allowed (Subject))
        (fact allowed alice)
        (fact allowed alice)
        "#,
    )
    .expect("write");

    let mut cmd = cargo_bin_cmd!("dtl");
    let output = cmd
        .arg("lint")
        .arg(&src)
        .arg("--format")
        .arg("json")
        .arg("--deny-warnings")
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    let value: Value = serde_json::from_slice(&output).expect("json");
    assert_eq!(value["status"], "error");
}

#[test]
fn cli_lint_semantic_dup_reports_universe_skip() {
    let dir = tempdir().expect("tempdir");
    let src = dir.path().join("semantic_skip.dtl");
    fs::write(
        &src,
        r#"
        (sort Subject)
        (relation p (Subject))
        (assert a ((u Subject)) (p u))
        (assert b ((x Subject)) (p x))
        "#,
    )
    .expect("write");

    let mut cmd = cargo_bin_cmd!("dtl");
    let output = cmd
        .arg("lint")
        .arg(&src)
        .arg("--format")
        .arg("json")
        .arg("--semantic-dup")
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    let value: Value = serde_json::from_slice(&output).expect("json");
    assert!(
        value["diagnostics"]
            .as_array()
            .expect("array")
            .iter()
            .any(|d| d["lint_code"] == "L-DUP-SKIP-UNIVERSE")
    );
}

#[test]
fn cli_fmt_check_and_write() {
    let dir = tempdir().expect("tempdir");
    let src = dir.path().join("fmt_target.dtl");
    fs::write(
        &src,
        r#"
        (sort Subject)
        (relation allowed (Subject))
        (fact allowed alice)
        "#,
    )
    .expect("write");

    let mut check_cmd = cargo_bin_cmd!("dtl");
    check_cmd
        .arg("fmt")
        .arg(&src)
        .arg("--check")
        .assert()
        .failure();

    let mut fmt_cmd = cargo_bin_cmd!("dtl");
    fmt_cmd.arg("fmt").arg(&src).assert().success();

    let mut check_after_cmd = cargo_bin_cmd!("dtl");
    check_after_cmd
        .arg("fmt")
        .arg(&src)
        .arg("--check")
        .assert()
        .success();

    let body = fs::read_to_string(&src).expect("read");
    assert!(body.contains("; syntax: surface"));
    assert!(body.contains("(型 Subject)"));
}

#[test]
fn cli_doc_pdf_gracefully_degrades_without_pandoc() {
    let dir = tempdir().expect("tempdir");
    let src = dir.path().join("doc_ok.dtl");
    let out = dir.path().join("out");
    fs::write(
        &src,
        r#"
        (data Subject (alice) (bob))
        (relation allowed (Subject))
        (fact allowed (alice))
        (universe Subject ((alice) (bob)))
        (assert consistency ((u Subject)) (not (and (allowed u) (not (allowed u)))))
        "#,
    )
    .expect("write");

    let mut cmd = cargo_bin_cmd!("dtl");
    cmd.arg("doc")
        .arg(&src)
        .arg("--out")
        .arg(&out)
        .arg("--format")
        .arg("markdown")
        .arg("--pdf")
        .env("PATH", "")
        .assert()
        .success();

    assert!(out.join("spec.md").exists());
    assert!(out.join("proof-trace.json").exists());
    assert!(out.join("doc-index.json").exists());

    let index: Value =
        serde_json::from_slice(&fs::read(out.join("doc-index.json")).expect("read index"))
            .expect("valid json");
    assert_eq!(index["pdf"]["requested"], Value::Bool(true));
    assert_eq!(index["pdf"]["generated"], Value::Bool(false));
}
