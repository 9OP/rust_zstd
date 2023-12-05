# Rust ZSTD (net7212)

Pure rust implementation of zstd decompression algorithm: https://www.rfc-editor.org/rfc/rfc8878

### Commands:

(Display module tree:)
- `cargo modules generate tree --lib --types --package zstd_lib`

Run all tests:
- `cargo test --workspace -- --nocapture `

**Note:** The `corpus` (generated via decodedcorpus) is a bit large (~ 1000 files). Feel free to remove some of them 
to accelerate the testing

Generate coverage report:
- `cargo tarpaulin --tests --workspace --count --line --force-clean -p zstd_lib --out html`

**ZstdLib coverage ~81% yay!**

Decompress a file:
- `cargo run tests/fixtures/txt/mobydick.zst `

Fuzzing:
- `cargo fuzz run fuzz_decode -- -timeout=10 -seed_inputs=@fuzz/seed_inputs.txt`

**Note:** Fuzzing was ran for more than 30minutes without finding any pathological input. 
Fuzz use the decode corpus as inputs to guide the fuzzer.

Install pre-commit:
- `pre-commit install`