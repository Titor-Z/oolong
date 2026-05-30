mod common;
use std::path::Path;
// ── node:zlib ────────────────────────────────────────────────────────────────

#[test]
fn test_node_zlib_gzip_roundtrip() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { gzipSync, gunzipSync } from "node:zlib";
var original = new TextEncoder().encode("hello zlib");
var compressed = gzipSync(original);
var decompressed = gunzipSync(compressed);
var dec = new TextDecoder();
globalThis.r = dec.decode(decompressed);"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "hello zlib");
}

#[test]
fn test_node_zlib_deflate_roundtrip() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { deflateSync, inflateSync } from "node:zlib";
var original = new TextEncoder().encode("hello deflate");
var compressed = deflateSync(original);
var decompressed = inflateSync(compressed);
var dec = new TextDecoder();
globalThis.r = dec.decode(decompressed);"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "hello deflate");
}

#[test]
fn test_node_zlib_deflate_raw_roundtrip() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { deflateRawSync, inflateRawSync } from "node:zlib";
var original = new TextEncoder().encode("hello raw");
var compressed = deflateRawSync(original);
var decompressed = inflateRawSync(compressed);
var dec = new TextDecoder();
globalThis.r = dec.decode(decompressed);"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "hello raw");
}

#[test]
fn test_node_zlib_gzip_not_empty() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { gzipSync } from "node:zlib";
var original = new TextEncoder().encode("test");
var compressed = gzipSync(original);
globalThis.r = compressed.byteLength > 0;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

#[test]
fn test_node_zlib_default_import() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import zlib from "node:zlib";
globalThis.r = typeof zlib.gzipSync === "function" && typeof zlib.gunzipSync === "function";"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

#[test]
fn test_node_zlib_constants() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { constants } from "node:zlib";
globalThis.r = constants.Z_OK === 0 && constants.ZLIB_VERNUM === 0x12a0;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}
