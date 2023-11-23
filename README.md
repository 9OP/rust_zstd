helpers:
- `cargo install cargo-modules`
- `cargo modules generate tree --lib --types --package zstd_lib`
- `cargo test --workspace --lib -- --nocapture `
- `cargo tarpaulin --count --line --force-clean -p zstd_lib --out html`

ToDo:
- check if relevant to use usize or use u64 instead. usize is portable but tied to architecture, find counter-example
- add code coverage check
- prop test
- doc tests
- fuzzy test
- documentation
- add logs (outside stdout)
- huffman insert is not keeping the tree balanced, correct that


Refactor:
In idiomatic Rust code we tend to:
Import types directly (e.g., structures, enumerations), so that we can name it directly in
constructors, pattern matching, etc. E.g., use a::MyBox; then MyBox::new().
Import the parent module of a function. E.g., use a::ab followed by ab::ab_f().
