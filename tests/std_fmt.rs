mod common;

#[test]
fn test_import_fmt_module() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import * as fmt from "@std/fmt"; globalThis.r = typeof fmt;"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "object");
}

#[test]
fn test_colors_red() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { colors } from "@std/fmt";
globalThis.r = colors.red("hello");"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(
        rt.eval_script("globalThis.r").unwrap(),
        "\x1b[31mhello\x1b[0m"
    );
}

#[test]
fn test_colors_green() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { colors } from "@std/fmt";
globalThis.r = colors.green("world");"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(
        rt.eval_script("globalThis.r").unwrap(),
        "\x1b[32mworld\x1b[0m"
    );
}

#[test]
fn test_colors_nested() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { colors } from "@std/fmt";
globalThis.r = colors.blue(colors.bold("text"));"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(
        rt.eval_script("globalThis.r").unwrap(),
        "\x1b[34m\x1b[1mtext\x1b[22m\x1b[0m"
    );
}

#[test]
fn test_strip_color() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { colors } from "@std/fmt";
let colored = colors.red("\x1b[1mbold red\x1b[22m");
globalThis.r = colors.stripColor(colored);"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "bold red");
}

#[test]
fn test_sprintf_simple() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { sprintf } from "@std/fmt";
globalThis.r = sprintf("hello %s", "world");"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "hello world");
}

#[test]
fn test_sprintf_multiple() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { sprintf } from "@std/fmt";
globalThis.r = sprintf("%s %s %s", "a", "b", "c");"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "a b c");
}

#[test]
fn test_sprintf_integer() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { sprintf } from "@std/fmt";
globalThis.r = sprintf("%d + %d = %d", 1, 2, 3);"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "1 + 2 = 3");
}

#[test]
fn test_sprintf_float() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { sprintf } from "@std/fmt";
globalThis.r = sprintf("%f", 3.14);"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    assert!(r.starts_with("3"), "expected 3.x, got {r}");
}

#[test]
fn test_sprintf_percent_escape() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { sprintf } from "@std/fmt";
globalThis.r = sprintf("100%%");"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "100%");
}

#[test]
fn test_sprintf_no_args() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { sprintf } from "@std/fmt";
globalThis.r = sprintf("no placeholders");"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "no placeholders");
}

#[test]
fn test_sprintf_mixed() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { sprintf } from "@std/fmt";
let n = 42;
let s = "answer";
globalThis.r = sprintf("the %s is %d", s, n);"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "the answer is 42");
}

#[test]
fn test_default_export() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import fmt from "@std/fmt";
globalThis.r = typeof fmt.colors.red + " " + typeof fmt.sprintf;"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "function function");
}

#[test]
fn test_type_consistency_std_fmt() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import * as mod from "@std/fmt";
let keys = Object.keys(mod).sort();
globalThis.r = JSON.stringify(keys);"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    let result = rt.eval_script("globalThis.r").unwrap();
    assert!(result.contains("colors"), "missing colors: {result}");
    assert!(result.contains("sprintf"), "missing sprintf: {result}");
    assert!(result.contains("default"), "missing default: {result}");
}

#[test]
fn test_colors_bold() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { colors } from "@std/fmt";
globalThis.r = colors.bold("bold text");"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(
        rt.eval_script("globalThis.r").unwrap(),
        "\x1b[1mbold text\x1b[22m"
    );
}

#[test]
fn test_colors_all_exist() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { colors } from "@std/fmt";
let names = ["red","green","yellow","blue","magenta","cyan","white","gray","bold","dim","italic","underline","stripColor"];
let ok = names.every(n => typeof colors[n] === "function");
globalThis.r = ok ? "yes" : "no";"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "yes");
}
