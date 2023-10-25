helpers:
`cargo install cargo-modules`
`cargo modules generate tree --lib --types --package zstd_lib`
`cargo test --workspace --lib -- --nocapture `
cargo test --workspace decoders --lib -- --nocapture `

ToDo:
- add logs (outside stdout)
- huffman insert is not keeping the tree balanced, correct that
- investigate creating a bitparser trait


Todo:
In idiomatic Rust code we tend to:
Import types directly (e.g., structures, enumerations), so that we can name it directly in
constructors, pattern matching, etc. E.g., use a::MyBox; then MyBox::new().
Import the parent module of a function. E.g., use a::ab followed by ab::ab_f().

Todo:
add code coverage
- prop test
- doc tests
- fuzzy test
- documentation