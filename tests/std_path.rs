mod common;
use std::path::Path;

#[test]
fn test_import_path_join() {
    let mut rt = common::create_runtime();
    let code = r#"
import { join } from "@std/path";
export const result = join("a", "b", "c");
"#;
    // 使用 eval_module_str 需要确保模块解析正确
    // 用 path_hint 配合 builtin 模块加载
    let result = rt
        .eval_module_str(code, Some(Path::new("__test__.js")))
        .unwrap();
    // 模块 eval 返回 undefined（module 没 default），但导出的值在命名导出中
    // 我们通过 eval_module_str 只能拿到 module 执行后的 promise 值
    // 实际验证通过第二个模块来 import
    assert_eq!(result, "undefined");
}

#[test]
fn test_import_path_join_across_modules() {
    let mut rt = common::create_runtime();
    // 导入 path 模块并执行函数
    let code = r#"
import * as path from "@std/path";
globalThis.result = path.join("a", "b", "c");
"#;
    rt.eval_module_str(code, Some(Path::new("__test__.js")))
        .unwrap();
    let out = rt.eval_script("globalThis.result").unwrap();
    assert_eq!(out, "a/b/c");
}

#[test]
fn test_import_path_dirname() {
    let mut rt = common::create_runtime();
    let code = r#"
import { dirname } from "@std/path";
globalThis.r = dirname("/a/b/c.txt");
"#;
    rt.eval_module_str(code, Some(Path::new("__test__.js")))
        .unwrap();
    let out = rt.eval_script("globalThis.r").unwrap();
    assert_eq!(out, "/a/b");
}

#[test]
fn test_import_path_basename() {
    let mut rt = common::create_runtime();
    let code = r#"
import { basename } from "@std/path";
globalThis.r = basename("/a/b/c.txt");
"#;
    rt.eval_module_str(code, Some(Path::new("__test__.js")))
        .unwrap();
    let out = rt.eval_script("globalThis.r").unwrap();
    assert_eq!(out, "c.txt");
}

#[test]
fn test_import_path_extname() {
    let mut rt = common::create_runtime();
    let code = r#"
import { extname } from "@std/path";
globalThis.r = extname("/a/b/c.txt");
"#;
    rt.eval_module_str(code, Some(Path::new("__test__.js")))
        .unwrap();
    let out = rt.eval_script("globalThis.r").unwrap();
    assert_eq!(out, ".txt");
}

#[test]
fn test_import_path_is_absolute() {
    let mut rt = common::create_runtime();
    let code = r#"
import { isAbsolute } from "@std/path";
globalThis.r1 = isAbsolute("/a/b");
globalThis.r2 = isAbsolute("a/b");
"#;
    rt.eval_module_str(code, Some(Path::new("__test__.js")))
        .unwrap();
    let r1 = rt.eval_script("globalThis.r1").unwrap();
    let r2 = rt.eval_script("globalThis.r2").unwrap();
    assert_eq!(r1, "true");
    assert_eq!(r2, "false");
}

#[test]
fn test_import_path_normalize() {
    let mut rt = common::create_runtime();
    let code = r#"
import { normalize } from "@std/path";
globalThis.r = normalize("/a/../b/./c//d");
"#;
    rt.eval_module_str(code, Some(Path::new("__test__.js")))
        .unwrap();
    let out = rt.eval_script("globalThis.r").unwrap();
    assert_eq!(out, "/b/c/d");
}

#[test]
fn test_import_path_default_import() {
    let mut rt = common::create_runtime();
    let code = r#"
import path from "@std/path";
globalThis.r = path.join("x", "y");
"#;
    rt.eval_module_str(code, Some(Path::new("__test__.js")))
        .unwrap();
    let out = rt.eval_script("globalThis.r").unwrap();
    assert_eq!(out, "x/y");
}

#[test]
fn test_import_path_relative() {
    let mut rt = common::create_runtime();
    let code = r#"
import { relative } from "@std/path";
globalThis.r = relative("/a/b/c", "/a/d/e");
"#;
    rt.eval_module_str(code, Some(Path::new("__test__.js")))
        .unwrap();
    let out = rt.eval_script("globalThis.r").unwrap();
    assert_eq!(out, "../../d/e");
}

#[test]
fn test_import_path_sep() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { sep } from "@std/path"; globalThis.r = sep;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "/");
}

#[test]
fn test_import_path_delimiter() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { delimiter } from "@std/path"; globalThis.r = delimiter;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), ":");
}

#[test]
fn test_import_path_parse() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"
import { parse } from "@std/path";
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
fn test_import_path_format() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"
import { format } from "@std/path";
globalThis.r = format({ dir: "/a/b", base: "c.txt" });
"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "/a/b/c.txt");
}

