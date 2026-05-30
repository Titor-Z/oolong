mod common;
use std::path::Path;

// ── node:events ────────────────────────────────────────────────────────────────

#[test]
fn test_node_events_default_import() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import EventEmitter from "node:events";
const ee = new EventEmitter();
globalThis.r = typeof ee.on;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "function");
}

#[test]
fn test_node_events_named_import() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { EventEmitter } from "node:events";
const ee = new EventEmitter();
globalThis.r = typeof ee.emit;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "function");
}

#[test]
fn test_node_events_on_emit() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import EventEmitter from "node:events";
const ee = new EventEmitter();
ee.on("foo", (x) => { globalThis.r = x; });
ee.emit("foo", 42);"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "42");
}

#[test]
fn test_node_events_once() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import EventEmitter from "node:events";
const ee = new EventEmitter();
let count = 0;
ee.once("foo", () => { count++; });
ee.emit("foo");
ee.emit("foo");
globalThis.r = count;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "1");
}

#[test]
fn test_node_events_off() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import EventEmitter from "node:events";
const ee = new EventEmitter();
function handler() { globalThis.r = "called"; }
ee.on("foo", handler);
ee.off("foo", handler);
ee.emit("foo");
globalThis.r = globalThis.r || "not_called";"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "not_called");
}

#[test]
fn test_node_events_remove_all_listeners() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import EventEmitter from "node:events";
const ee = new EventEmitter();
ee.on("a", () => {});
ee.on("b", () => {});
ee.removeAllListeners();
globalThis.r = ee.eventNames().length;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "0");
}

#[test]
fn test_node_events_listener_count() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import EventEmitter from "node:events";
const ee = new EventEmitter();
const fn = () => {};
ee.on("foo", fn);
ee.on("foo", fn);
globalThis.r = ee.listenerCount("foo");"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "2");
}

#[test]
fn test_node_events_event_names() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import EventEmitter from "node:events";
const ee = new EventEmitter();
ee.on("a", () => {});
ee.on("b", () => {});
const names = ee.eventNames().sort().join(",");
globalThis.r = names;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "a,b");
}

#[test]
fn test_node_events_max_listeners() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import EventEmitter from "node:events";
const ee = new EventEmitter();
ee.setMaxListeners(5);
globalThis.r1 = ee.getMaxListeners();
EventEmitter.defaultMaxListeners = 20;
const ee2 = new EventEmitter();
globalThis.r2 = ee2.getMaxListeners();"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r1").unwrap(), "5");
    assert_eq!(rt.eval_script("globalThis.r2").unwrap(), "20");
}

#[test]
fn test_node_events_prepend_listener() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import EventEmitter from "node:events";
const ee = new EventEmitter();
const order = [];
ee.on("foo", () => order.push(1));
ee.prependListener("foo", () => order.push(2));
ee.emit("foo");
globalThis.r = order.join(",");"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "2,1");
}

#[test]
fn test_node_events_new_listener_event() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import EventEmitter from "node:events";
const ee = new EventEmitter();
const events = [];
ee.on("newListener", (ev, fn) => { events.push(ev); });
ee.on("foo", () => {});
ee.on("bar", () => {});
globalThis.r = events.join(",");"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "foo,bar");
}

#[test]
fn test_node_events_emit_return_value() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import EventEmitter from "node:events";
const ee = new EventEmitter();
ee.on("foo", () => {});
globalThis.r1 = ee.emit("foo");
globalThis.r2 = ee.emit("nonexistent");"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r1").unwrap(), "true");
    assert_eq!(rt.eval_script("globalThis.r2").unwrap(), "false");
}

#[test]
fn test_node_events_static_listener_count() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { EventEmitter } from "node:events";
const ee = new EventEmitter();
ee.on("foo", () => {});
globalThis.r = EventEmitter.listenerCount(ee, "foo");"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "1");
}
