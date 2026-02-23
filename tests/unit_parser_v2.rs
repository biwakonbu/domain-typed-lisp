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
        (型 主体)
        (データ 顧客種別 :コンストラクタ ((法人) (個人)))
        (関係 契約締結可能 :引数 (主体 顧客種別))
        (事実 契約締結可能 :項 (山田 (法人)))
        (検証 整合性 :引数 ((u 主体)) :式 true)
        (関数 判定
          :引数 ((u 主体) (t 顧客種別))
          :戻り Bool
          :本体 true)
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
        (データ 顧客種別 ((法人) (個人)))
    "#;

    let errs = parse_program(src).expect_err("surface parse should fail");
    assert!(errs.iter().any(|d| d.code == "E-PARSE"));
    assert!(
        errs.iter()
            .any(|d| d.message.contains("data expects tagged constructors"))
    );
}
