use dtl::parse_program;

#[test]
fn parser_accepts_selfdoc_surface_forms() {
    let src = r#"
    ; syntax: surface
    (プロジェクト :名前 "domain-typed-lisp" :概要 "自己記述")
    (モジュール :名前 "README" :パス "README.md" :カテゴリ doc)
    (参照 :元 "README.md" :先 "docs/language-spec.md")
    (契約 :名前 "cli::check" :出典 "README.md" :パス "src/main.rs")
    (品質ゲート :名前 "ci:quality:1" :コマンド "cargo test" :出典 ".github/workflows/ci.yml" :必須 true)
    "#;

    let program = parse_program(src).expect("parse should succeed");
    assert_eq!(program.facts.len(), 12);
    assert!(program.facts.iter().any(|f| f.name == "sd-project"));
    assert!(program.facts.iter().any(|f| f.name == "artifact"));
    assert!(program.facts.iter().any(|f| f.name == "ref"));
    assert!(program.facts.iter().any(|f| f.name == "contract-doc"));
    assert!(program.facts.iter().any(|f| f.name == "gate-source"));
}

#[test]
fn parser_rejects_selfdoc_form_without_required_tags() {
    let src = r#"
    ; syntax: surface
    (契約 :名前 "cli::check" :出典 "README.md")
    "#;

    let errors = parse_program(src).expect_err("parse should fail");
    assert!(errors.iter().any(|d| d.code == "E-PARSE"));
    assert!(
        errors
            .iter()
            .any(|d| d.message.contains("contract requires :パス"))
    );
}
