mod common;
use std::path::Path;
// ── node:timers ──────────────────────────────────────────────────────────────

#[test]
fn test_node_timers_set_timeout() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { setTimeout } from "node:timers";
globalThis.r = typeof setTimeout === "function";"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

#[test]
fn test_node_timers_set_interval() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { setInterval } from "node:timers";
globalThis.r = typeof setInterval === "function";"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

#[test]
fn test_node_timers_set_immediate() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { setImmediate } from "node:timers";
globalThis.r = typeof setImmediate === "function";"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

#[test]
fn test_node_timers_default_import() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import timers from "node:timers";
globalThis.r = typeof timers.setTimeout === "function" && typeof timers.setInterval === "function";"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

#[test]
fn test_node_timers_promises_set_timeout() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import timers from "node:timers";
var p = timers.promises.setTimeout(0, "ok");
globalThis.r = (typeof p.then === "function").toString();"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

#[test]
fn test_node_timers_promises_set_immediate() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import timers from "node:timers";
var p = timers.promises.setImmediate("done");
p.then(function(v) { globalThis.r = v; });
globalThis.r = "pending";"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let _ = rt.context.run_jobs();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "done");
}

#[test]
fn test_node_timers_clear_timeout() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { clearTimeout } from "node:timers";
globalThis.r = typeof clearTimeout === "function";"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}
