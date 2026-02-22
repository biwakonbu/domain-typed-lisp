use std::hint::black_box;
use std::time::Duration;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use dtl::{KnowledgeBase, has_failed_obligation, parse_program, prove_program, solve_facts};

fn bench_fact_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("solve_facts/fact_scaling");
    for fact_count in [20usize, 80, 160, 320] {
        let src = build_reachability_program(fact_count);
        let program = parse_program(&src).expect("parse");
        let kb = KnowledgeBase::from_program(&program).expect("kb");
        group.bench_with_input(BenchmarkId::from_parameter(fact_count), &kb, |b, kb| {
            b.iter(|| solve_facts(black_box(kb)).expect("solve"))
        });
    }
    group.finish();
}

fn bench_rule_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("solve_facts/rule_scaling");
    for rule_count in [10usize, 30, 60, 120] {
        let src = build_rule_chain_program(rule_count, 200);
        let program = parse_program(&src).expect("parse");
        let kb = KnowledgeBase::from_program(&program).expect("kb");
        group.bench_with_input(BenchmarkId::from_parameter(rule_count), &kb, |b, kb| {
            b.iter(|| solve_facts(black_box(kb)).expect("solve"))
        });
    }
    group.finish();
}

fn bench_counterexample_minimization_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("prove/minimize_counterexample");
    for premise_count in [4usize, 6, 8, 10] {
        let src = build_counterexample_minimization_program(premise_count);
        let program = parse_program(&src).expect("parse");
        group.bench_with_input(
            BenchmarkId::from_parameter(premise_count),
            &program,
            |b, program| {
                b.iter(|| {
                    let trace = prove_program(black_box(program)).expect("prove");
                    black_box(has_failed_obligation(&trace));
                })
            },
        );
    }
    group.finish();
}

fn build_reachability_program(edge_count: usize) -> String {
    let mut src = String::new();
    src.push_str("(sort Node)\n");
    src.push_str("(relation edge (Node Node))\n");
    src.push_str("(relation reach (Node Node))\n");
    src.push_str("(rule (reach ?x ?y) (edge ?x ?y))\n");
    src.push_str("(rule (reach ?x ?z) (and (reach ?x ?y) (edge ?y ?z)))\n");
    for i in 0..edge_count {
        src.push_str(&format!("(fact edge n{i} n{})\n", i + 1));
    }
    src
}

fn build_rule_chain_program(rule_count: usize, fact_count: usize) -> String {
    let mut src = String::new();
    src.push_str("(sort Node)\n");
    src.push_str("(relation base (Node))\n");
    for i in 1..=rule_count {
        src.push_str(&format!("(relation r{i} (Node))\n"));
    }
    for i in 0..fact_count {
        src.push_str(&format!("(fact base n{i})\n"));
    }
    src.push_str("(rule (r1 ?x) (base ?x))\n");
    for i in 2..=rule_count {
        src.push_str(&format!("(rule (r{i} ?x) (r{} ?x))\n", i - 1));
    }
    src
}

fn build_counterexample_minimization_program(premise_count: usize) -> String {
    let mut src = String::new();
    src.push_str("(data Subject (alice))\n");
    for i in 1..=premise_count {
        src.push_str(&format!("(relation p{i} (Subject))\n"));
    }
    src.push_str("(relation goal (Subject))\n");
    src.push_str("(universe Subject ((alice)))\n");
    src.push_str("(defn witness ((u Subject))\n");
    src.push_str("  (Refine b Bool (goal u))\n");
    src.push_str("  ");
    src.push_str(&nested_if_body(premise_count));
    src.push_str(")\n");
    src
}

fn nested_if_body(premise_count: usize) -> String {
    fn build(current: usize, max: usize) -> String {
        if current > max {
            return "true".to_string();
        }
        format!("(if (p{current} u) {} false)", build(current + 1, max))
    }
    build(1, premise_count)
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .sample_size(10)
        .measurement_time(Duration::from_millis(300));
    targets = bench_fact_scaling, bench_rule_scaling, bench_counterexample_minimization_scaling
}
criterion_main!(benches);
