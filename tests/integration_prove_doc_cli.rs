use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

fn example_path(file: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join(file)
}

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
    let index: Value = serde_json::from_slice(
        &fs::read(ok_out.join("doc-index.json")).expect("read markdown doc index"),
    )
    .expect("valid markdown doc index");
    assert_eq!(index["files"], json!(["spec.md", "proof-trace.json"]));
    assert_eq!(index["status"], "ok");

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
    assert!(!ng_out.join("spec.json").exists());
}

#[test]
fn cli_doc_json_generates_json_bundle() {
    let dir = tempdir().expect("tempdir");
    let src = dir.path().join("doc_json_ok.dtl");
    let out = dir.path().join("doc_json_out");
    fs::write(
        &src,
        r#"
        (data Subject (alice) (bob))
        (data Action (read))
        (sort Resource)
        (relation can-access (Subject Resource Action))
        (fact can-access (alice) doc-1 (read))
        (universe Subject ((alice) (bob)))
        (universe Resource (doc-1))
        (universe Action ((read)))
        (assert consistency ((u Subject)) (not (and (can-access u doc-1 (read)) (not (can-access u doc-1 (read))))))
        "#,
    )
    .expect("write json case");

    let mut cmd = cargo_bin_cmd!("dtl");
    cmd.arg("doc")
        .arg(&src)
        .arg("--out")
        .arg(&out)
        .arg("--format")
        .arg("json")
        .assert()
        .success();

    assert!(out.join("spec.json").exists());
    assert!(!out.join("spec.md").exists());
    assert!(out.join("proof-trace.json").exists());
    assert!(out.join("doc-index.json").exists());

    let spec: Value =
        serde_json::from_slice(&fs::read(out.join("spec.json")).expect("read json spec"))
            .expect("valid spec json");
    let expected_spec = json!({
        "schema_version": "1.0.0",
        "sorts": [
            {"name": "Resource"}
        ],
        "data_declarations": [
            {"name": "Subject", "constructors": [
                {"name": "alice", "fields": []},
                {"name": "bob", "fields": []}
            ]},
            {"name": "Action", "constructors": [
                {"name": "read", "fields": []}
            ]}
        ],
        "relations": [
            {"name": "can-access", "arg_sorts": ["Subject", "Resource", "Action"]}
        ],
        "assertions": [
            {"name": "consistency"}
        ],
        "proof_status": [
            {"id": "assert::consistency", "kind": "assert", "result": "proved"}
        ]
    });
    assert_eq!(spec, expected_spec);

    let index: Value =
        serde_json::from_slice(&fs::read(out.join("doc-index.json")).expect("read json index"))
            .expect("valid doc index json");
    assert_eq!(index["files"], json!(["spec.json", "proof-trace.json"]));
    assert_eq!(index["status"], "ok");
}

#[test]
fn cli_doc_generates_bundle_for_japanese_example() {
    let src = example_path("customer_contract_ja.dtl");
    let dir = tempdir().expect("tempdir");
    let out = dir.path().join("ja_doc_out");

    let mut cmd = cargo_bin_cmd!("dtl");
    cmd.arg("doc")
        .arg(&src)
        .arg("--out")
        .arg(&out)
        .arg("--format")
        .arg("json")
        .assert()
        .success();

    assert!(out.join("spec.json").exists());
    assert!(out.join("proof-trace.json").exists());
    assert!(out.join("doc-index.json").exists());

    let trace: Value = serde_json::from_slice(
        &fs::read(out.join("proof-trace.json")).expect("read japanese proof trace"),
    )
    .expect("valid japanese proof trace");
    assert!(
        trace["obligations"]
            .as_array()
            .expect("obligations")
            .iter()
            .any(|o| o["id"] == "defn::契約可否" && o["result"] == "proved")
    );
}
