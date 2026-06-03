/// 模拟 cha.json imports 解析 + ESM import → CJS require chain
#[test]
fn test_import_via_imports_map_and_chain() {
    use std::path::{Path, PathBuf};
    let dir = std::env::temp_dir().join("oolong_test_import_chain");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    // 模拟 ~/.cha/modules/npm/ 结构
    let cache_root = dir.join(".cha").join("modules").join("npm");
    std::fs::create_dir_all(&cache_root).unwrap();

    // inner-pkg — 最深层
    let inner_dir = cache_root.join("inner-pkg@1.0.0");
    std::fs::create_dir_all(&inner_dir).unwrap();
    std::fs::write(
        inner_dir.join("package.json"),
        r#"{"name":"inner-pkg","version":"1.0.0"}"#,
    ).unwrap();
    std::fs::write(
        inner_dir.join("index.js"),
        r#"'use strict';
var keys = ['a', 'b']
var vals = [1, 2]
var obj = {}
for (var i = 0; i < keys.length; i++) {
  obj[keys[i]] = vals[i]
}
var result = JSON.stringify(obj)
module.exports = { result: result }
"#,
    )
    .unwrap();

    // mid-pkg — 中间层，通过 require 引用 inner-pkg
    let mid_dir = cache_root.join("mid-pkg@1.0.0");
    std::fs::create_dir_all(&mid_dir).unwrap();
    std::fs::write(
        mid_dir.join("package.json"),
        r#"{"name":"mid-pkg","version":"1.0.0"}"#,
    ).unwrap();
    std::fs::write(
        mid_dir.join("index.js"),
        r#"'use strict';
var inner = require('inner-pkg')
exports = module.exports = createApp
function createApp() {
  return inner.result
}
"#,
    )
    .unwrap();

    // cha.json — 模拟 imports
    let cha_json = dir.join("cha.json");
    std::fs::write(
        &cha_json,
        r#"{
  "name": "test-chain",
  "version": "0.1.0",
  "nodeCompat": 22,
  "imports": {
    "mid-pkg": "npm:mid-pkg@1.0.0",
    "inner-pkg": "npm:inner-pkg@1.0.0"
  }
}"#,
    )
    .unwrap();

    // main.mjs — ESM import bare specifier
    let main = dir.join("main.mjs");
    std::fs::write(
        &main,
        r#"import mod from "mid-pkg";
globalThis.r = typeof mod === 'function';
"#,
    )
    .unwrap();

    let mut rt = oolong::runtime::OolongRuntime::with_node_compat(&dir, Some(22)).unwrap();
    let result = rt.eval_module_file(&main);
    match result {
        Ok(val) => println!("OK: {}", val),
        Err(e) => println!("ERR: {}", e),
    }
}

/// 更精确模拟：通过 cha.json imports 解析 + 多个 function 声明
#[test]
fn test_import_via_imports_many_funcs() {
    use std::path::Path;
    let dir = std::env::temp_dir().join("oolong_test_import_funcs");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    // 模拟 ~/.cha/modules/npm/
    let cache_root = dir.join(".cha").join("modules").join("npm");
    std::fs::create_dir_all(&cache_root).unwrap();

    // pkg — 有大量 function 声明
    let pkg_dir = cache_root.join("test-pkg@1.0.0");
    std::fs::create_dir_all(&pkg_dir).unwrap();
    std::fs::write(
        pkg_dir.join("package.json"),
        r#"{"name":"test-pkg","version":"1.0.0"}"#,
    ).unwrap();
    std::fs::write(
        pkg_dir.join("index.js"),
        r#"'use strict';
var a = require('./lib/a')
var b = require('./lib/b')
exports = module.exports = testFn
function testFn() {
  return a.val + b.val
}
"#,
    )
    .unwrap();

    // lib/a.js
    let lib_dir = pkg_dir.join("lib");
    std::fs::create_dir_all(&lib_dir).unwrap();
    std::fs::write(
        lib_dir.join("a.js"),
        r#"'use strict';
var items = [1, 2, 3]
var result = 0
for (var i = 0; i < items.length; i++) {
  result += items[i]
}
function calc() { return result }
module.exports = { val: calc() }
"#,
    )
    .unwrap();

    // lib/b.js
    std::fs::write(
        lib_dir.join("b.js"),
        r#"'use strict';
var x = 10
var y = 20
function add(a, b) { return a + b }
module.exports = { val: add(x, y) }
"#,
    )
    .unwrap();

    // cha.json
    let cha_json = dir.join("cha.json");
    std::fs::write(
        &cha_json,
        r#"{
  "name": "test-chain",
  "version": "0.1.0",
  "nodeCompat": 22,
  "imports": {
    "test-pkg": "npm:test-pkg@1.0.0"
  }
}"#,
    )
    .unwrap();

    // main.mjs
    let main = dir.join("main.mjs");
    std::fs::write(
        &main,
        r#"import mod from "test-pkg";
globalThis.r = typeof mod === 'function';
"#,
    )
    .unwrap();

    let mut rt = oolong::runtime::OolongRuntime::with_node_compat(&dir, Some(22)).unwrap();
    let result = rt.eval_module_file(&main);
    match result {
        Ok(val) => println!("OK: {}", val),
        Err(e) => println!("ERR: {}", e),
    }
}
