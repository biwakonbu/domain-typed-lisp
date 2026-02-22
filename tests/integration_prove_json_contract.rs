use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

fn fixture_path(file: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("prove")
        .join(file)
}

fn read_fixture_json(file: &str) -> Value {
    serde_json::from_slice(&fs::read(fixture_path(file)).expect("read fixture json"))
        .expect("valid fixture json")
}

#[test]
fn prove_json_success_contract_is_stable() {
    let src = fixture_path("ok.dtl");
    let expected = read_fixture_json("ok.stdout.json");

    let mut cmd = cargo_bin_cmd!("dtl");
    let output = cmd
        .arg("prove")
        .arg(&src)
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
    let actual: Value = serde_json::from_slice(&output.stdout).expect("valid json output");
    assert_eq!(actual, expected);
}

#[test]
fn prove_json_failure_contract_is_stable() {
    let src = fixture_path("ng.dtl");
    let expected = read_fixture_json("ng.stdout.json");

    let mut cmd = cargo_bin_cmd!("dtl");
    let output = cmd
        .arg("prove")
        .arg(&src)
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
    let actual: Value = serde_json::from_slice(&output.stdout).expect("valid json output");
    assert_eq!(actual, expected);
}

#[test]
fn prove_json_out_trace_contract_is_stable() {
    let src = fixture_path("ok.dtl");
    let expected_stdout = read_fixture_json("ok.stdout.json");
    let expected_trace = read_fixture_json("ok.proof-trace.json");
    let temp = tempdir().expect("tempdir");
    let out_dir = temp.path().join("out");

    let mut cmd = cargo_bin_cmd!("dtl");
    let output = cmd
        .arg("prove")
        .arg(&src)
        .arg("--format")
        .arg("json")
        .arg("--out")
        .arg(&out_dir)
        .assert()
        .success()
        .get_output()
        .clone();

    let actual_stdout: Value = serde_json::from_slice(&output.stdout).expect("valid json output");
    assert_eq!(actual_stdout, expected_stdout);

    let actual_trace: Value = serde_json::from_slice(
        &fs::read(out_dir.join("proof-trace.json")).expect("read proof trace output"),
    )
    .expect("valid proof trace json");
    assert_eq!(actual_trace, expected_trace);
    assert_eq!(actual_stdout["proof"], actual_trace);
}
