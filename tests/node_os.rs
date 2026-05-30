mod common;
use std::path::Path;

// ── node:os 模块 ───────────────────────────────────────────────────────────────

#[test]
fn test_node_os_arch() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { arch } from "node:os";
globalThis.r = arch();"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    assert!(!r.is_empty(), "arch should not be empty");
}

#[test]
fn test_node_os_platform() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { platform } from "node:os";
globalThis.r = platform();"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    assert!(!r.is_empty());
}

#[test]
fn test_node_os_eol() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { EOL } from "node:os";
globalThis.r = EOL;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    assert!(r == "\n" || r == "\r\n");
}

#[test]
fn test_node_os_endianness() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { endianness } from "node:os";
globalThis.r = endianness();"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    assert!(
        r == "LE" || r == "BE",
        "endianness should be LE or BE; got {r}"
    );
}

#[test]
fn test_node_os_hostname() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { hostname } from "node:os";
globalThis.r = hostname();"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    assert!(!r.is_empty(), "hostname should not be empty");
}

#[test]
fn test_node_os_type() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { type } from "node:os";
globalThis.r = type();"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    assert!(!r.is_empty());
}

#[test]
fn test_node_os_release() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { release } from "node:os";
globalThis.r = release();"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    assert!(!r.is_empty());
}

#[test]
fn test_node_os_homedir() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { homedir } from "node:os";
globalThis.r = homedir();"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    assert!(r.starts_with('/'), "homedir should be absolute; got {r}");
}

#[test]
fn test_node_os_tmpdir() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { tmpdir } from "node:os";
globalThis.r = tmpdir();"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    assert!(!r.is_empty());
}

#[test]
fn test_node_os_totalmem() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { totalmem } from "node:os";
globalThis.r = totalmem();"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    let mem: f64 = r.parse().unwrap_or(0.0);
    assert!(mem > 0.0, "totalmem should be positive; got {r}");
}

#[test]
fn test_node_os_freemem() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { freemem } from "node:os";
globalThis.r = freemem();"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    let mem: f64 = r.parse().unwrap_or(-1.0);
    assert!(mem >= 0.0, "freemem should be >= 0; got {r}");
}

#[test]
fn test_node_os_uptime() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { uptime } from "node:os";
globalThis.r = uptime();"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    let secs: f64 = r.parse().unwrap_or(-1.0);
    assert!(secs >= 0.0, "uptime should be >= 0; got {r}");
}

#[test]
fn test_node_os_loadavg() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { loadavg } from "node:os";
globalThis.r = loadavg();"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    // 返回格式 "1,2,3" 或 undefined
    assert!(!r.is_empty(), "loadavg should return an array; got empty");
}

#[test]
fn test_node_os_cpus() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { cpus } from "node:os";
const arr = cpus();
globalThis.r1 = arr.length;
globalThis.r2 = arr.length > 0 ? typeof arr[0].model : "no_cpus";"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let len = rt.eval_script("globalThis.r1").unwrap();
    let count: usize = len.parse().unwrap_or(0);
    assert!(count > 0, "should have at least 1 cpu; got {len}");
    let model_type = rt.eval_script("globalThis.r2").unwrap();
    assert_eq!(model_type, "string", "cpu model should be a string");
}

#[test]
fn test_node_os_user_info() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { userInfo } from "node:os";
const info = userInfo();
globalThis.r1 = typeof info.username;
globalThis.r2 = typeof info.shell;
globalThis.r3 = typeof info.homedir;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r1").unwrap(), "string");
    assert_eq!(rt.eval_script("globalThis.r2").unwrap(), "string");
    assert_eq!(rt.eval_script("globalThis.r3").unwrap(), "string");
}

#[test]
fn test_node_os_version() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { version } from "node:os";
globalThis.r = version();"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    assert!(!r.is_empty(), "version should not be empty");
}

#[test]
fn test_node_os_machine() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { machine } from "node:os";
globalThis.r = machine();"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    assert!(!r.is_empty(), "machine should not be empty");
}

#[test]
fn test_node_os_default_import() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import os from "node:os";
globalThis.r = typeof os.platform;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "function");
}
