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
fn cli_prove_json_accepts_constructor_alias() {
    let dir = tempdir().expect("tempdir");
    let src = dir.path().join("ok_alias.dtl");
    let out_dir = dir.path().join("out_alias");
    fs::write(
        &src,
        r#"
        (data Action (read) (write))
        (alias 閲覧 read)
        (relation allowed (Action))
        (fact allowed (閲覧))
        (universe Action ((read) (write)))
        (assert consistency ((a Action)) (not (and (allowed a) (not (allowed a)))))
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
fn cli_prove_reference_engine_matches_native_on_supported_input() {
    let dir = tempdir().expect("tempdir");
    let src = dir.path().join("supported.dtl");
    fs::write(
        &src,
        r#"
        (data Subject (alice) (bob))
        (relation allowed (Subject))
        (fact allowed (alice))
        (universe Subject ((alice) (bob)))
        (assert consistency ((u Subject))
          (not (and (allowed u) (not (allowed u)))))
        (defn witness ((u Subject))
          (Refine b Bool (allowed u))
          (allowed u))
        "#,
    )
    .expect("write");

    let native = {
        let mut cmd = cargo_bin_cmd!("dtl");
        let output = cmd
            .arg("prove")
            .arg(&src)
            .arg("--format")
            .arg("json")
            .arg("--engine")
            .arg("native")
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        serde_json::from_slice::<Value>(&output).expect("native json")
    };

    let reference = {
        let mut cmd = cargo_bin_cmd!("dtl");
        let output = cmd
            .arg("prove")
            .arg(&src)
            .arg("--format")
            .arg("json")
            .arg("--engine")
            .arg("reference")
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        serde_json::from_slice::<Value>(&output).expect("reference json")
    };

    assert_eq!(native["proof"], reference["proof"]);
}

#[test]
fn cli_prove_reference_engine_supports_function_typed_quantifier() {
    let dir = tempdir().expect("tempdir");
    let src = dir.path().join("function_quantifier.dtl");
    fs::write(
        &src,
        r#"
        (defn witness ((f (-> (Symbol) Bool)) (x Symbol))
          (Refine b Bool true)
          true)

        (universe Symbol (alice bob))
        (universe Bool (true false))
        "#,
    )
    .expect("write");

    let mut native_cmd = cargo_bin_cmd!("dtl");
    let native_output = native_cmd
        .arg("prove")
        .arg(&src)
        .arg("--format")
        .arg("json")
        .arg("--engine")
        .arg("native")
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let native: Value = serde_json::from_slice(&native_output).expect("native json");
    assert_eq!(native["status"], "error");
    assert!(
        native["diagnostics"]
            .as_array()
            .expect("diagnostics")
            .iter()
            .any(|diag| diag["message"]
                .as_str()
                .unwrap_or_default()
                .contains("function-typed quantified variables"))
    );

    let mut reference_cmd = cargo_bin_cmd!("dtl");
    let reference_output = reference_cmd
        .arg("prove")
        .arg(&src)
        .arg("--format")
        .arg("json")
        .arg("--engine")
        .arg("reference")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let reference: Value = serde_json::from_slice(&reference_output).expect("reference json");
    assert_eq!(reference["status"], "ok");
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
    assert_eq!(index["schema_version"], "2.0.0");
    assert_eq!(index["profile"], "standard");
    assert_eq!(index["intermediate"]["dsl"], Value::Null);

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
        "schema_version": "2.0.0",
        "profile": "standard",
        "summary": {
            "total": 1,
            "proved": 1,
            "failed": 0
        },
        "self_description": {
            "project": null,
            "modules": [],
            "references": [],
            "contracts": [],
            "quality_gates": []
        },
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
    assert_eq!(index["schema_version"], "2.0.0");
    assert_eq!(index["profile"], "standard");
    assert_eq!(index["intermediate"]["dsl"], Value::Null);
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
    assert_eq!(trace["schema_version"], "2.1.0");
    assert_eq!(trace["profile"], "standard");
    assert!(
        trace["obligations"]
            .as_array()
            .expect("obligations")
            .iter()
            .any(|o| o["id"] == "defn::契約可否" && o["result"] == "proved")
    );
}

#[test]
fn cli_prove_and_doc_support_recursive_function_sample() {
    let src = example_path("recursive_totality_ok.dtl");
    let dir = tempdir().expect("tempdir");
    let prove_out = dir.path().join("recursive_prove_out");
    let doc_out = dir.path().join("recursive_doc_out");

    let mut prove_cmd = cargo_bin_cmd!("dtl");
    let prove_stdout = prove_cmd
        .arg("prove")
        .arg(&src)
        .arg("--format")
        .arg("json")
        .arg("--out")
        .arg(&prove_out)
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    let prove_value: Value = serde_json::from_slice(&prove_stdout).expect("valid prove json");
    assert_eq!(prove_value["status"], "ok");
    assert!(
        prove_value["proof"]["obligations"]
            .as_array()
            .expect("obligations")
            .iter()
            .any(|o| o["id"] == "assert::recursion-sample-proves" && o["result"] == "proved")
    );
    assert!(prove_out.join("proof-trace.json").exists());

    let mut doc_cmd = cargo_bin_cmd!("dtl");
    doc_cmd
        .arg("doc")
        .arg(&src)
        .arg("--out")
        .arg(&doc_out)
        .arg("--format")
        .arg("json")
        .assert()
        .success();

    assert!(doc_out.join("spec.json").exists());
    assert!(doc_out.join("proof-trace.json").exists());
    assert!(doc_out.join("doc-index.json").exists());
}
