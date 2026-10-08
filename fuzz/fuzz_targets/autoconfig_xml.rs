//! Fuzz target for Mozilla autoconfig XML. Arbitrary text must return a config or an error.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = str::from_utf8(data) else {
        return;
    };
    let _ = mailune_auth::parse_mozilla_xml(text);
});
