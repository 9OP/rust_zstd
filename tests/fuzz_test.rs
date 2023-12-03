use zstd_lib;

/*
    Replay the pathological inputs that did broke the implementation once.


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
    let _ = zstd_lib::decode(input.to_vec(), false);
}

#[test]
fn test_fuzz_2() {
    // git log: 80f6e4acc3f1f88c329798ba3a68eaefe0a5388b
    // panicked at zstd_lib/src/parsing/forward_bit_parser.rs:35:9: attempt to subtract with overflow
    let input = [
        40, 181, 47, 253, 32, 12, 36, 39, 20, 0, 36, 24, 0, 0, 0, 0, 0, 0, 0, 233, 233,
    ];
    let _ = zstd_lib::decode(input.to_vec(), false);
}

#[test]
fn test_fuzz_3() {
    // git log: 767b5780f580d86b973051252b35e56890e08eed
    // panicked at zstd_lib/src/decoders/fse.rs:247:9: not initialized
    let input = [
        40, 181, 47, 253, 32, 12, 36, 1, 0, 0, 0, 0, 32, 40, 181, 47, 253, 32, 1, 36, 4, 253, 47,
        181, 40, 181, 47, 12, 12, 12, 12, 12, 24, 40, 130, 1,
    ];
    let _ = zstd_lib::decode(input.to_vec(), false);
}
