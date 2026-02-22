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
