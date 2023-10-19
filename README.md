helpers:
`cargo install cargo-modules`
`cargo modules generate tree --lib --types --package zstd_lib`
`cargo test --workspace --lib -- --nocapture `
cargo test --workspace decoders --lib -- --nocapture `

ToDo:
- add logs (outside stdout)
- huffman insert is not keeping the tree balanced, correct that
- investigate creating a bitparser trait