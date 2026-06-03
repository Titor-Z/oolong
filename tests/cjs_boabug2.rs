use std::path::Path;

/// 测试：CJS 包装函数中，内部 function 声明是否触发 DefInitVar 越界
#[test]
fn test_cjs_function_declaration() {
    let dir = std::env::temp_dir().join("oolong_test_cjs_funcdecl");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let cjs_path = dir.join("module.cjs");
    std::fs::write(
        &cjs_path,
        r#"function bodyParser() {
  return {};
}
var a = bodyParser();
module.exports = { result: a };"#,
    )
    .unwrap();

    let entry = dir.join("main.mjs");
    std::fs::write(
        &entry,
        format!(
            r#"import mod from "{}";
globalThis.r = typeof mod.result === "object";"#,
            cjs_path.display(),
        ),
    )
    .unwrap();

    let mut rt = oolong::runtime::OolongRuntime::with_node_compat(&dir, Some(22)).unwrap();
    rt.eval_module_file(&entry).unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");

    let _ = std::fs::remove_dir_all(dir);
}

/// 测试：多个 function 声明（模拟 Express 模式）
#[test]
fn test_cjs_multiple_functions() {
    let dir = std::env::temp_dir().join("oolong_test_cjs_multi_fn");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let cjs_path = dir.join("module.cjs");
    std::fs::write(
        &cjs_path,
        r#"function json() { return 'json'; }
function raw() { return 'raw'; }
function text() { return 'text'; }
function urlencoded() { return 'urlencoded'; }
module.exports = { json: json(), raw: raw() };"#,
    )
    .unwrap();

    let entry = dir.join("main.mjs");
    std::fs::write(
        &entry,
        format!(
            r#"import mod from "{}";
globalThis.r = mod.json === 'json' && mod.raw === 'raw';"#,
            cjs_path.display(),
        ),
    )
    .unwrap();

    let mut rt = oolong::runtime::OolongRuntime::with_node_compat(&dir, Some(22)).unwrap();
    rt.eval_module_file(&entry).unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");

    let _ = std::fs::remove_dir_all(dir);
}

/// 测试：function + var (5个参数 + 1 function + 1 var = 7)
#[test]
fn test_cjs_func_plus_var() {
    let dir = std::env::temp_dir().join("oolong_test_cjs_funcvar");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let cjs_path = dir.join("module.cjs");
    std::fs::write(
        &cjs_path,
        r#"function foo() { return 1; }
function bar() { return 2; }
var x = foo();
var y = bar();
module.exports = { result: x + y };"#,
    )
    .unwrap();

    let entry = dir.join("main.mjs");
    std::fs::write(
        &entry,
        format!(
            r#"import mod from "{}";
globalThis.r = mod.result === 3;"#,
            cjs_path.display(),
        ),
    )
    .unwrap();

    let mut rt = oolong::runtime::OolongRuntime::with_node_compat(&dir, Some(22)).unwrap();
    rt.eval_module_file(&entry).unwrap();

    let _ = std::fs::remove_dir_all(dir);
}

/// 测试：嵌套 require（最深的是 Express 模式）
#[test]
fn test_cjs_func_require_chain() {
    let dir = std::env::temp_dir().join("oolong_test_cjs_chain");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    // lib.js — 最内层，有 function 声明
    let lib_path = dir.join("lib.cjs");
    std::fs::write(
        &lib_path,
        r#"function helper() { return 42; }
module.exports = { val: helper() };"#,
    )
    .unwrap();

    // mid.js — 中间层，function + require
    let mid_path = dir.join("mid.cjs");
    std::fs::write(
        &mid_path,
        format!(
            r#"function midFn() {{ return require("{}").val; }}
var v = midFn();
module.exports = {{ result: v }};"#,
            lib_path.display()
        ),
    )
    .unwrap();

    // entry.cjs — 外层
    let entry_path = dir.join("entry.cjs");
    std::fs::write(
        &entry_path,
        format!(
            r#"function entryFn() {{ return require("{}").result; }}
module.exports = {{ result: entryFn() }};"#,
            mid_path.display()
        ),
    )
    .unwrap();

    let entry = dir.join("main.mjs");
    std::fs::write(
        &entry,
        format!(
            r#"import mod from "{}";
globalThis.r = mod.result === 42;"#,
            entry_path.display(),
        ),
    )
    .unwrap();

    let mut rt = oolong::runtime::OolongRuntime::with_node_compat(&dir, Some(22)).unwrap();
    rt.eval_module_file(&entry).unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");

    let _ = std::fs::remove_dir_all(dir);
}

/// 测试：body-parser 类似的模式（exports reassignment）
#[test]
fn test_cjs_exports_reassignment() {
    let dir = std::env::temp_dir().join("oolong_test_cjs_exports");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let cjs_path = dir.join("module.cjs");
    std::fs::write(
        &cjs_path,
        r#"exports = module.exports = something
function something() { return 'ok'; }
Object.defineProperty(exports, 'json', {
  get: () => 'json'
});"#,
    )
    .unwrap();

    let entry = dir.join("main.mjs");
    std::fs::write(
        &entry,
        format!(
            r#"import mod from "{}";
globalThis.r = typeof mod === 'function';"#,
            cjs_path.display(),
        ),
    )
    .unwrap();

    let mut rt = oolong::runtime::OolongRuntime::with_node_compat(&dir, Some(22)).unwrap();
    rt.eval_module_file(&entry).unwrap();

    let _ = std::fs::remove_dir_all(dir);
}
