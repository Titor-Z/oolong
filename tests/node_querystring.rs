mod common;
use std::path::Path;
// ── node:querystring ─────────────────────────────────────────────────────────

#[test]
fn test_node_querystring_parse() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { parse } from "node:querystring";
globalThis.r = JSON.stringify(parse("foo=bar&baz=qux"));"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(
        rt.eval_script("globalThis.r").unwrap(),
        r#"{"foo":"bar","baz":"qux"}"#
    );
}

#[test]
fn test_node_querystring_parse_with_eq_and_sep() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { parse } from "node:querystring";
globalThis.r = JSON.stringify(parse("foo=bar;baz=qux", ";", "="));"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(
        rt.eval_script("globalThis.r").unwrap(),
        r#"{"foo":"bar","baz":"qux"}"#
    );
}

#[test]
fn test_node_querystring_parse_array_duplicate() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { parse } from "node:querystring";
globalThis.r = JSON.stringify(parse("a=1&a=2&a=3"));"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(
        rt.eval_script("globalThis.r").unwrap(),
        r#"{"a":["1","2","3"]}"#
    );
}

#[test]
fn test_node_querystring_parse_empty() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { parse } from "node:querystring";
globalThis.r = JSON.stringify(parse(""));"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "{}");
}

#[test]
fn test_node_querystring_parse_no_value() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { parse } from "node:querystring";
globalThis.r = JSON.stringify(parse("foo&bar=baz"));"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(
        rt.eval_script("globalThis.r").unwrap(),
        r#"{"foo":"","bar":"baz"}"#
    );
}

#[test]
fn test_node_querystring_stringify() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { stringify } from "node:querystring";
globalThis.r = stringify({ foo: "bar", baz: "qux" });"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "foo=bar&baz=qux");
}

#[test]
fn test_node_querystring_stringify_array() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { stringify } from "node:querystring";
globalThis.r = stringify({ a: [1, 2, 3] });"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "a=1&a=2&a=3");
}

#[test]
fn test_node_querystring_escape_unescape() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { escape, unescape } from "node:querystring";
globalThis.r = unescape(escape("hello world"));"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "hello world");
}

#[test]
fn test_node_querystring_default_import() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import qs from "node:querystring";
globalThis.r = typeof qs.parse === "function" && typeof qs.stringify === "function";"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

#[test]
fn test_node_querystring_decode_encode_aliases() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { decode, encode } from "node:querystring";
globalThis.r = decode("a=1").a === "1" && encode({b:2}) === "b=2";"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}
