/// 重现 Boa DefInitVar crash — ESM import → CJS → require → CJS → crash
#[test]
fn test_import_cjs_chain_triggers_panic() {
    use std::path::Path;
    let dir = std::env::temp_dir().join("oolong_test_chain_panic");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    // innermost.js — 有 baseName 声明的 CJS
    let inner = dir.join("innermost.cjs");
    std::fs::write(
        &inner,
        r#"'use strict';
var base32 = 42
var callback = base32
var check = base32
var ext = base32
var parse = base32
var res = base32
var val = base32
module.exports = { val: base32 }
"#,
    )
    .unwrap();

    // middle.js — 有 function 声明的 CJS
    let middle = dir.join("middle.cjs");
    std::fs::write(
        &middle,
        format!(
            r#"'use strict';
var inner = require("{}")
function wrap() {{ return inner.val }}
function check() {{ return wrap() }}
module.exports = {{ result: check() }}
"#,
            inner.display()
        ),
    )
    .unwrap();

    // entry.cjs — 有 function + 裸名 require
    let entry_cjs = dir.join("entry.cjs");
    std::fs::write(
        &entry_cjs,
        format!(
            r#"'use strict';
exports = module.exports = createApp
function createApp() {{
  var m = require("{}")
  return m.result
}}
"#,
            middle.display()
        ),
    )
    .unwrap();

    // main.mjs — ESM import
    let main = dir.join("main.mjs");
    std::fs::write(
        &main,
        format!(
            r#"import mod from "{}";
globalThis.r = typeof mod === 'function'
"#,
            entry_cjs.display()
        ),
    )
    .unwrap();

    let mut rt = oolong::runtime::OolongRuntime::with_node_compat(&dir, Some(22)).unwrap();
    let result = rt.eval_module_file(&main);
    match result {
        Ok(val) => println!("OK: {}", val),
        Err(e) => println!("ERR: {}", e),
    }
}
