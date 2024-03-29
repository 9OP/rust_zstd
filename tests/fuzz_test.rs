use zstd_lib;

/*
    Replay the pathological inputs that did broke the implementation once.
    Run fuzzing:
        cargo fuzz run fuzz_decode -- -timeout=10

    test_fuzz_9 is interresting. It creates an endless spin with the AlternatingDecoder.

    Please provide:
        - The git log to replicate the bug
        - The panick message and file
*/

#[test]
fn test_fuzz_1() {
    // git log: 4a197d5be59f5ddd2f2b64c5eba9a3ce4fcb656f
    // panicked at zstd_lib/src/literals.rs:238:25: attempt to subtract with overflow
    let input = [
        40, 181, 47, 253, 32, 4, 36, 76, 3, 39, 17, 1, 26, 0, 0, 0, 0, 0, 0, 0, 255, 1, 39, 234,
        13, 65, 173, 17, 74,
    ];
    let _ = zstd_lib::decode(&input, false);
}

#[test]
fn test_fuzz_2() {
    // git log: 80f6e4acc3f1f88c329798ba3a68eaefe0a5388b
    // panicked at zstd_lib/src/parsing/forward_bit_parser.rs:35:9: attempt to subtract with overflow
    let input = [
        40, 181, 47, 253, 32, 12, 36, 39, 20, 0, 36, 24, 0, 0, 0, 0, 0, 0, 0, 233, 233,
    ];
    let _ = zstd_lib::decode(&input, false);
}

#[test]
fn test_fuzz_3() {
    // git log: 767b5780f580d86b973051252b35e56890e08eed
    // panicked at zstd_lib/src/decoders/fse.rs:247:9: not initialized
    let input = [
        40, 181, 47, 253, 32, 12, 36, 1, 0, 0, 0, 0, 32, 40, 181, 47, 253, 32, 1, 36, 4, 253, 47,
        181, 40, 181, 47, 12, 12, 12, 12, 12, 24, 40, 130, 1,
    ];
    let _ = zstd_lib::decode(&input, false);
}

#[test]
fn test_fuzz_4() {
    // git log: 4da237f49c9cd31b857fe4eacdf3ee5f09b2cf68
    // panicked at zstd_lib/src/decoders/huffman.rs:239:54: called `Result::unwrap()` on an `Err` value: TryFromIntError(())
    let input = [
        40, 181, 47, 253, 32, 59, 253, 4, 173, 74, 36, 0, 75, 40, 162, 162, 162, 162, 162, 162,
        202, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
        255, 255, 255, 255, 255, 255, 255, 255, 255, 0, 0, 0, 175, 255, 255, 255, 255, 255, 255,
        255, 255, 0, 0, 0, 0, 0, 51, 51, 191, 176, 0,
    ];
    let _ = zstd_lib::decode(&input, false);
}

#[test]
fn test_fuzz_5() {
    // git log: dacd8e1a9f43112700933b2aa8e3decb5ea47472
    // panicked at zstd_lib/src/literals.rs:233:53: attempt to subtract with overflow
    let input = [
        40, 181, 47, 253, 32, 41, 181, 0, 162, 162, 162, 0, 162, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 162, 162, 1, 0, 0, 0, 0, 0, 2, 162, 162, 162, 162, 162, 162,
        162, 162,
    ];
    let _ = zstd_lib::decode(&input, false);
}

#[test]
fn test_fuzz_6() {
    // git log: 2991a6bf1957fe67fec857386639c4f01119af95
    // panicked at zstd_lib/src/decoders/rle.rs:15:9: not implemented: initialize not supported for RLEDecoder
    let input = [
        40, 181, 47, 253, 32, 12, 36, 39, 46, 181, 0, 0, 0, 64, 32, 40, 0, 0, 0, 0, 27, 237, 115,
        115, 0, 196, 196, 196, 40, 181, 47, 253, 32, 196, 0, 196, 196,
    ];
    let _ = zstd_lib::decode(&input, false);
}

#[test]
fn test_fuzz_7() {
    // git log: 2766db59b4bdf1a64a351a62e7af2ec58fd44616
    // panicked at zstd_lib/src/decoders/huffman.rs:42:9: unexpected number of symbols
    let input = [
        40, 181, 47, 253, 32, 59, 253, 4, 173, 74, 36, 0, 75, 40, 0, 235, 235, 235, 235, 24, 20,
        20, 20, 235, 64, 203, 235, 119, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0,
        0, 0, 235, 235, 235, 235, 235, 235, 235, 235, 235, 255, 255, 255, 255, 255, 255, 255, 255,
        255, 0,
    ];
    let _ = zstd_lib::decode(&input, false);
}

#[test]
fn test_fuzz_8() {
    // git log: 5479d4972d3691885e5de1acc505263593257bd1
    // panicked at zstd_lib/src/decoders/huffman.rs:247:54:
    let input = [
        40, 181, 47, 253, 32, 59, 253, 4, 173, 74, 36, 0, 75, 40, 96, 100, 162, 45, 162, 162, 255,
        255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
        255, 255, 255, 255, 255, 255, 6, 255, 173, 74, 255, 255, 255, 255, 255, 255, 32, 12, 36,
        39, 12, 36, 20, 32, 176, 39, 20, 16, 36,
    ];
    let _ = zstd_lib::decode(&input, false);
}

#[test]
fn test_fuzz_9() {
    // git log: 2714ea9a68962186e7cf74465a1579289f9ef752
    // endless loop: zstd_lib/src/decoders/huffman.rs:253
    // root cause: did not check correctly the block compressed size in
    // zstd_lib/src/block.rs:73
    let input = [
        40, 181, 47, 253, 32, 59, 253, 4, 173, 74, 36, 0, 75, 40, 241, 255, 231, 235, 20, 20, 20,
        70, 20, 235, 0, 255, 255, 255, 26, 0, 0, 0, 16, 0, 0, 235, 235, 235, 235, 171, 235, 235,
        235, 235, 235, 235, 235, 235, 235, 235, 235, 235, 235, 71, 0, 255, 255, 1, 4, 255, 255, 8,
        255, 255, 255, 251, 40, 181, 47, 255,
    ];
    let _ = zstd_lib::decode(&input, false);
}

#[test]
fn test_fuzz_10() {
    // git log: 9e6a7fa7bc8b3fec995b3a5eb246dcbb11aad385
    // endless loop:  zstd_lib/src/decoders/huffman.rs:258
    // root cause: did not check and return error early when weights.len
    // is above 256
    let input = [
        40, 181, 47, 253, 48, 40, 181, 0, 0, 42, 0, 165, 47, 16, 16, 246, 23, 64, 0, 2, 0, 0, 0, 0,
        90, 28, 0, 255, 247, 255, 255,
    ];
    let _ = zstd_lib::decode(&input, false);
}

#[test]
fn test_fuzz_11() {
    // git log: 1cc6b2b52e0419815e9f1399e6c8ef4ebdbf94fe
    // panicked at zstd_lib/src/decoders/huffman.rs:44:9: assertion failed: widths.len() <= MAX_NUM_WEIGTHS
    let input = [
        40, 181, 47, 253, 48, 40, 181, 0, 0, 42, 0, 165, 45, 16, 0, 254, 0, 23, 255, 255, 255, 255,
        255, 255, 0, 0, 255, 255, 255, 255, 0, 0, 0, 255, 255, 247, 0, 0, 28, 12, 90, 255, 239,
        185, 0, 45,
    ];
    let _ = zstd_lib::decode(&input, false);
}
