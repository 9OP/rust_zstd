use std::fs;
use zstd_lib;

fn read_file(path: &str) -> Vec<u8> {
    let bytes = fs::read(path).unwrap();
    bytes
}

fn decode_file(path: &str) -> Vec<u8> {
    let bytes = read_file(path);
    let decoded = zstd_lib::decode(bytes, false).unwrap();
    decoded
}

#[test]
fn test_mobydick() {
    let expected = read_file("./tests/fixtures/txt/mobydick.txt");
    let decoded = decode_file("./tests/fixtures/txt/mobydick.zst");
    assert_eq!(expected, decoded);
}

#[test]
fn test_les_miserables() {
    let expected = read_file("./tests/fixtures/txt/les_miserables.txt");
    let decoded = decode_file("./tests/fixtures/txt/les_miserables.zst");
    assert_eq!(expected, decoded);
}

#[test]
fn test_hamlet() {
    let expected = read_file("./tests/fixtures/txt/hamlet.txt");
    let decoded = decode_file("./tests/fixtures/txt/hamlet.zst");
    assert_eq!(expected, decoded);
}

#[test]
fn test_the_war_of_the_worlds() {
    let expected = read_file("./tests/fixtures/txt/the_war_of_the_worlds.txt");
    let decoded = decode_file("./tests/fixtures/txt/the_war_of_the_worlds.zst");
    assert_eq!(expected, decoded);
}
