use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
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
        (sort Action)
        (relation can-access (Subject Resource Action))
        (defn can-read ((u Subject) (r Resource))
          (Refine b Bool (can-access u r read))
          (can-access u r read))
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
