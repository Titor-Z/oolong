mod common;
use std::path::Path;
// ── Blob ──────────────────────────────────────────────────────────────────────

#[test]
fn test_blob_constructor_string() {
    let mut rt = common::create_runtime();
    rt.eval_script(
        r#"
let b = new Blob(["hello"]);
globalThis.r = b.size;
        "#,
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "5");
}

#[test]
fn test_blob_type() {
    let mut rt = common::create_runtime();
    rt.eval_script(
        r#"
let b = new Blob(["test"], { type: "text/plain" });
globalThis.r = b.type;
        "#,
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "text/plain");
}

#[test]
fn test_blob_text() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"
let b = new Blob(["hello world"]);
b.text().then(v => { globalThis.r = v; });
        "#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    rt.eval_script("globalThis.r").ok();
    let r = rt.eval_script("globalThis.r").unwrap_or_default();
    assert_eq!(
        r, "hello world",
        "Blob.text() should resolve to 'hello world'; got {r}"
    );
}

#[test]
fn test_blob_slice() {
    let mut rt = common::create_runtime();
    rt.eval_script(
        r#"
let b = new Blob(["abcdefgh"]);
let s = b.slice(2, 5);
globalThis.r = s.size;
        "#,
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "3");
}

#[test]
fn test_blob_multiple_parts() {
    let mut rt = common::create_runtime();
    rt.eval_script(
        r#"
let b = new Blob(["abc", "def", "ghi"]);
globalThis.r = b.size;
        "#,
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "9");
}

// ── File ──────────────────────────────────────────────────────────────────────

#[test]
fn test_file_constructor() {
    let mut rt = common::create_runtime();
    rt.eval_script(
        r#"
let f = new File(["data"], "test.txt");
globalThis.r = f.name + "|" + f.size;
        "#,
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "test.txt|4");
}

#[test]
fn test_file_last_modified() {
    let mut rt = common::create_runtime();
    rt.eval_script(
        r#"
let f = new File(["x"], "x.txt", { lastModified: 1234567890000 });
globalThis.r = f.lastModified;
        "#,
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "1234567890000");
}

// ── URL ──────────────────────────────────────────────────────────────────────

#[test]
fn test_url_basic() {
    let mut rt = common::create_runtime();
    rt.eval_script(
        r#"
let u = new URL("https://example.com/path?q=1#hash");
globalThis.r = u.href + "|" + u.hostname + "|" + u.pathname + "|" + u.search + "|" + u.hash;
        "#,
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    assert!(
        r.contains("example.com"),
        "URL href should contain hostname; got {r}"
    );
    assert!(r.contains("path"), "URL should contain path; got {r}");
}

#[test]
fn test_url_relative() {
    let mut rt = common::create_runtime();
    rt.eval_script(
        r#"
let u = new URL("/foo", "https://base.com/bar/");
globalThis.r = u.href;
        "#,
    )
    .unwrap();
    assert_eq!(
        rt.eval_script("globalThis.r").unwrap(),
        "https://base.com/foo"
    );
}

// ── URLSearchParams ───────────────────────────────────────────────────────────

#[test]
fn test_url_search_params_get() {
    let mut rt = common::create_runtime();
    rt.eval_script(
        r#"
let p = new URLSearchParams("a=1&b=2&a=3");
globalThis.r = p.get("a") + "|" + p.get("b");
        "#,
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "1|2");
}

#[test]
fn test_url_search_params_get_all() {
    let mut rt = common::create_runtime();
    rt.eval_script(
        r#"
let p = new URLSearchParams("a=1&a=2&a=3");
let all = p.getAll("a");
globalThis.r = all.length + "|" + all[0] + "|" + all[1] + "|" + all[2];
        "#,
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "3|1|2|3");
}

#[test]
fn test_url_search_params_has_delete_set_append() {
    let mut rt = common::create_runtime();
    rt.eval_script(
        r#"
let p = new URLSearchParams("a=1&b=2");
p.set("a", "10");
p.append("c", "3");
p.delete("b");
globalThis.r = p.toString();
        "#,
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "a=10&c=3");
}

#[test]
fn test_url_search_params_sort() {
    let mut rt = common::create_runtime();
    rt.eval_script(
        r#"
let p = new URLSearchParams("z=1&a=2&m=3");
p.sort();
globalThis.r = p.toString();
        "#,
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "a=2&m=3&z=1");
}

#[test]
fn test_url_search_params_no_question_mark() {
    let mut rt = common::create_runtime();
    rt.eval_script(
        r#"
let p = new URLSearchParams("?a=1&b=2");
globalThis.r = p.get("a") + "|" + p.get("b");
        "#,
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "1|2");
}

// ── W3C 全局 API ─────────────────────────────────────────────────────────────

#[test]
fn test_global_atob() {
    let mut rt = common::create_runtime();
    let result = rt.eval_script("atob('SGVsbG8=')").unwrap();
    assert_eq!(result, "Hello");
}

#[test]
fn test_global_btoa() {
    let mut rt = common::create_runtime();
    let result = rt.eval_script("btoa('Hello')").unwrap();
    assert_eq!(result, "SGVsbG8=");
}

#[test]
fn test_global_atob_invalid() {
    let mut rt = common::create_runtime();
    let result = rt.eval_script(r#"try { atob("!!!") } catch(e) { "error:" + e.message }"#);
    assert!(result.unwrap().contains("error:"));
}

#[test]
fn test_global_performance_now() {
    let mut rt = common::create_runtime();
    let result = rt
        .eval_script("typeof performance.now === 'function' && performance.now() >= 0")
        .unwrap();
    assert_eq!(result, "true");
}

#[test]
#[allow(non_snake_case)]
fn test_global_performance_timeOrigin() {
    let mut rt = common::create_runtime();
    let result = rt
        .eval_script("typeof performance.timeOrigin === 'number' && performance.timeOrigin > 0")
        .unwrap();
    assert_eq!(result, "true");
}

#[test]
fn test_global_performance_mark_measure() {
    let mut rt = common::create_runtime();
    let result = rt
        .eval_script(
            r#"performance.mark("a");
performance.mark("b");
performance.measure("x", "a", "b");
var entries = performance.getEntries();
entries.length === 3 && entries[0].name === "a" && entries[2].name === "x""#,
        )
        .unwrap();
    assert_eq!(result, "true");
}

#[test]
fn test_global_performance_clear() {
    let mut rt = common::create_runtime();
    let result = rt
        .eval_script(
            r#"performance.mark("x");
performance.clearMarks("x");
performance.getEntries().length === 0"#,
        )
        .unwrap();
    assert_eq!(result, "true");
}

#[test]
fn test_global_abort_controller() {
    let mut rt = common::create_runtime();
    let result = rt
        .eval_script(
            r#"var c = new AbortController();
c.abort();
c.signal.aborted"#,
        )
        .unwrap();
    assert_eq!(result, "true");
}

#[test]
fn test_global_abort_signal_event() {
    let mut rt = common::create_runtime();
    let result = rt
        .eval_script(
            r#"var c = new AbortController();
var called = false;
c.signal.addEventListener("abort", function() { called = true; });
c.abort();
called"#,
        )
        .unwrap();
    assert_eq!(result, "true");
}

#[test]
fn test_global_performance_class_global() {
    let mut rt = common::create_runtime();
    let result = rt
        .eval_script(
            r#"typeof PerformanceMark === 'function' && typeof Performance === 'function' && typeof PerformanceEntry === 'function'"#,
        )
        .unwrap();
    assert_eq!(result, "true");
}

#[test]
fn test_import_perf_hooks_from_node() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { performance } from "node:perf_hooks";
globalThis.r = performance === globalThis.performance;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

// ── Event / EventTarget ─────────────────────────────────────────────────────

#[test]
fn test_event_constructor() {
    let mut rt = common::create_runtime();
    let result = rt
        .eval_script(
            r#"var e = new Event("click");
(e.type === "click" && e.target === undefined && e.defaultPrevented === false && e.cancelable === false && e.bubbles === false)"#,
        )
        .unwrap();
    assert_eq!(result, "true");
}

#[test]
fn test_event_constructor_with_options() {
    let mut rt = common::create_runtime();
    let result = rt
        .eval_script(
            r#"var e = new Event("custom", { bubbles: true, cancelable: true });
(e.bubbles === true && e.cancelable === true)"#,
        )
        .unwrap();
    assert_eq!(result, "true");
}

#[test]
fn test_event_prevent_default() {
    let mut rt = common::create_runtime();
    let result = rt
        .eval_script(
            r#"var e = new Event("test", { cancelable: true });
e.preventDefault();
e.defaultPrevented"#,
        )
        .unwrap();
    assert_eq!(result, "true");
}

#[test]
fn test_event_prevent_default_non_cancelable() {
    let mut rt = common::create_runtime();
    let result = rt
        .eval_script(
            r#"var e = new Event("test", { cancelable: false });
e.preventDefault();
e.defaultPrevented"#,
        )
        .unwrap();
    assert_eq!(result, "false");
}

#[test]
fn test_event_stop_propagation() {
    let mut rt = common::create_runtime();
    let result = rt
        .eval_script(
            r#"var e = new Event("test");
e.stopPropagation();
typeof e.stopPropagation === 'function'"#,
        )
        .unwrap();
    assert_eq!(result, "true");
}

#[test]
fn test_event_target_constructor() {
    let mut rt = common::create_runtime();
    let result = rt
        .eval_script(
            r#"var t = new EventTarget();
typeof t.addEventListener === 'function' && typeof t.removeEventListener === 'function' && typeof t.dispatchEvent === 'function'"#,
        )
        .unwrap();
    assert_eq!(result, "true");
}

#[test]
fn test_event_target_dispatch() {
    let mut rt = common::create_runtime();
    let result = rt
        .eval_script(
            r#"var t = new EventTarget();
var called = false;
t.addEventListener("test", function() { called = true; });
t.dispatchEvent(new Event("test"));
called"#,
        )
        .unwrap();
    assert_eq!(result, "true");
}

#[test]
fn test_event_target_dispatch_with_data() {
    let mut rt = common::create_runtime();
    let result = rt
        .eval_script(
            r#"var t = new EventTarget();
var result = [];
t.addEventListener("foo", function(e) { result.push("foo:" + e.type); });
t.addEventListener("bar", function(e) { result.push("bar:" + e.type); });
t.dispatchEvent(new Event("foo"));
result.join(",")"#,
        )
        .unwrap();
    assert_eq!(result, "foo:foo");
}

#[test]
fn test_event_target_remove_listener() {
    let mut rt = common::create_runtime();
    let result = rt
        .eval_script(
            r#"var t = new EventTarget();
var called = false;
function handler() { called = true; }
t.addEventListener("test", handler);
t.removeEventListener("test", handler);
t.dispatchEvent(new Event("test"));
called"#,
        )
        .unwrap();
    assert_eq!(result, "false");
}

#[test]
fn test_event_target_dispatch_return_value() {
    let mut rt = common::create_runtime();
    let result = rt
        .eval_script(
            r#"var t = new EventTarget();
var r1 = t.dispatchEvent(new Event("ok", { cancelable: true }));
t.addEventListener("ok", function(e) { e.preventDefault(); });
var r2 = t.dispatchEvent(new Event("ok", { cancelable: true }));
(r1 === true && r2 === false)"#,
        )
        .unwrap();
    assert_eq!(result, "true");
}

#[test]
fn test_event_target_non_callable_listener_ignored() {
    let mut rt = common::create_runtime();
    let result = rt
        .eval_script(
            r#"var t = new EventTarget();
t.addEventListener("test", "not a function");
t.dispatchEvent(new Event("test"));
true"#,
        )
        .unwrap();
    assert_eq!(result, "true");
}

#[test]
fn test_event_target_multiple_events() {
    let mut rt = common::create_runtime();
    let result = rt
        .eval_script(
            r#"var t = new EventTarget();
var count = 0;
t.addEventListener("a", function() { count++; });
t.addEventListener("b", function() { count++; });
t.dispatchEvent(new Event("a"));
t.dispatchEvent(new Event("b"));
t.dispatchEvent(new Event("a"));
count"#,
        )
        .unwrap();
    assert_eq!(result, "3");
}
