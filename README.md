helpers:
- `cargo install cargo-modules`
- `cargo modules generate tree --lib --types --package zstd_lib`
- `cargo test --workspace --lib -- --nocapture `
- `cargo tarpaulin --count --line --force-clean -p zstd_lib --out html`

ToDo:
- add code coverage check
- fuzz test
 


Refactor:
In idiomatic Rust code we tend to:
Import types directly (e.g., structures, enumerations), so that we can name it directly in
constructors, pattern matching, etc. E.g., use a::MyBox; then MyBox::new().
Import the parent module of a function. E.g., use a::ab followed by ab::ab_f().


Question?:
does rust reorder operations / or the CPU ?
if: input.u8()? + (input.u8()? << 2) the order in which 
the input are triggered change the result. Do I need to use mfence ?

TODO:
- split big functions in smaller functions