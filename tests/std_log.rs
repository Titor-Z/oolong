mod common;

// ── Module import ─────────────────────────────────────────────────────────────

#[test]
fn test_import_log_module() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import * as log from "@std/log"; globalThis.r = typeof log;"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "object");
}

#[test]
fn test_import_logger_class() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { Logger } from "@std/log"; globalThis.r = typeof Logger;"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "function");
}

#[test]
fn test_import_loglevel() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { LogLevel } from "@std/log";
globalThis.r1 = LogLevel.DEBUG;
globalThis.r2 = LogLevel.INFO;
globalThis.r3 = LogLevel.WARN;
globalThis.r4 = LogLevel.ERROR;
globalThis.r5 = LogLevel.FATAL;"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(
        rt.eval_script("globalThis.r1")
            .unwrap()
            .parse::<i32>()
            .unwrap(),
        10
    );
    assert_eq!(
        rt.eval_script("globalThis.r2")
            .unwrap()
            .parse::<i32>()
            .unwrap(),
        20
    );
    assert_eq!(
        rt.eval_script("globalThis.r3")
            .unwrap()
            .parse::<i32>()
            .unwrap(),
        30
    );
    assert_eq!(
        rt.eval_script("globalThis.r4")
            .unwrap()
            .parse::<i32>()
            .unwrap(),
        40
    );
    assert_eq!(
        rt.eval_script("globalThis.r5")
            .unwrap()
            .parse::<i32>()
            .unwrap(),
        50
    );
}

// ── Logger constructor ───────────────────────────────────────────────────────

#[test]
fn test_logger_constructor() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { Logger } from "@std/log";
let log = new Logger("test");
globalThis.r = log.name;"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "test");
}

#[test]
fn test_logger_default_name() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { Logger } from "@std/log";
let log = new Logger();
globalThis.r = log.name;"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "default");
}

#[test]
fn test_logger_methods_exist() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { Logger } from "@std/log";
let log = new Logger("app");
globalThis.r = typeof log.debug + " " + typeof log.info + " " + typeof log.warn + " " + typeof log.error + " " + typeof log.fatal;"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(
        rt.eval_script("globalThis.r").unwrap(),
        "function function function function function"
    );
}

#[test]
fn test_logger_child_exists() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { Logger } from "@std/log";
let log = new Logger("app");
globalThis.r = typeof log.child;"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "function");
}

// ── Module-level shortcuts ───────────────────────────────────────────────────

#[test]
fn test_module_functions_exist() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import * as log from "@std/log";
globalThis.r = typeof log.debug + " " + typeof log.info + " " + typeof log.warn + " " + typeof log.error + " " + typeof log.fatal + " " + typeof log.getLogger + " " + typeof log.setup;"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(
        rt.eval_script("globalThis.r").unwrap(),
        "function function function function function function function"
    );
}

#[test]
fn test_module_level_functions_run() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import * as log from "@std/log";
// These should not throw
log.debug("test");
log.info("test");
log.warn("test");
log.error("test");
log.fatal("test");
globalThis.r = "ok";"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "ok");
}

// ── getLogger ─────────────────────────────────────────────────────────────────

#[test]
fn test_get_logger_singleton() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { getLogger } from "@std/log";
let a = getLogger("app");
let b = getLogger("app");
globalThis.r = a === b;"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

#[test]
fn test_get_logger_default_name() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { getLogger } from "@std/log";
let log = getLogger();
globalThis.r = log.name;"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "default");
}

#[test]
fn test_get_logger_different_names() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { getLogger } from "@std/log";
let a = getLogger("alice");
let b = getLogger("bob");
globalThis.r = a === b ? "same" : "different";"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "different");
}

// ── Level filtering ──────────────────────────────────────────────────────────

#[test]
fn test_logger_level_property() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { Logger, LogLevel } from "@std/log";
let log = new Logger("app", { level: LogLevel.WARN });
globalThis.r = log.level;"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(
        rt.eval_script("globalThis.r")
            .unwrap()
            .parse::<i32>()
            .unwrap(),
        30
    );
}

// ── child() ───────────────────────────────────────────────────────────────────

#[test]
fn test_child_returns_logger() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { getLogger } from "@std/log";
let parent = getLogger("parent");
let child = parent.child({ reqId: "abc" });
globalThis.r = typeof child;"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "object");
}

#[test]
fn test_child_inherits_name() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { getLogger } from "@std/log";
let parent = getLogger("parent");
let child = parent.child({ reqId: "abc" });
globalThis.r = child.name;"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "parent");
}

#[test]
fn test_child_has_methods() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { getLogger } from "@std/log";
let child = getLogger("app").child({ reqId: "abc" });
child.info("hello from child");
globalThis.r = "ok";"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "ok");
}

// ── Default export ───────────────────────────────────────────────────────────

#[test]
fn test_default_export() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import log from "@std/log";
globalThis.r = typeof log.Logger + " " + typeof log.LogLevel + " " + typeof log.getLogger + " " + typeof log.debug;"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(
        rt.eval_script("globalThis.r").unwrap(),
        "function object function function"
    );
}

// ── setup() ──────────────────────────────────────────────────────────────────

#[test]
fn test_setup_json_format() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { setup } from "@std/log";
setup({ format: "json" });
globalThis.r = "ok";"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "ok");
}

// ── Type consistency ─────────────────────────────────────────────────────────

#[test]
fn test_type_consistency_std_log() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import * as log from "@std/log";
let keys = Object.keys(log).sort();
globalThis.r = JSON.stringify(keys);"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    let result = rt.eval_script("globalThis.r").unwrap();
    assert!(result.contains("Logger"), "缺少 Logger 导出: {result}");
    assert!(result.contains("LogLevel"), "缺少 LogLevel 导出: {result}");
    assert!(
        result.contains("getLogger"),
        "缺少 getLogger 导出: {result}"
    );
    assert!(result.contains("debug"), "缺少 debug 导出: {result}");
    assert!(result.contains("info"), "缺少 info 导出: {result}");
    assert!(result.contains("warn"), "缺少 warn 导出: {result}");
    assert!(result.contains("error"), "缺少 error 导出: {result}");
    assert!(result.contains("fatal"), "缺少 fatal 导出: {result}");
    assert!(result.contains("setup"), "缺少 setup 导出: {result}");
    assert!(result.contains("default"), "缺少 default 导出: {result}");
}

#[test]
fn test_type_consistency_std_log_default() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import log from "@std/log";
let keys = Object.keys(log).sort();
globalThis.r = JSON.stringify(keys);"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    let result = rt.eval_script("globalThis.r").unwrap();
    // Default export should contain Logger, LogLevel, and shortcut functions
    assert!(result.contains("Logger"), "default 缺少 Logger: {result}");
    assert!(
        result.contains("LogLevel"),
        "default 缺少 LogLevel: {result}"
    );
    assert!(
        result.contains("getLogger"),
        "default 缺少 getLogger: {result}"
    );
    assert!(result.contains("debug"), "default 缺少 debug: {result}");
    assert!(result.contains("info"), "default 缺少 info: {result}");
    assert!(result.contains("warn"), "default 缺少 warn: {result}");
    assert!(result.contains("error"), "default 缺少 error: {result}");
    assert!(result.contains("fatal"), "default 缺少 fatal: {result}");
}
