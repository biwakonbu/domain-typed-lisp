# 検証計画

## テスト戦略
- unit: parser / name resolve / stratify / logic engine / typecheck
- integration: CLI 振る舞い（終了コード、診断）
- property: 論理エンジンと型検査の性質検証

## 品質ゲート
1. `cargo fmt --all -- --check`
2. `cargo clippy --workspace --all-targets --all-features -- -D warnings`
3. `cargo test --workspace --all-targets`
4. `cargo test --test integration_cli`
5. `cargo test --test property_logic`
6. `cargo llvm-cov --workspace --all-features --summary-only --fail-under-lines 80`（80%以上）

## 実施順
1. ドキュメント固定
2. テスト作成（まず失敗）
3. 実装でテストを通す
4. リファクタ
5. 品質ゲート実行

## 再現性
- Rust toolchain は安定版（本環境: rustc 1.93.0）
- CI でも同一コマンドを実行する
