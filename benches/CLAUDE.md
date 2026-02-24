# CLAUDE.md (benches)

この階層は Criterion ベンチマーク。

## 対象
- `perf_scaling.rs`

## 編集ルール
- ベンチは「相対比較可能」な設計を維持する。
- CI では smoke 実行のみなので、過剰な入力サイズ増加は避ける。
- `solve_facts/*` と `prove/*` のベンチグループ名は既存命名規約を維持する。

## 実行
- `cargo bench --bench perf_scaling -- solve_facts/fact_scaling/20 --quick --noplot`
- `cargo bench --bench perf_scaling -- solve_facts/rule_scaling/10 --quick --noplot`
- `cargo bench --bench perf_scaling -- prove/minimize_counterexample/4 --quick --noplot`
