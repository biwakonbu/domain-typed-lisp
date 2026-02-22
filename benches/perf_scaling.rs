use std::time::Duration;
use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use dtl::{KnowledgeBase, parse_program, solve_facts};

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

criterion_group! {
    name = benches;
    config = Criterion::default()
        .sample_size(10)
        .measurement_time(Duration::from_millis(300));
    targets = bench_fact_scaling, bench_rule_scaling
}
criterion_main!(benches);
