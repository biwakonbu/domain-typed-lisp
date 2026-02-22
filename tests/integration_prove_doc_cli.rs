use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use serde_json::Value;
use std::fs;
use tempfile::tempdir;

#[test]
fn cli_prove_json_writes_trace_file() {
    let dir = tempdir().expect("tempdir");
    let src = dir.path().join("ok.dtl");
    let out_dir = dir.path().join("out");
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
        .stdout
        .clone();

    let value: Value = serde_json::from_slice(&output).expect("json");
    assert_eq!(value["status"], "ok");
    assert!(out_dir.join("proof-trace.json").exists());
}

#[test]
fn cli_prove_returns_error_when_obligation_fails() {
    let dir = tempdir().expect("tempdir");
    let src = dir.path().join("ng.dtl");
    fs::write(
        &src,
        r#"
        (data Subject (alice) (bob))
        (relation allowed (Subject))
        (fact allowed (alice))
        (universe Subject ((alice) (bob)))
        (assert everyone-allowed ((u Subject)) (allowed u))
        "#,
    )
    .expect("write");

    let mut cmd = cargo_bin_cmd!("dtl");
    let output = cmd
        .arg("prove")
        .arg(&src)
        .arg("--format")
        .arg("json")
        .assert()
        .failure()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    let value: Value = serde_json::from_slice(&output).expect("json");
    assert_eq!(value["status"], "error");
    assert!(
        value["proof"]["obligations"]
            .as_array()
            .expect("obligations")
            .iter()
            .any(|o| o["result"] == "failed")
    );
}

#[test]
fn cli_doc_generates_bundle_only_when_proved() {
    let dir = tempdir().expect("tempdir");
    let ok_src = dir.path().join("doc_ok.dtl");
    let ok_out = dir.path().join("doc_out_ok");
    fs::write(
        &ok_src,
        r#"
        (data Subject (alice) (bob))
        (relation allowed (Subject))
        (fact allowed (alice))
        (universe Subject ((alice) (bob)))
        (assert consistency ((u Subject)) (not (and (allowed u) (not (allowed u)))))
        "#,
    )
    .expect("write ok");

    let mut ok_cmd = cargo_bin_cmd!("dtl");
    ok_cmd
        .arg("doc")
        .arg(&ok_src)
        .arg("--out")
        .arg(&ok_out)
        .assert()
        .success();

    assert!(ok_out.join("spec.md").exists());
    assert!(ok_out.join("proof-trace.json").exists());
    assert!(ok_out.join("doc-index.json").exists());

    let ng_src = dir.path().join("doc_ng.dtl");
    let ng_out = dir.path().join("doc_out_ng");
    fs::write(
        &ng_src,
        r#"
        (data Subject (alice) (bob))
        (relation allowed (Subject))
        (fact allowed (alice))
        (universe Subject ((alice) (bob)))
        (assert everyone-allowed ((u Subject)) (allowed u))
        "#,
    )
    .expect("write ng");

    let mut ng_cmd = cargo_bin_cmd!("dtl");
    ng_cmd
        .arg("doc")
        .arg(&ng_src)
        .arg("--out")
        .arg(&ng_out)
        .assert()
        .failure();

    assert!(!ng_out.join("spec.md").exists());
}
