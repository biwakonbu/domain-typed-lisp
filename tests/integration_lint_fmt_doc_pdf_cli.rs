use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use serde_json::Value;
use std::fs;
use tempfile::tempdir;

fn nested_nat(depth: usize) -> String {
    let mut out = "(zero)".to_string();
    for _ in 0..depth {
        out = format!("(succ {out})");
    }
    out
}

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
fn cli_lint_semantic_dup_detects_assert_rule_defn_equivalence() {
    let dir = tempdir().expect("tempdir");
    let src = dir.path().join("semantic_equivalent.dtl");
    fs::write(
        &src,
        r#"
        (sort Subject)
        (relation base (Subject))
        (relation allowed (Subject))
        (fact base alice)
        (fact base bob)

        (rule (allowed ?x) (and (base ?x) true))
        (rule (allowed ?y) (base ?y))

        (assert a ((u Subject)) (and (allowed u) true))
        (assert b ((v Subject)) (allowed v))

        (defn can1 ((u Subject)) Bool (allowed u))
        (defn can2 ((x Subject)) Bool (if true (allowed x) false))

        (universe Subject (alice bob))
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
    let diags = value["diagnostics"].as_array().expect("array");
    assert!(diags.iter().any(|d| {
        d["lint_code"] == "L-DUP-MAYBE"
            && d["message"]
                .as_str()
                .unwrap_or_default()
                .contains("assert a と b")
    }));
    assert!(diags.iter().any(|d| {
        d["lint_code"] == "L-DUP-MAYBE"
            && d["message"]
                .as_str()
                .unwrap_or_default()
                .contains("rule allowed")
    }));
    assert!(diags.iter().any(|d| {
        d["lint_code"] == "L-DUP-MAYBE"
            && d["message"]
                .as_str()
                .unwrap_or_default()
                .contains("defn can1 と can2")
    }));
}

#[test]
fn cli_lint_semantic_dup_does_not_report_non_equivalent_defn() {
    let dir = tempdir().expect("tempdir");
    let src = dir.path().join("semantic_not_equivalent.dtl");
    fs::write(
        &src,
        r#"
        (sort Subject)
        (relation allowed (Subject))
        (fact allowed alice)
        (defn can1 ((u Subject)) Bool (allowed u))
        (defn can2 ((u Subject)) Bool false)
        (universe Subject (alice bob))
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
    let has_maybe = value["diagnostics"]
        .as_array()
        .map(|diags| diags.iter().any(|d| d["lint_code"] == "L-DUP-MAYBE"))
        .unwrap_or(false);
    assert!(!has_maybe);
}

#[test]
fn cli_lint_semantic_dup_confidence_scales_with_model_search() {
    let dir = tempdir().expect("tempdir");
    let src = dir.path().join("semantic_confidence_scaling.dtl");
    fs::write(
        &src,
        r#"
        (sort Subject)
        (relation p (Subject))
        (fact p a)
        (fact p b)

        (assert small_a ((u Subject)) (and (p u) true))
        (assert small_b ((x Subject)) (p x))

        (assert large_a ((u Subject) (v Subject)) (and (p u) true))
        (assert large_b ((x Subject) (y Subject)) (p x))

        (universe Subject (a b c))
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
    let diags = value["diagnostics"].as_array().expect("array");
    let small_conf = diags
        .iter()
        .find(|d| {
            d["lint_code"] == "L-DUP-MAYBE"
                && d["message"]
                    .as_str()
                    .unwrap_or_default()
                    .contains("assert small_a と small_b")
        })
        .and_then(|d| d["confidence"].as_f64())
        .expect("small confidence");
    let large_conf = diags
        .iter()
        .find(|d| {
            d["lint_code"] == "L-DUP-MAYBE"
                && d["message"]
                    .as_str()
                    .unwrap_or_default()
                    .contains("assert large_a と large_b")
        })
        .and_then(|d| d["confidence"].as_f64())
        .expect("large confidence");

    assert!(small_conf > 0.0);
    assert!(large_conf <= 0.99);
    assert!(large_conf > small_conf);
}

#[test]
fn cli_lint_semantic_dup_handles_function_typed_defn_params() {
    let dir = tempdir().expect("tempdir");
    let src = dir.path().join("semantic_fun_param_equivalent.dtl");
    fs::write(
        &src,
        r#"
        (defn passthrough_a ((f (-> (Symbol) Bool))) (-> (Symbol) Bool) f)
        (defn passthrough_b ((g (-> (Symbol) Bool))) (-> (Symbol) Bool) (if true g g))

        (universe Symbol (alice bob))
        (universe Bool (true false))
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
    let diags = value["diagnostics"].as_array().expect("array");
    assert!(diags.iter().any(|d| {
        d["lint_code"] == "L-DUP-MAYBE"
            && d["message"]
                .as_str()
                .unwrap_or_default()
                .contains("defn passthrough_a と passthrough_b")
    }));
}

#[test]
fn cli_lint_semantic_dup_reports_depth_limit_warning() {
    let dir = tempdir().expect("tempdir");
    let src = dir.path().join("semantic_depth_limit.dtl");
    let deep_nat = nested_nat(1300);
    let body = format!(
        r#"
        (data Nat (zero) (succ Nat))

        (defn loop_a ((n Nat)) Bool
          (match n
            ((zero) true)
            ((succ m) (loop_a m))))
        (defn loop_b ((n Nat)) Bool
          (match n
            ((zero) true)
            ((succ m) (loop_b m))))

        (universe Nat ({deep_nat}))
        "#
    );
    fs::write(&src, body).expect("write");

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
    let diags = value["diagnostics"].as_array().expect("array");
    assert!(diags.iter().any(|d| {
        d["lint_code"] == "L-DUP-SKIP-EVAL-DEPTH"
            && d["message"]
                .as_str()
                .unwrap_or_default()
                .contains("loop_a と loop_b")
    }));
    assert!(!diags.iter().any(|d| d["lint_code"] == "L-DUP-MAYBE"));
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
fn cli_fmt_preserves_multi_context_blocks_idempotently() {
    let dir = tempdir().expect("tempdir");
    let src = dir.path().join("fmt_multi_context.dtl");
    fs::write(
        &src,
        r#"; syntax: surface
; @context: sales
(関係 sellable :引数 (商品))
(型 商品)
(事実 sellable :項 (本))

; @context: support
(型 チケット)
(関係 open :引数 (チケット))
(事実 open :項 (T1))
"#,
    )
    .expect("write");

    let mut first_fmt = cargo_bin_cmd!("dtl");
    first_fmt.arg("fmt").arg(&src).assert().success();

    let once = fs::read_to_string(&src).expect("read once");
    assert_eq!(once.matches("; @context:").count(), 2);
    assert!(once.contains("; @context: sales\n\n(型 商品)\n"));
    assert!(once.contains("; @context: support\n\n(型 チケット)\n"));

    let mut second_fmt = cargo_bin_cmd!("dtl");
    second_fmt.arg("fmt").arg(&src).assert().success();
    let twice = fs::read_to_string(&src).expect("read twice");
    assert_eq!(once, twice);

    let mut check_cmd = cargo_bin_cmd!("dtl");
    check_cmd
        .arg("fmt")
        .arg(&src)
        .arg("--check")
        .assert()
        .success();
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
