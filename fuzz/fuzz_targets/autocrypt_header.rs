//! Fuzz target for Autocrypt headers. Arbitrary bytes must return a key, none, or an error.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = mailune_mime::sender_autocrypt(data);
    let _ = mailune_mime::gossip_keys(data, &[]);
});
