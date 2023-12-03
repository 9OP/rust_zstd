use std::fs;
use zstd_lib::{self, ZstdLibError};

fn read_file(path: &str) -> Vec<u8> {
    let bytes = fs::read(path).unwrap();
    bytes
}

fn decode_file(path: &str) -> Result<Vec<u8>, ZstdLibError> {
    let bytes = read_file(path);
    let decoded = zstd_lib::decode(bytes, false);
    decoded
}

#[cfg(test)]
mod text {
    use super::*;

    #[test]
    fn test_mobydick() {
        let expected = read_file("./tests/txt/mobydick.txt");
        let decoded = decode_file("./tests/txt/mobydick.zst").unwrap();
        assert_eq!(expected, decoded);
    }

    #[test]
    fn test_les_miserables() {
        let expected = read_file("./tests/txt/les_miserables.txt");
        let decoded = decode_file("./tests/txt/les_miserables.zst").unwrap();
        assert_eq!(expected, decoded);
    }

    #[test]
    fn test_hamlet() {
        let expected = read_file("./tests/txt/hamlet.txt");
        let decoded = decode_file("./tests/txt/hamlet.zst").unwrap();
        assert_eq!(expected, decoded);
    }

    #[test]
    fn test_the_war_of_the_worlds() {
        let expected = read_file("./tests/txt/the_war_of_the_worlds.txt");
        let decoded = decode_file("./tests/txt/the_war_of_the_worlds.zst").unwrap();
        assert_eq!(expected, decoded);
    }
}

#[cfg(test)]
mod golden {
    use super::*;

    #[test]
    fn test_block_128k() {
        let expected = read_file("./tests/golden/block-128k.bin");
        let decoded = decode_file("./tests/golden/block-128k.zst").unwrap();
        assert_eq!(expected, decoded);
    }

    #[test]
    fn test_empty_block() {
        let expected = read_file("./tests/golden/empty-block.bin");
        let decoded = decode_file("./tests/golden/empty-block.zst").unwrap();
        assert_eq!(expected, decoded);
    }

    #[test]
    fn test_rle_first_block() {
        let expected = read_file("./tests/golden/rle-first-block.bin");
        let decoded = decode_file("./tests/golden/rle-first-block.zst").unwrap();
        assert_eq!(expected, decoded);
    }
}
