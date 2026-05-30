mod common;
use std::path::Path;
// ── node:vm ──────────────────────────────────────────────────────────────────

#[test]
fn test_node_vm_run_in_this_context() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { runInThisContext } from "node:vm";
globalThis.r = runInThisContext("1 + 2");"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "3");
}

#[test]
fn test_node_vm_run_in_new_context() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { runInNewContext } from "node:vm";
globalThis.r = runInNewContext("x + y", { x: 1, y: 2 });"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "3");
}

#[test]
fn test_node_vm_script() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { Script } from "node:vm";
var s = new Script("a + b");
globalThis.r = typeof s.runInThisContext === "function" && typeof s.runInNewContext === "function";"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

#[test]
fn test_node_vm_script_run_in_new_context() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { Script } from "node:vm";
var s = new Script("x * y");
globalThis.r = s.runInNewContext({ x: 3, y: 4 });"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "12");
}

#[test]
fn test_node_vm_compile_function() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { compileFunction } from "node:vm";
var fn = compileFunction("return a + b", ["a", "b"]);
globalThis.r = fn(3, 4);"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "7");
}

#[test]
fn test_node_vm_default_import() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import vm from "node:vm";
globalThis.r = typeof vm.runInThisContext === "function" && typeof vm.Script === "function";"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}
