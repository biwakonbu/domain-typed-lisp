use dtl::parse_program;

fn expect_parse_error(src: &str, needle: &str) {
    let errs = parse_program(src).expect_err("parse should fail");
    assert!(errs.iter().any(|d| d.code == "E-PARSE"));
    assert!(errs.iter().any(|d| d.message.contains(needle)));
}

#[test]
fn parser_rejects_non_list_top_level() {
    expect_parse_error("sort", "top-level form must be a list");
}

#[test]
fn parser_rejects_empty_top_level_form() {
    expect_parse_error("()", "empty top-level form");
}

#[test]
fn parser_rejects_sort_arity() {
    expect_parse_error("(sort A B)", "sort expects exactly 1 argument");
}

#[test]
fn parser_rejects_import_arity() {
    expect_parse_error("(import a b)", "import expects exactly 1 path argument");
}

#[test]
fn parser_rejects_relation_non_list_args() {
    expect_parse_error("(relation r A)", "relation argument sorts must be a list");
}

#[test]
fn parser_rejects_fact_variable() {
    expect_parse_error(
        "(relation p (Symbol)) (fact p ?x)",
        "fact/universe cannot contain rule variables",
    );
}

#[test]
fn parser_rejects_rule_arity() {
    expect_parse_error("(rule (p ?x))", "rule expects head and body");
}

#[test]
fn parser_rejects_bad_rule_formula_atom() {
    expect_parse_error("(rule (p ?x) bad)", "rule formula atom must be 'true'");
}

#[test]
fn parser_rejects_not_arity() {
    expect_parse_error(
        "(rule (p ?x) (not (p ?x) (p ?x)))",
        "not requires exactly one operand",
    );
}

#[test]
fn parser_rejects_defn_params_non_list() {
    expect_parse_error("(defn f x Bool true)", "defn params must be a list");
}

#[test]
fn parser_rejects_bad_param_shape() {
    expect_parse_error(
        "(defn f ((x)) Bool true)",
        "parameter must contain exactly name and type",
    );
}

#[test]
fn parser_rejects_unknown_type_constructor() {
    expect_parse_error(
        "(defn f ((x Symbol)) (Unknown x Symbol) true)",
        "unknown type constructor",
    );
}

#[test]
fn parser_rejects_refine_arity() {
    expect_parse_error(
        "(defn f ((x Symbol)) (Refine y Symbol) true)",
        "Refine expects",
    );
}

#[test]
fn parser_rejects_let_shape() {
    expect_parse_error(
        "(defn f ((x Symbol)) Bool (let x x))",
        "let bindings must be a list",
    );
}

#[test]
fn parser_rejects_if_shape() {
    expect_parse_error(
        "(defn f ((x Symbol)) Bool (if x true))",
        "if expects cond, then, else",
    );
}

#[test]
fn parser_rejects_empty_expr_list() {
    expect_parse_error(
        "(defn f ((x Symbol)) Bool ())",
        "expression list cannot be empty",
    );
}
