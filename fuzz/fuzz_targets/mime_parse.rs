//! Fuzz target for MIME parse. Arbitrary bytes must return a message or an error.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = mailune_mime::parse(data);
});
