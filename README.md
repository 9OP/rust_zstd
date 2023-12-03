# Rust ZSTD (net7212)

Pure rust implementation of zstd decompression algorithm: https://www.rfc-editor.org/rfc/rfc8878

### Commands:

(Display module tree:)
- `cargo modules generate tree --lib --types --package zstd_lib`

Run all tests:
- `cargo test --workspace --lib -- --nocapture `

Generate coverage report:
- `cargo tarpaulin --tests --workspace --count --line --force-clean -p zstd_lib --out html`

**ZstdLib coverage ~77%**

Decompress a file:
- `cargo run tests/fixtures/txt/mobydick.zst `

Fuzzing:
- `cargo fuzz run fuzz_decode -- -timeout=10`
