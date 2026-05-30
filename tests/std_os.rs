mod common;

// ── os 模块 ─────────────────────────────────────────────────────────────────

#[test]
fn test_import_os_platform() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { platform } from "@std/os";
globalThis.r = platform();"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    assert!(!r.is_empty());
}

#[test]
fn test_import_os_arch() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { arch } from "@std/os";
globalThis.r = arch();"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    assert!(!r.is_empty());
}

#[test]
fn test_import_os_eol() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { EOL } from "@std/os";
globalThis.r = EOL;"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    assert!(r == "\n" || r == "\r\n");
}

#[test]
fn test_import_os_hostname() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { hostname } from "@std/os";
globalThis.r = hostname();"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    assert!(!r.is_empty(), "hostname should not be empty");
}

#[test]
fn test_import_os_type() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { type } from "@std/os";
globalThis.r = type();"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    assert!(!r.is_empty());
}

#[test]
fn test_import_os_release() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { release } from "@std/os";
globalThis.r = release();"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    assert!(!r.is_empty());
}

#[test]
fn test_import_os_homedir() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { homedir } from "@std/os";
globalThis.r = homedir();"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    assert!(r.starts_with('/'), "homedir should be absolute; got {r}");
}

#[test]
fn test_import_os_tmpdir() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { tmpdir } from "@std/os";
globalThis.r = tmpdir();"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    assert!(!r.is_empty());
}

#[test]
fn test_import_os_totalmem() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { totalmem } from "@std/os";
globalThis.r = totalmem();"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    let mem: f64 = r.parse().unwrap_or(0.0);
    assert!(mem > 0.0, "totalmem should be positive; got {r}");
}

#[test]
fn test_import_os_freemem() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { freemem } from "@std/os";
globalThis.r = freemem();"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    let mem: f64 = r.parse().unwrap_or(-1.0);
    assert!(mem >= 0.0, "freemem should be >= 0; got {r}");
}

#[test]
fn test_import_os_default_import() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import os from "@std/os";
globalThis.r = typeof os.platform;"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "function");
}

#[test]
fn test_import_os_cpus() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import os from "@std/os";
const cpus = os.cpus();
globalThis._r = JSON.stringify({ count: cpus.length, hasModel: typeof cpus[0]?.model === 'string', hasSpeed: typeof cpus[0]?.speed === 'number' });"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    let result = rt.eval_script("globalThis._r").unwrap();
    let v: serde_json::Value = serde_json::from_str(&result).unwrap();
    let count = v["count"].as_i64().unwrap();
    assert!(count >= 1, "expected at least 1 CPU, got {count}");
    assert_eq!(v["hasModel"], true);
    assert_eq!(v["hasSpeed"], true);
}

#[test]
fn test_import_os_uptime() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import os from "@std/os";
globalThis._r = os.uptime();"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    let result = rt.eval_script("globalThis._r").unwrap();
    let uptime: f64 = result.parse().unwrap();
    assert!(uptime > 0.0, "uptime should be positive, got {uptime}");
}

#[test]
fn test_import_os_loadavg() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import os from "@std/os";
globalThis._r = JSON.stringify(os.loadavg());"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    let result = rt.eval_script("globalThis._r").unwrap();
    let v: Vec<f64> = serde_json::from_str(&result).unwrap();
    assert_eq!(v.len(), 3);
    for &val in &v {
        assert!(val >= 0.0);
    }
}

#[test]
fn test_import_os_endianness() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import os from "@std/os";
globalThis._r = os.endianness();"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    let result = rt.eval_script("globalThis._r").unwrap();
    assert!(
        result == "LE" || result == "BE",
        "expected LE or BE, got {result}"
    );
}
