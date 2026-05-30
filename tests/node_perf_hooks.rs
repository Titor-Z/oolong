mod common;
use std::path::Path;
// ── node:perf_hooks ──────────────────────────────────────────────────────────

#[test]
fn test_node_perf_hooks_performance_now() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { performance } from "node:perf_hooks";
var n = performance.now();
globalThis.r = typeof n === "number" && n >= 0;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

#[test]
fn test_node_perf_hooks_performance_now_increasing() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { performance } from "node:perf_hooks";
var a = performance.now();
var b = performance.now();
globalThis.r = b >= a;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

#[test]
fn test_node_perf_hooks_performance_time_origin() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { performance } from "node:perf_hooks";
globalThis.r = typeof performance.timeOrigin === "number" && performance.timeOrigin > 0;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

#[test]
fn test_node_perf_hooks_performance_entry() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { PerformanceEntry } from "node:perf_hooks";
var e = new PerformanceEntry("test", "mark", 0, 10);
globalThis.r = e.name === "test" && e.entryType === "mark" && e.duration === 10;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

#[test]
fn test_node_perf_hooks_performance_mark() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { performance, PerformanceMark } from "node:perf_hooks";
var m = performance.mark("test");
globalThis.r = m instanceof PerformanceMark && m.name === "test" && m.entryType === "mark";"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

#[test]
fn test_node_perf_hooks_default_import() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import perf_hooks from "node:perf_hooks";
globalThis.r = typeof perf_hooks.performance === "object" && typeof perf_hooks.PerformanceEntry === "function";"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}
