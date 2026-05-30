mod common;
use std::path::Path;

// ── node:util ────────────────────────────────────────────────────

#[test]
fn test_node_util_default_import() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import util from "node:util";
globalThis.r = typeof util.format;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "function");
}

#[test]
fn test_node_util_promisify() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { promisify } from "node:util";
function add(a, b, cb) { cb(null, a + b); }
const addAsync = promisify(add);
addAsync(3, 4).then(v => { globalThis.r = v; });"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "7");
}

#[test]
fn test_node_util_format() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { format } from "node:util";
globalThis.r = format("%s:%d", "hello", 42);"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "hello:42");
}

#[test]
fn test_node_util_inspect() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { inspect } from "node:util";
globalThis.r = inspect({a: 1, b: "hello"});"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let result = rt.eval_script("globalThis.r").unwrap();
    assert!(result.contains("a"));
    assert!(result.contains("hello"));
}

#[test]
fn test_node_util_types() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { types } from "node:util";
globalThis.r =
  types.isDate(new Date()) &&
  !types.isDate(42) &&
  types.isRegExp(/abc/) &&
  types.isArrayBuffer(new ArrayBuffer(8)) &&
  types.isMap(new Map()) &&
  types.isSet(new Set()) &&
  types.isNativeError(new Error()) &&
  types.isTypedArray(new Uint8Array());"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}
