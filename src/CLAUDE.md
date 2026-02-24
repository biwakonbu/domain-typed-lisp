# CLAUDE.md (src)

この階層は `dtl` 本体実装（Rust）。

## モジュール境界
- `parser.rs` / `ast.rs`: 構文解析とAST
- `name_resolve.rs`: 名前解決
- `stratify.rs`: 層化否定検査
- `typecheck.rs` / `types.rs`: 型検査・停止性/網羅性関連
- `logic_engine.rs` / `prover.rs`: 導出・証明
- `lint.rs`: lint（重複/未使用）
- `fmt.rs`: 整形
- `diagnostics.rs`: 診断表現
- `main.rs`: CLI I/O とサブコマンド分岐

## 編集ルール
- 新しい診断コードを追加したら、`docs-site/src/reference/codes.md` と関連テストを更新する。
- JSON 出力スキーマを変更する場合は `tests/integration_*` の契約テストを必ず更新する。
- public API 変更時は `src/lib.rs` の再公開定義を同期する。

## 検証
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --lib --bins --tests`
