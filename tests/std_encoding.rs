mod common;

// ── base64 ───────────────────────────────────────────────────────────────────

#[test]
fn test_import_base64_encode() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { base64 } from "@std/encoding";
globalThis.r = base64.encode("Hello");"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "SGVsbG8=");
}

#[test]
fn test_import_base64_decode() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { base64 } from "@std/encoding";
let bytes = base64.decode("SGVsbG8=");
globalThis.r = new TextDecoder().decode(bytes);"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "Hello");
}

#[test]
fn test_import_base64_roundtrip() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { base64 } from "@std/encoding";
let encoded = base64.encode("Hello, OOLONG!");
let decoded = base64.decode(encoded);
globalThis.r = new TextDecoder().decode(decoded);"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "Hello, OOLONG!");
}

#[test]
fn test_import_base64_encode_uint8array() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { base64 } from "@std/encoding";
let data = new Uint8Array([72, 101, 108, 108, 111]);
globalThis.r = base64.encode(data);"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "SGVsbG8=");
}

#[test]
fn test_import_base64_decode_invalid() {
    let mut rt = common::create_runtime();
    let result = rt.eval_module_str(
        r#"import { base64 } from "@std/encoding";
try { base64.decode("!!!invalid!!!"); globalThis.r = "no-error"; }
catch(e) { globalThis.r = "error"; }"#,
        Some(std::path::Path::new("__t.js")),
    );
    assert!(result.is_ok());
    assert_eq!(
        rt.eval_script("globalThis.r").unwrap(),
        "error",
        "无效 base64 应该抛出错误"
    );
}

#[test]
fn test_import_base64_encode_empty() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { base64 } from "@std/encoding";
globalThis.r = base64.encode("");"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "");
}

#[test]
fn test_import_base64_named() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { base64 } from "@std/encoding";
globalThis.r = typeof base64.encode;"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "function");
}

// ── hex ──────────────────────────────────────────────────────────────────────

#[test]
fn test_import_hex_encode() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { hex } from "@std/encoding";
globalThis.r = hex.encode("Hello");"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "48656c6c6f");
}

#[test]
fn test_import_hex_decode() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { hex } from "@std/encoding";
let bytes = hex.decode("48656c6c6f");
globalThis.r = new TextDecoder().decode(bytes);"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "Hello");
}

#[test]
fn test_import_hex_roundtrip() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { hex } from "@std/encoding";
let encoded = hex.encode("OOLONG rocks!");
let decoded = hex.decode(encoded);
globalThis.r = new TextDecoder().decode(decoded);"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "OOLONG rocks!");
}

#[test]
fn test_import_hex_encode_uint8array() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { hex } from "@std/encoding";
let data = new Uint8Array([79, 79, 76, 79, 78, 71]);
globalThis.r = hex.encode(data);"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "4f4f4c4f4e47");
}

#[test]
fn test_import_hex_decode_invalid() {
    let mut rt = common::create_runtime();
    let result = rt.eval_module_str(
        r#"import { hex } from "@std/encoding";
try { hex.decode("xyz"); globalThis.r = "no-error"; }
catch(e) { globalThis.r = "error"; }"#,
        Some(std::path::Path::new("__t.js")),
    );
    assert!(result.is_ok());
    assert_eq!(
        rt.eval_script("globalThis.r").unwrap(),
        "error",
        "无效 hex 应该抛出错误"
    );
}

#[test]
fn test_import_hex_decode_odd_length() {
    let mut rt = common::create_runtime();
    let result = rt.eval_module_str(
        r#"import { hex } from "@std/encoding";
try { hex.decode("abc"); globalThis.r = "no-error"; }
catch(e) { globalThis.r = "error"; }"#,
        Some(std::path::Path::new("__t.js")),
    );
    assert!(result.is_ok());
    assert_eq!(
        rt.eval_script("globalThis.r").unwrap(),
        "error",
        "奇数长度 hex 应该抛出错误"
    );
}

#[test]
fn test_import_hex_encode_empty() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { hex } from "@std/encoding";
globalThis.r = hex.encode("");"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "");
}

#[test]
fn test_import_hex_named() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { hex } from "@std/encoding";
globalThis.r = typeof hex.decode;"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "function");
}

// ── default import ──────────────────────────────────────────────────────────

#[test]
fn test_import_default() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import enc from "@std/encoding";
globalThis.r = typeof enc.base64.encode;"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "function");
}

// ── .d.ts 类型一致性校验 ──────────────────────────────────────────

#[test]
fn test_type_consistency_std_encoding() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import enc from "@std/encoding";
globalThis._names = Object.keys(enc).sort();"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    let names = rt.eval_script("JSON.stringify(globalThis._names)").unwrap();
    assert_eq!(
        names, r#"["base64","hex"]"#,
        "@std/encoding 默认导出名与 types/std/encoding.d.ts 不一致"
    );
}
