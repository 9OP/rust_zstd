#![no_main]

use libfuzzer_sys::fuzz_target;
use zstd_lib;

fuzz_target!(|data: &[u8]| {
    zstd_lib::decode(data.to_vec(), false);
});
