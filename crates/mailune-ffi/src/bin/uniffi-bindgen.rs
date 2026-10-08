//! Generates Swift bindings from a built `mailune-ffi` library.
//!
//! Built from this workspace so the generator is the same uniffi release the
//! library was compiled with.

fn main() {
    uniffi::uniffi_bindgen_main()
}
