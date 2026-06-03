#[test]
fn test_cjs_var_inside_wrapper_triggers_boabug() {
    let dir = std::env::temp_dir().join("oolong_test_cjs_boabug");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    // 模拟 CJS 包装：5 个参数 + var 声明
    let cjs_path = dir.join("module.cjs");
    std::fs::write(
        &cjs_path,
        r#"var a = 1;
var b = 2;
module.exports = { sum: a + b };"#,
    )
    .unwrap();

    let entry = dir.join("main.mjs");
    std::fs::write(
        &entry,
        format!(
            r#"import mod from "{}";
globalThis.r = mod.sum === 3;"#,
            cjs_path.display(),
        ),
    )
    .unwrap();

    let mut rt = oolong::runtime::OolongRuntime::with_node_compat(&dir, Some(22)).unwrap();
    rt.eval_module_file(&entry).unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_cjs_nested_require_triggers_boabug() {
    let dir = std::env::temp_dir().join("oolong_test_cjs_nested");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    // lib.cjs — 有 var 声明的 CJS 模块
    let lib_path = dir.join("lib.cjs");
    std::fs::write(
        &lib_path,
        r#"var val = 42;
module.exports = { val };"#,
    )
    .unwrap();

    // entry.cjs — require 上面的模块
    let cjs_path = dir.join("entry.cjs");
    std::fs::write(
        &cjs_path,
        format!(
            r#"var lib = require("{}");
module.exports = {{ result: lib.val }};"#,
            lib_path.display()
        ),
    )
    .unwrap();

    let entry = dir.join("main.mjs");
    std::fs::write(
        &entry,
        format!(
            r#"import mod from "{}";
globalThis.r = mod.result === 42;"#,
            cjs_path.display(),
        ),
    )
    .unwrap();

    let mut rt = oolong::runtime::OolongRuntime::with_node_compat(&dir, Some(22)).unwrap();
    rt.eval_module_file(&entry).unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_cjs_many_vars_triggers_boabug() {
    let dir = std::env::temp_dir().join("oolong_test_cjs_manyvars");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let cjs_path = dir.join("module.cjs");
    std::fs::write(
        &cjs_path,
        r#"var a = 1;
var b = 2;
var c = 3;
var d = 4;
var e = 5;
module.exports = { result: a + b + c + d + e };"#,
    )
    .unwrap();

    let entry = dir.join("main.mjs");
    std::fs::write(
        &entry,
        format!(
            r#"import mod from "{}";
globalThis.r = mod.result === 15;"#,
            cjs_path.display(),
        ),
    )
    .unwrap();

    let mut rt = oolong::runtime::OolongRuntime::with_node_compat(&dir, Some(22)).unwrap();
    rt.eval_module_file(&entry).unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_cjs_mixed_var_let_triggers_boabug() {
    let dir = std::env::temp_dir().join("oolong_test_cjs_mixed");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let cjs_path = dir.join("module.cjs");
    std::fs::write(
        &cjs_path,
        r#"var a = 1;
let b = 2;
const c = 3;
module.exports = { result: a + b + c };"#,
    )
    .unwrap();

    let entry = dir.join("main.mjs");
    std::fs::write(
        &entry,
        format!(
            r#"import mod from "{}";
globalThis.r = mod.result === 6;"#,
            cjs_path.display(),
        ),
    )
    .unwrap();

    let mut rt = oolong::runtime::OolongRuntime::with_node_compat(&dir, Some(22)).unwrap();
    rt.eval_module_file(&entry).unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");

    let _ = std::fs::remove_dir_all(dir);
}
