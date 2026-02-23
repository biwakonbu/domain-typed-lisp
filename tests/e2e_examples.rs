use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::{Value, json};
use std::path::PathBuf;

fn example_path(file: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join(file)
}

#[test]
fn e2e_example_ok_json_contract_is_stable() {
    let path = example_path("access_control_ok.dtl");
    let mut cmd = cargo_bin_cmd!("dtl");
    let output = cmd
        .arg("check")
        .arg(&path)
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .get_output()
        .clone();

    assert!(
        output.stderr.is_empty(),
        "json mode should not write stderr on success"
    );
    let actual: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    let expected = json!({
        "status": "ok",
        "report": {
            "functions_checked": 1,
            "errors": 0
        }
    });
    assert_eq!(actual, expected);
}

#[test]
fn e2e_example_ng_json_contract_is_stable() {
    let path = example_path("access_control_ng_unknown_relation.dtl");
    let mut cmd = cargo_bin_cmd!("dtl");
    let output = cmd
        .arg("check")
        .arg(&path)
        .arg("--format")
        .arg("json")
        .assert()
        .failure()
        .get_output()
        .clone();

    assert!(
        output.stderr.is_empty(),
        "json mode should not write stderr on failure"
    );
    let actual: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    let expected = json!({
        "status": "error",
            "diagnostics": [{
                "code": "E-RESOLVE",
                "message": "unknown function/relation/constructor: unknown",
                "source": path.display().to_string(),
                "hint": "sort/relation/関数名の定義漏れや重複定義を確認してください。",
                "span": {
                "start": 65,
                "end": 76,
                "line": 3,
                "column": 28
            }
        }]
    });
    assert_eq!(actual, expected);
}

#[test]
fn e2e_example_split_files_json_contract_is_stable() {
    let schema = example_path("access_control_split_schema.dtl");
    let policy = example_path("access_control_split_policy.dtl");
    let mut cmd = cargo_bin_cmd!("dtl");
    let output = cmd
        .arg("check")
        .arg(&schema)
        .arg(&policy)
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .get_output()
        .clone();

    assert!(
        output.stderr.is_empty(),
        "json mode should not write stderr on success"
    );
    let actual: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    let expected = json!({
        "status": "ok",
        "report": {
            "functions_checked": 1,
            "errors": 0
        }
    });
    assert_eq!(actual, expected);
}

#[test]
fn e2e_example_import_entry_json_contract_is_stable() {
    let entry = example_path("access_control_import_entry.dtl");
    let mut cmd = cargo_bin_cmd!("dtl");
    let output = cmd
        .arg("check")
        .arg(&entry)
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .get_output()
        .clone();

    assert!(
        output.stderr.is_empty(),
        "json mode should not write stderr on success"
    );
    let actual: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    let expected = json!({
        "status": "ok",
        "report": {
            "functions_checked": 1,
            "errors": 0
        }
    });
    assert_eq!(actual, expected);
}

#[test]
fn e2e_complex_policy_import_entry_check_and_prove_succeeds() {
    let entry = example_path("complex_policy_import_entry.dtl");

    let mut check_cmd = cargo_bin_cmd!("dtl");
    let check_output = check_cmd
        .arg("check")
        .arg(&entry)
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .get_output()
        .clone();

    assert!(
        check_output.stderr.is_empty(),
        "json mode should not write stderr on success"
    );
    let check_actual: Value = serde_json::from_slice(&check_output.stdout).expect("valid json");
    assert_eq!(check_actual["status"], "ok");
    assert_eq!(check_actual["report"]["functions_checked"], 2);

    let out_dir = std::env::temp_dir().join("dtl_e2e_complex_policy_out");
    if out_dir.exists() {
        let _ = std::fs::remove_dir_all(&out_dir);
    }

    let mut prove_cmd = cargo_bin_cmd!("dtl");
    let prove_output = prove_cmd
        .arg("prove")
        .arg(&entry)
        .arg("--format")
        .arg("json")
        .arg("--out")
        .arg(&out_dir)
        .assert()
        .success()
        .get_output()
        .clone();

    assert!(
        prove_output.stderr.is_empty(),
        "json mode should not write stderr on success"
    );
    let prove_actual: Value = serde_json::from_slice(&prove_output.stdout).expect("valid json");
    assert_eq!(prove_actual["status"], "ok");
    assert_eq!(
        prove_actual["proof"]["obligations"]
            .as_array()
            .map(|v| v.len()),
        Some(2)
    );
}

#[test]
fn e2e_semantic_dup_advanced_reports_strict_maybe_candidates() {
    let path = example_path("semantic_dup_advanced.dtl");
    let mut cmd = cargo_bin_cmd!("dtl");
    let output = cmd
        .arg("lint")
        .arg(&path)
        .arg("--format")
        .arg("json")
        .arg("--semantic-dup")
        .assert()
        .success()
        .get_output()
        .clone();

    assert!(
        output.stderr.is_empty(),
        "json mode should not write stderr on success"
    );
    let actual: Value = serde_json::from_slice(&output.stdout).expect("valid json");
    assert_eq!(actual["status"], "ok");
    let diagnostics = actual["diagnostics"].as_array().expect("array");
    assert_eq!(diagnostics.len(), 3);
    assert!(diagnostics.iter().all(|d| d["lint_code"] == "L-DUP-MAYBE"));
}

#[test]
fn e2e_recursive_nested_ok_check_and_prove_succeeds() {
    let path = example_path("recursive_nested_ok.dtl");

    let mut check_cmd = cargo_bin_cmd!("dtl");
    let check_output = check_cmd
        .arg("check")
        .arg(&path)
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .get_output()
        .clone();

    assert!(
        check_output.stderr.is_empty(),
        "json mode should not write stderr on success"
    );
    let check_actual: Value = serde_json::from_slice(&check_output.stdout).expect("valid json");
    assert_eq!(check_actual["status"], "ok");
    assert_eq!(check_actual["report"]["functions_checked"], 1);

    let out_dir = std::env::temp_dir().join("dtl_e2e_recursive_nested_out");
    if out_dir.exists() {
        let _ = std::fs::remove_dir_all(&out_dir);
    }

    let mut prove_cmd = cargo_bin_cmd!("dtl");
    let prove_output = prove_cmd
        .arg("prove")
        .arg(&path)
        .arg("--format")
        .arg("json")
        .arg("--out")
        .arg(&out_dir)
        .assert()
        .success()
        .get_output()
        .clone();

    assert!(
        prove_output.stderr.is_empty(),
        "json mode should not write stderr on success"
    );
    let prove_actual: Value = serde_json::from_slice(&prove_output.stdout).expect("valid json");
    assert_eq!(prove_actual["status"], "ok");
}
