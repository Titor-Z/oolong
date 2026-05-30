mod common;
use std::path::Path;
// ── Phase 5.5: node:module ─────────────────────────────────────────────────

#[test]
fn test_node_module_is_builtin() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { isBuiltin } from "node:module";
globalThis.r1 = isBuiltin("fs");
globalThis.r2 = isBuiltin("node:fs");
globalThis.r3 = isBuiltin("not-a-real-module");"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r1").unwrap(), "true");
    assert_eq!(rt.eval_script("globalThis.r2").unwrap(), "true");
    assert_eq!(rt.eval_script("globalThis.r3").unwrap(), "false");
}

#[test]
fn test_node_module_builtin_modules() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { builtinModules } from "node:module";
globalThis.r = Array.isArray(builtinModules) && builtinModules.length > 0;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

#[test]
fn test_node_module_create_require() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { createRequire } from "node:module";
const req = createRequire("/test/path/file.js");
globalThis.r = typeof req === "function";"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

#[test]
fn test_node_module_default_import() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import mod from "node:module";
globalThis.r = typeof mod.isBuiltin === "function" && Array.isArray(mod.builtinModules);"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

#[test]
fn test_node_module_resolve_filename() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { Module } from "node:module";
globalThis.r = typeof Module._resolveFilename === "function";"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}
