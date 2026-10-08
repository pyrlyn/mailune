//! Fuzz target for the search query parser. Arbitrary text must return a query or an error.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = str::from_utf8(data) else {
        return;
    };
    let _ = mailune_core::parse_query(text);
});
