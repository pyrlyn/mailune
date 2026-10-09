//! Convention gates: every Rust file opens with a `//!` header, and an
//! exported surface body (`#[uniffi::export]`, `#[wasm_bindgen]`) is one
//! expression.
//!
//! Fixtures prove the gate, and the same check walks the surface crates
//! that exist (`mailune-wasm` now, `mailune-ffi` once it lands). The shape
//! follows cox-ffi's forward-only test: `syn` parses the file, and a body
//! that is not a single expression statement fails.

#![allow(clippy::unwrap_used, clippy::panic)]

use std::fs;
use std::path::{Path, PathBuf};

use syn::{Attribute, Block, ImplItem, Item, Meta, Stmt};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn rust_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries {
        let Ok(entry) = entry else {
            continue;
        };
        let path = entry.path();
        if path.is_dir() {
            rust_sources(&path, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

fn has_module_header(source: &str) -> bool {
    source.starts_with("//!")
}

fn is_export(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        let path = attr.path();
        let mut segments = path.segments.iter();
        let uniffi = segments.next().is_some_and(|seg| seg.ident == "uniffi");
        let export = segments.next().is_some_and(|seg| seg.ident == "export");
        (uniffi && export && segments.next().is_none()) || path.is_ident("wasm_bindgen")
    })
}

fn is_cfg_test(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| match &attr.meta {
        Meta::List(list) => list.path.is_ident("cfg") && list.tokens.to_string().contains("test"),
        _ => false,
    })
}

/// `None` when `block` is exactly one expression statement.
fn body_problem(block: &Block) -> Option<String> {
    match block.stmts.as_slice() {
        [Stmt::Expr(_, _)] => None,
        [Stmt::Local(_)] => Some("a `let`".to_string()),
        [Stmt::Item(_)] => Some("a nested item".to_string()),
        [Stmt::Macro(_)] => Some("a macro".to_string()),
        [] => Some("an empty body".to_string()),
        stmts => Some(format!("{} statements", stmts.len())),
    }
}

fn export_violations(file: &str, items: &[Item], out: &mut Vec<String>) {
    let note = |out: &mut Vec<String>, name: &str, block: &Block| {
        if let Some(why) = body_problem(block) {
            out.push(format!("{file}: fn {name} — {why}"));
        }
    };
    for item in items {
        match item {
            Item::Fn(func) if is_export(&func.attrs) => {
                note(out, &func.sig.ident.to_string(), &func.block);
            }
            Item::Impl(imp) => {
                // `#[wasm_bindgen]` on the block exports every method in it.
                let whole = is_export(&imp.attrs);
                for method in imp.items.iter().filter_map(|item| match item {
                    ImplItem::Fn(func) if whole || is_export(&func.attrs) => Some(func),
                    _ => None,
                }) {
                    note(out, &method.sig.ident.to_string(), &method.block);
                }
            }
            Item::Mod(module) if !is_cfg_test(&module.attrs) => {
                if let Some((_, inner)) = &module.content {
                    export_violations(file, inner, out);
                }
            }
            _ => {}
        }
    }
}

fn violations_in(file: &str, source: &str) -> Vec<String> {
    let mut out = Vec::new();
    match syn::parse_file(source) {
        Ok(parsed) => export_violations(file, &parsed.items, &mut out),
        Err(err) => out.push(format!("{file}: does not parse — {err}")),
    }
    out
}

#[test]
fn every_rust_file_starts_with_a_module_header() {
    assert!(
        !has_module_header("fn main() {}\n"),
        "a file with no //! header must fail"
    );
    let root = workspace_root();
    let mut files = Vec::new();
    rust_sources(&root.join("crates"), &mut files);
    let mut missing = Vec::new();
    for path in files {
        let source = fs::read_to_string(&path).unwrap();
        if !has_module_header(&source) {
            missing.push(path.display().to_string());
        }
    }
    assert!(
        missing.is_empty(),
        "every Rust file starts with a //! header:\n{}",
        missing.join("\n")
    );
}

#[test]
fn a_single_expression_export_passes() {
    let source = "#[uniffi::export]\nfn send() { app::send() }\n";
    assert_eq!(violations_in("lib.rs", source), Vec::<String>::new());
}

#[test]
fn a_multi_statement_export_fails() {
    let source = "#[uniffi::export]\nfn send() { prepare(); commit(); }\n";
    assert_eq!(
        violations_in("lib.rs", source),
        ["lib.rs: fn send — 2 statements"]
    );
}

#[test]
fn wasm_bindgen_exports_are_checked_too() {
    let source = "#[wasm_bindgen(js_name = a)]\npub fn a() { let x = 1; x }\n\
                  #[wasm_bindgen]\nimpl S { pub fn b(&self) { one(); two() } }\n\
                  #[wasm_bindgen]\npub fn c() { core::c() }\n";
    assert_eq!(
        violations_in("lib.rs", source),
        ["lib.rs: fn a — 2 statements", "lib.rs: fn b — 2 statements"]
    );
}

#[test]
fn surface_sources_stay_forward_only_when_the_crate_exists() {
    let mut found = Vec::new();
    let mut walked = 0;
    for krate in ["mailune-ffi", "mailune-wasm"] {
        let src = workspace_root().join("crates").join(krate).join("src");
        if !src.is_dir() {
            continue;
        }
        walked += 1;
        let mut files = Vec::new();
        rust_sources(&src, &mut files);
        for path in files {
            let source = fs::read_to_string(&path).unwrap();
            let name = format!("{krate}/{}", path.file_name().unwrap().to_string_lossy());
            found.extend(violations_in(&name, &source));
        }
    }
    assert!(walked > 0, "no surface crate found to check");
    assert!(
        found.is_empty(),
        "a surface holds a multi-expression export — forward one call:\n{}",
        found.join("\n")
    );
}
