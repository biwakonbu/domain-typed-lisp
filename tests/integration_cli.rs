use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use serde_json::Value;
use std::fs;
use tempfile::tempdir;

#[test]
fn cli_returns_zero_for_valid_program() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("ok.dtl");
    fs::write(
        &path,
        r#"
        (sort Subject)
        (sort Resource)
        (data Action (read))
        (relation can-access (Subject Resource Action))
        (defn can-read ((u Subject) (r Resource))
          (Refine b Bool (can-access u r (read)))
          (can-access u r (read)))
        "#,
    )
    .expect("write");

    let mut cmd = cargo_bin_cmd!("dtl");
    cmd.arg("check").arg(&path);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("ok"));
}

#[test]
fn cli_returns_one_for_invalid_program() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("ng.dtl");
    fs::write(
        &path,
        r#"
        (sort Subject)
        (relation p (Subject))
        (defn f ((x Subject)) Bool (unknown x))
        "#,
    )
    .expect("write");

    let mut cmd = cargo_bin_cmd!("dtl");
    cmd.arg("check").arg(&path);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("E-RESOLVE"));
}

#[test]
fn cli_returns_one_for_missing_file() {
    let mut cmd = cargo_bin_cmd!("dtl");
    cmd.arg("check")
        .arg("/tmp/non-existent-domain-typed-lisp-file.dtl");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("E-IO"));
}

#[test]
fn cli_accepts_multiple_files() {
    let dir = tempdir().expect("tempdir");
    let path1 = dir.path().join("schema.dtl");
    let path2 = dir.path().join("policy.dtl");
    fs::write(
        &path1,
        r#"
        (sort Subject)
        (sort Resource)
        (data Action (read))
        (relation can-access (Subject Resource Action))
        "#,
    )
    .expect("write");
    fs::write(
        &path2,
        r#"
        (defn can-read ((u Subject) (r Resource))
          (Refine b Bool (can-access u r (read)))
          (can-access u r (read)))
        "#,
    )
    .expect("write");

    let mut cmd = cargo_bin_cmd!("dtl");
    cmd.arg("check").arg(&path1).arg(&path2);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("ok"));
}

#[test]
fn cli_accepts_import_entry_file() {
    let dir = tempdir().expect("tempdir");
    let schema = dir.path().join("schema.dtl");
    let policy = dir.path().join("policy.dtl");
    let entry = dir.path().join("entry.dtl");
    fs::write(
        &schema,
        r#"
        (sort Subject)
        (sort Resource)
        (data Action (read))
        (relation can-access (Subject Resource Action))
        "#,
    )
    .expect("write");
    fs::write(
        &policy,
        r#"
        (defn can-read ((u Subject) (r Resource))
          (Refine b Bool (can-access u r (read)))
          (can-access u r (read)))
        "#,
    )
    .expect("write");
    fs::write(
        &entry,
        r#"
        (import "schema.dtl")
        (import "policy.dtl")
        "#,
    )
    .expect("write");

    let mut cmd = cargo_bin_cmd!("dtl");
    cmd.arg("check").arg(&entry);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("ok"));
}

#[test]
fn cli_reports_missing_import_file() {
    let dir = tempdir().expect("tempdir");
    let entry = dir.path().join("entry_missing_import.dtl");
    fs::write(
        &entry,
        r#"
        (import "missing.dtl")
        "#,
    )
    .expect("write");

    let mut cmd = cargo_bin_cmd!("dtl");
    cmd.arg("check").arg(&entry);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("E-IO"))
        .stderr(predicate::str::contains("missing.dtl"));
}

#[test]
fn cli_reports_import_cycle() {
    let dir = tempdir().expect("tempdir");
    let a = dir.path().join("a.dtl");
    let b = dir.path().join("b.dtl");
    fs::write(
        &a,
        r#"
        (import "b.dtl")
        "#,
    )
    .expect("write");
    fs::write(
        &b,
        r#"
        (import "a.dtl")
        "#,
    )
    .expect("write");

    let mut cmd = cargo_bin_cmd!("dtl");
    cmd.arg("check").arg(&a);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("E-IMPORT"));
}

#[test]
fn cli_json_output_for_success() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("ok_json.dtl");
    fs::write(
        &path,
        r#"
        (sort Subject)
        (sort Resource)
        (data Action (read))
        (relation can-access (Subject Resource Action))
        (defn can-read ((u Subject) (r Resource))
          (Refine b Bool (can-access u r (read)))
          (can-access u r (read)))
        "#,
    )
    .expect("write");

    let mut cmd = cargo_bin_cmd!("dtl");
    let output = cmd
        .arg("check")
        .arg(&path)
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let value: Value = serde_json::from_slice(&output).expect("valid json");
    assert_eq!(value["status"], "ok");
    assert_eq!(value["report"]["functions_checked"], 1);
}

#[test]
fn cli_json_output_for_failure() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("ng_json.dtl");
    fs::write(
        &path,
        r#"
        (sort Subject)
        (relation p (Subject))
        (defn f ((x Subject)) Bool (unknown x))
        "#,
    )
    .expect("write");

    let mut cmd = cargo_bin_cmd!("dtl");
    let output = cmd
        .arg("check")
        .arg(&path)
        .arg("--format")
        .arg("json")
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let value: Value = serde_json::from_slice(&output).expect("valid json");
    assert_eq!(value["status"], "error");
    assert!(
        value["diagnostics"]
            .as_array()
            .expect("diagnostics array")
            .iter()
            .any(|d| d["code"] == "E-RESOLVE")
    );
    assert!(
        value["diagnostics"]
            .as_array()
            .expect("diagnostics array")
            .iter()
            .any(|d| d["hint"].is_string())
    );
    assert!(
        value["diagnostics"]
            .as_array()
            .expect("diagnostics array")
            .iter()
            .any(|d| d["source"] == path.display().to_string().as_str())
    );
}

#[test]
fn cli_json_output_for_totality_error_has_machine_readable_fields() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("totality_ng_json.dtl");
    fs::write(
        &path,
        r#"
        (data Nat (z) (s Nat))
        (defn bad ((n Nat)) Bool
          (match n
            ((z) true)
            ((s m) (bad n))))
        "#,
    )
    .expect("write");

    let mut cmd = cargo_bin_cmd!("dtl");
    let output = cmd
        .arg("check")
        .arg(&path)
        .arg("--format")
        .arg("json")
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();
    let value: Value = serde_json::from_slice(&output).expect("valid json");
    assert_eq!(value["status"], "error");

    let total = value["diagnostics"]
        .as_array()
        .expect("diagnostics array")
        .iter()
        .find(|d| d["code"] == "E-TOTAL")
        .expect("E-TOTAL diagnostics");

    assert_eq!(total["reason"], "non_decreasing_argument");
    assert_eq!(
        total["arg_indices"]
            .as_array()
            .expect("arg_indices array")
            .iter()
            .map(|v| v.as_u64().expect("u64"))
            .collect::<Vec<_>>(),
        vec![1]
    );
}

#[test]
fn cli_json_output_for_multi_file_failure_has_per_file_source() {
    let dir = tempdir().expect("tempdir");
    let schema = dir.path().join("schema.dtl");
    let policy = dir.path().join("policy_bad.dtl");
    fs::write(
        &schema,
        r#"
        (sort Subject)
        (sort Resource)
        (data Action (read))
        (relation can-access (Subject Resource Action))
        "#,
    )
    .expect("write");
    fs::write(
        &policy,
        r#"
        (defn can-read ((u Subject) (r Resource))
          Bool
          (unknown-call u r))
        "#,
    )
    .expect("write");

    let mut cmd = cargo_bin_cmd!("dtl");
    let output = cmd
        .arg("check")
        .arg(&schema)
        .arg(&policy)
        .arg("--format")
        .arg("json")
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    let value: Value = serde_json::from_slice(&output).expect("valid json");
    assert_eq!(value["status"], "error");
    assert!(
        value["diagnostics"]
            .as_array()
            .expect("diagnostics array")
            .iter()
            .any(|d| d["code"] == "E-RESOLVE")
    );
    assert!(
        value["diagnostics"]
            .as_array()
            .expect("diagnostics array")
            .iter()
            .any(|d| d["source"] == policy.display().to_string().as_str())
    );
}

#[test]
fn cli_json_output_for_imported_failure_has_imported_source() {
    let dir = tempdir().expect("tempdir");
    let schema = dir.path().join("schema.dtl");
    let policy = dir.path().join("policy_bad.dtl");
    let entry = dir.path().join("entry.dtl");
    fs::write(
        &schema,
        r#"
        (sort Subject)
        (sort Resource)
        (data Action (read))
        (relation can-access (Subject Resource Action))
        "#,
    )
    .expect("write");
    fs::write(
        &policy,
        r#"
        (defn can-read ((u Subject) (r Resource))
          Bool
          (unknown-call u r))
        "#,
    )
    .expect("write");
    fs::write(
        &entry,
        r#"
        (import "schema.dtl")
        (import "policy_bad.dtl")
        "#,
    )
    .expect("write");

    let mut cmd = cargo_bin_cmd!("dtl");
    let output = cmd
        .arg("check")
        .arg(&entry)
        .arg("--format")
        .arg("json")
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    let value: Value = serde_json::from_slice(&output).expect("valid json");
    assert_eq!(value["status"], "error");
    assert!(
        value["diagnostics"]
            .as_array()
            .expect("diagnostics array")
            .iter()
            .any(|d| d["code"] == "E-RESOLVE")
    );
    assert!(
        value["diagnostics"]
            .as_array()
            .expect("diagnostics array")
            .iter()
            .any(|d| d["source"] == policy.display().to_string().as_str())
    );
}

#[test]
fn cli_json_output_for_missing_file_has_source() {
    let missing = "/tmp/non-existent-domain-typed-lisp-json-missing-file.dtl";
    let mut cmd = cargo_bin_cmd!("dtl");
    let output = cmd
        .arg("check")
        .arg(missing)
        .arg("--format")
        .arg("json")
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    let value: Value = serde_json::from_slice(&output).expect("valid json");
    assert_eq!(value["status"], "error");
    assert!(
        value["diagnostics"]
            .as_array()
            .expect("diagnostics array")
            .iter()
            .any(|d| d["code"] == "E-IO")
    );
    assert!(
        value["diagnostics"]
            .as_array()
            .expect("diagnostics array")
            .iter()
            .any(|d| d["source"] == missing)
    );
}

#[test]
fn cli_handles_japanese_identifiers_without_panic() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("ja_ok.dtl");
    fs::write(
        &path,
        r#"
        (sort 主体)
        (sort 契約)
        (data 顧客種別 (法人) (個人))
        (relation 契約可能 (主体 契約 顧客種別))
        (fact 契約可能 山田 基本契約 (法人))
        (defn 契約可能か ((u 主体) (k 契約) (種別 顧客種別))
          (Refine b Bool (契約可能 u k 種別))
          (契約可能 u k 種別))
        "#,
    )
    .expect("write");

    let mut cmd = cargo_bin_cmd!("dtl");
    cmd.arg("check").arg(&path);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("ok"));
}
