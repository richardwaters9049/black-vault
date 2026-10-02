#![no_main]

use libfuzzer_sys::fuzz_target;

#[path = "../../src/transfer_codec.rs"]
mod transfer_codec;

fuzz_target!(|data: &[u8]| {
    let _ = transfer_codec::decode_package(data);
});
