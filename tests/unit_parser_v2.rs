use dtl::parse_program;

#[test]
fn parser_accepts_data_assert_universe_and_match() {
    let src = r#"
        (data Subject (alice) (bob))
        (universe Subject ((alice) (bob)))
        (relation allowed (Subject))
        (assert consistency ((u Subject)) (not (and (allowed u) (not (allowed u)))))

        (defn classify ((u Subject)) Bool
          (match u
            ((alice) true)
            ((bob) false)))
    "#;

    let program = parse_program(src).expect("parse should succeed");
    assert_eq!(program.data_decls.len(), 1);
    assert_eq!(program.universes.len(), 1);
    assert_eq!(program.asserts.len(), 1);
    assert_eq!(program.defns.len(), 1);
}

#[test]
fn parser_rejects_data_without_constructor() {
    let src = "(data Subject)";
    let errs = parse_program(src).expect_err("parse should fail");
    assert!(errs.iter().any(|d| d.code == "E-PARSE"));
    assert!(
        errs.iter()
            .any(|d| d.message.contains("at least one constructor"))
    );
}

#[test]
fn parser_rejects_malformed_match_arm() {
    let src = r#"
        (data Subject (alice))
        (defn f ((u Subject)) Bool
          (match u
            ((alice) true)
            ((alice))))
    "#;

    let errs = parse_program(src).expect_err("parse should fail");
    assert!(errs.iter().any(|d| d.code == "E-PARSE"));
    assert!(errs.iter().any(|d| {
        d.message
            .contains("match arm must contain exactly pattern and expression")
    }));
}

#[test]
fn parser_accepts_surface_syntax_with_tags() {
    let src = r#"
        ; syntax: surface
        (sort 主体)
        (data 顧客種別 :constructors ((法人) (個人)))
        (relation 契約締結可能 :args (主体 顧客種別))
        (fact 契約締結可能 :terms (山田 (法人)))
        (assert 整合性 :params ((u 主体)) :formula true)
        (defn 判定
          :params ((u 主体) (t 顧客種別))
          :ret Bool
          :body true)
    "#;

    let program = parse_program(src).expect("surface parse should succeed");
    assert_eq!(program.sorts.len(), 1);
    assert_eq!(program.data_decls.len(), 1);
    assert_eq!(program.relations.len(), 1);
    assert_eq!(program.facts.len(), 1);
    assert_eq!(program.asserts.len(), 1);
    assert_eq!(program.defns.len(), 1);
}

#[test]
fn parser_rejects_surface_data_without_required_tag() {
    let src = r#"
        ; syntax: surface
        (data 顧客種別 ((法人) (個人)))
    "#;

    let errs = parse_program(src).expect_err("surface parse should fail");
    assert!(errs.iter().any(|d| d.code == "E-PARSE"));
    assert!(
        errs.iter()
            .any(|d| d.message.contains("data expects tagged constructors"))
    );
}

#[test]
fn parser_rejects_deprecated_japanese_surface_head_with_migration_hint() {
    let src = r#"
        (sort Subject)
        (relation allowed (Subject))
        (事実 allowed :項 (alice))
    "#;

    let errs = parse_program(src).expect_err("deprecated japanese head should fail");
    assert!(errs.iter().any(|d| d.code == "E-PARSE"));
    assert!(
        errs.iter()
            .any(|d| d.message.contains("日本語予約語 `事実` は廃止"))
    );
}

#[test]
fn parser_accepts_syntax_auto_pragma_when_surface_is_consistent() {
    let src = r#"
        ; syntax: auto
        (sort 主体)
        (relation 契約可能 :args (主体))
        (fact 契約可能 :terms (山田))
    "#;

    let program = parse_program(src).expect("syntax:auto with consistent surface should parse");
    assert_eq!(program.sorts.len(), 1);
    assert_eq!(program.relations.len(), 1);
    assert_eq!(program.facts.len(), 1);
}

#[test]
fn parser_accepts_core_alias_declaration() {
    let src = r#"
        (data Action (read) (write))
        (alias 閲覧 read)
    "#;

    let program = parse_program(src).expect("core alias should parse");
    assert_eq!(program.aliases.len(), 1);
    assert_eq!(program.aliases[0].alias, "閲覧");
    assert_eq!(program.aliases[0].canonical, "read");
}

#[test]
fn parser_accepts_surface_alias_declaration() {
    let src = r#"
        ; syntax: surface
        (alias :alias 閲覧 :canonical read)
    "#;

    let program = parse_program(src).expect("surface alias should parse");
    assert_eq!(program.aliases.len(), 1);
    assert_eq!(program.aliases[0].alias, "閲覧");
    assert_eq!(program.aliases[0].canonical, "read");
}

#[test]
fn parser_rejects_deprecated_japanese_surface_tag_with_migration_hint() {
    let src = r#"
        ; syntax: surface
        (relation 契約可能 :引数 (主体))
    "#;

    let errs = parse_program(src).expect_err("deprecated japanese tag should fail");
    assert!(errs.iter().any(|d| d.code == "E-PARSE"));
    assert!(
        errs.iter()
            .any(|d| d.message.contains("日本語タグ `:引数` は廃止"))
    );
}
