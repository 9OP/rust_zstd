# Rust ZSTD (net7212)

Pure rust implementation of zstd decompression algorithm: https://www.rfc-editor.org/rfc/rfc8878

### Commands:

(Display module tree:)
- `cargo modules generate tree --lib --types --package zstd_lib`

Run all tests:
- `cargo test --workspace -- --nocapture `

**Note:** The `corpus` (generated via decodecorpus tool) is a bit large (~ 1000 files). Feel free to remove some of them 
to accelerate the testing

Generate coverage report:
- `cargo tarpaulin --tests --workspace --count --line  --out htm`

**ZstdLib coverage ~82% yay!**

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

Decompress a file:
- `cargo run ./tests/txt/mobydick.zst --info`

Fuzzing:
- `cargo fuzz run fuzz_decode -- -timeout=10`

**Note:** Fuzzing was ran for more than 30minutes without finding any pathological input. 

You need to install [pre-commit](https://pre-commit.com/) for the following

Install pre-commit hooks:
- `pre-commit install`

Run pre-commit hooks manually:
- `pre-commit run --all-files`

### Notes:

- Clean literals.rs

#### What have been done:
So far everything, including the "optimization" with parallel decoding of the frames and parallel decoding of streams and of literals and sequences.
Only the dictionnary feature is missing.

#### What was difficult:
Understanding FSE parsing/decoding was difficult. The most difficult part was debugging the FSE code when it worked for the examples (Both Nigel's examples and the RFC cross checked examples for default sequences decoders). I had to use the EducationnalDecoder from official Zstd repo, compile and instrument it to check for any divergent step with my implementation. It was tedious, I had to check tens of thousands lines long files of debugging data to find the divergent point and then try to make sense of the different values observed and fix my code.
Hoppefully I was able to fix my implementation in the end.

#### How rust made it easy to write safe code:
It is difficult to say because the Rust compiler hardly never got into my way or prevented me to do anything. Sometimes the LSP gave me error squiggle but it was quickly fixed. I felt very confortable working with Rust because the compiler always tried to help me and the online documentation is full of examples and Q/A.
The most difficult Rust part was the parallelism part at the end of the project.

To be fair, I think that the overral architecture of the project made it easier, I would not have had such easiness without the Guide from the course.
