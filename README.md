# Rust ZSTD 

Pure rust implementation of zstd decompression algorithm: https://www.rfc-editor.org/rfc/rfc8878

<br />



<br />

####  Commands:

Decompress a file:
- `cargo run ./tests/txt/mobydick.zst --info`

Run all tests:
- `cargo test --workspace -- --nocapture`

**Note:** The `corpus` (generated via [decodecorpus](https://github.com/facebook/zstd/blob/dev/tests/decodecorpus.c) tool) is a bit large (~ 1000 files). Feel free to remove some of them 
to accelerate the testing

Generate coverage report:
- `cargo tarpaulin --tests --workspace --count --line  --out html`

**ZstdLib coverage ~82%**

```
|| Tested/Total Lines:
|| src/main.rs: 0/8 +0.00%
|| zstd_lib/src/block.rs: 37/48 +0.00%
|| zstd_lib/src/decoders/alternating.rs: 18/26 +0.00%
|| zstd_lib/src/decoders/decoding_context.rs: 75/81 +0.00%
|| zstd_lib/src/decoders/fse.rs: 113/118 +0.00%
|| zstd_lib/src/decoders/huffman.rs: 97/123 +0.00%
|| zstd_lib/src/decoders/rle.rs: 8/9 +0.00%
|| zstd_lib/src/decoders/sequence.rs: 13/17 +0.00%
|| zstd_lib/src/frame.rs: 58/74 +2.70%
|| zstd_lib/src/lib.rs: 19/21 +0.00%
|| zstd_lib/src/literals.rs: 99/130 +0.00%
|| zstd_lib/src/parsing/backward_bit_parser.rs: 44/54 +0.00%
|| zstd_lib/src/parsing/forward_bit_parser.rs: 44/53 +0.00%
|| zstd_lib/src/parsing/forward_byte_parser.rs: 30/32 +0.00%
|| zstd_lib/src/sequences.rs: 144/172 +0.00%
|| 
82.54% coverage, 799/968 lines covered, +0.21% change in coverage
```

Fuzzing:
- `cargo fuzz run fuzz_decode -- -timeout=10`

Install pre-commit hooks:
- `pre-commit install`

Run pre-commit hooks manually:
- `pre-commit run --all-files`
