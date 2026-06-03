/// 测试：body-parser 的精确代码模式（exports reassign + function after reference）
#[test]
fn test_cjs_exact_bodyparser() {
    use std::path::Path;
    let dir = std::env::temp_dir().join("oolong_test_cjs_bpexact");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let cjs_path = dir.join("module.cjs");
    std::fs::write(
        &cjs_path,
        r#"'use strict'
exports = module.exports = bodyParser

Object.defineProperty(exports, 'json', {
  get: () => 'json'
})

function bodyParser () {
  return 'ok'
}
"#,
    )
    .unwrap();

    let entry = dir.join("main.mjs");
    std::fs::write(
        &entry,
        format!(
            r#"import mod from "{}";
globalThis.r = mod.json === 'json';"#,
            cjs_path.display(),
        ),
    )
    .unwrap();

    let mut rt = oolong::runtime::OolongRuntime::with_node_compat(&dir, Some(22)).unwrap();
    let result = rt.eval_module_file(&entry);
    match result {
        Ok(val) => {
            println!("OK: {}", val);
        }
        Err(e) => {
            println!("ERR: {}", e);
        }
    }

    let _ = std::fs::remove_dir_all(dir);
}

/// 测试：function 在赋值后声明 + strict mode
#[test]
fn test_cjs_func_after_assign_strict() {
    use std::path::Path;
    let dir = std::env::temp_dir().join("oolong_test_cjs_fas");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let cjs_path = dir.join("module.cjs");
    std::fs::write(
        &cjs_path,
        r#"'use strict'
var result = bodyParser()
function bodyParser () { return 'ok' }
module.exports = { result }
"#,
    )
    .unwrap();

    let entry = dir.join("main.mjs");
    std::fs::write(
        &entry,
        format!(
            r#"import mod from "{}";
globalThis.r = mod.result === 'ok';"#,
            cjs_path.display(),
        ),
    )
    .unwrap();

    let mut rt = oolong::runtime::OolongRuntime::with_node_compat(&dir, Some(22)).unwrap();
    let result = rt.eval_module_file(&entry);
    match result {
        Ok(val) => println!("OK: {}", val),
        Err(e) => println!("ERR: {}", e),
    }

    let _ = std::fs::remove_dir_all(dir);
}

/// 测试：函数在主代码流之前被引用 + strict mode
#[test]
fn test_cjs_ref_before_decl() {
    use std::path::Path;
    let dir = std::env::temp_dir().join("oolong_test_cjs_rbd");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let cjs_path = dir.join("module.cjs");
    std::fs::write(
        &cjs_path,
        r#"'use strict'
exports = module.exports = myFunc

function myFunc () {
  return 42
}
"#,
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
    let result = rt.eval_module_file(&entry);
    match result {
        Ok(val) => println!("OK: {}", val),
        Err(e) => println!("ERR: {}", e),
    }

    let _ = std::fs::remove_dir_all(dir);
}
