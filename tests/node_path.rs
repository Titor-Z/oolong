mod common;
use std::path::Path;

// ── node:path 模块 ─────────────────────────────────────────────────────────────

#[test]
fn test_node_path_join() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { join } from "node:path";
globalThis.r = join("a", "b", "c");"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "a/b/c");
}

#[test]
fn test_node_path_dirname() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { dirname } from "node:path";
globalThis.r = dirname("/a/b/c.txt");"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "/a/b");
}

#[test]
fn test_node_path_basename() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { basename } from "node:path";
globalThis.r = basename("/a/b/c.txt");"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "c.txt");
}

#[test]
fn test_node_path_extname() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { extname } from "node:path";
globalThis.r = extname("/a/b/c.txt");"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), ".txt");
}

#[test]
fn test_node_path_resolve() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { resolve } from "node:path";
globalThis.r = resolve("/a", "b", "c");"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "/a/b/c");
}

#[test]
fn test_node_path_relative() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { relative } from "node:path";
globalThis.r = relative("/a/b/c", "/a/d/e");"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "../../d/e");
}

#[test]
fn test_node_path_sep() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { sep } from "node:path"; globalThis.r = sep;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "/");
}

#[test]
fn test_node_path_delimiter() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { delimiter } from "node:path"; globalThis.r = delimiter;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), ":");
}

#[test]
fn test_node_path_posix() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { posix } from "node:path";
globalThis.r1 = posix.sep;
globalThis.r2 = posix.delimiter;
globalThis.r3 = posix.join("a", "b");"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r1").unwrap(), "/");
    assert_eq!(rt.eval_script("globalThis.r2").unwrap(), ":");
    assert_eq!(rt.eval_script("globalThis.r3").unwrap(), "a/b");
}

#[test]
fn test_node_path_win32() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { win32 } from "node:path";
globalThis.r1 = win32.sep;
globalThis.r2 = win32.delimiter;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r1").unwrap(), "\\");
    assert_eq!(rt.eval_script("globalThis.r2").unwrap(), ";");
}

#[test]
fn test_node_path_default_import() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import path from "node:path";
globalThis.r = path.join("x", "y");"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "x/y");
}

#[test]
fn test_node_path_normalize() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { normalize } from "node:path";
globalThis.r = normalize("/a/../b/./c//d");"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "/b/c/d");
}

#[test]
fn test_node_path_parse() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"
import { parse } from "node:path";
const p = parse("/a/b/c.txt");
globalThis.root = p.root;
globalThis.dir = p.dir;
globalThis.base = p.base;
globalThis.ext = p.ext;
globalThis.name = p.name;
"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.root").unwrap(), "/");
    assert_eq!(rt.eval_script("globalThis.dir").unwrap(), "/a/b");
    assert_eq!(rt.eval_script("globalThis.base").unwrap(), "c.txt");
    assert_eq!(rt.eval_script("globalThis.ext").unwrap(), ".txt");
    assert_eq!(rt.eval_script("globalThis.name").unwrap(), "c");
}

#[test]
fn test_node_path_format() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { format } from "node:path";
globalThis.r = format({ dir: "/a/b", base: "c.txt" });"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "/a/b/c.txt");
}

#[test]
fn test_node_path_is_absolute() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { isAbsolute } from "node:path";
globalThis.r1 = isAbsolute("/a/b");
globalThis.r2 = isAbsolute("a/b");"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r1").unwrap(), "true");
    assert_eq!(rt.eval_script("globalThis.r2").unwrap(), "false");
}

#[test]
fn test_node_path_to_namespaced_path() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { toNamespacedPath } from "node:path";
globalThis.r = toNamespacedPath("/foo/bar");"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "/foo/bar");
}
