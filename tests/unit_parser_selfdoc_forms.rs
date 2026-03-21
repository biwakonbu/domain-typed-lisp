use dtl::parse_program;

#[test]
fn parser_accepts_selfdoc_surface_forms() {
    let src = r#"
    ; syntax: surface
    (project :name "domain-typed-lisp" :summary "自己記述")
    (module :name "README" :path "README.md" :category doc)
    (reference :from "README.md" :to "docs/language-spec.md")
    (contract :name "cli::check" :source "README.md" :path "src/main.rs")
    (quality-gate :name "ci:quality:1" :command "cargo test" :source ".github/workflows/ci.yml" :required true)
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
    (contract :name "cli::check" :source "README.md")
    "#;

    let errors = parse_program(src).expect_err("parse should fail");
    assert!(errors.iter().any(|d| d.code == "E-PARSE"));
    assert!(
        errors
            .iter()
            .any(|d| d.message.contains("contract requires :path"))
    );
}
