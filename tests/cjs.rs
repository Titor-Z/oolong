// ── CJS require ─────────────────────────────────────────────────────────────────

#[test]
fn test_cjs_require_builtin() {
    let dir = std::env::temp_dir().join("oolong_test_cjs_require");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let cjs_path = dir.join("module.cjs");
    std::fs::write(
        &cjs_path,
        r#"const path = require('node:path');
module.exports = { joined: path.join('a', 'b') };"#,
    )
    .unwrap();

    let entry = dir.join("main.mjs");
    std::fs::write(
        &entry,
        format!(
            r#"import mod from "{}";
globalThis.r = mod.joined === "a/b";"#,
            cjs_path.display(),
        ),
    )
    .unwrap();

    let mut rt = oolong::runtime::OolongRuntime::with_node_compat(&dir, true).unwrap();
    rt.eval_module_file(&entry).unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_cjs_require_relative() {
    let dir = std::env::temp_dir().join("oolong_test_cjs_require_rel");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let lib_path = dir.join("lib.cjs");
    std::fs::write(
        &lib_path,
        r#"module.exports = { val: 42 };"#,
    )
    .unwrap();

    let cjs_path = dir.join("module.cjs");
    std::fs::write(
        &cjs_path,
        r#"const lib = require('./lib.cjs');
module.exports = { result: lib.val + 1 };"#,
    )
    .unwrap();

    let entry = dir.join("main.mjs");
    std::fs::write(
        &entry,
        format!(
            r#"import mod from "{}";
globalThis.r = mod.result === 43;"#,
            cjs_path.display(),
        ),
    )
    .unwrap();

    let mut rt = oolong::runtime::OolongRuntime::new(&dir).unwrap();
    rt.eval_module_file(&entry).unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");

    let _ = std::fs::remove_dir_all(dir);
}

// ── CJS __dirname / __filename ────────────────────────────────────────────────

#[test]
fn test_cjs_file_dirname_filename() {
    let dir = std::env::temp_dir().join("oolong_test_cjs_dirname");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let cjs_path = dir.join("module.cjs");
    std::fs::write(
        &cjs_path,
        r#"module.exports = { dir: __dirname, file: __filename };"#,
    )
    .unwrap();

    let entry = dir.join("main.js");
    std::fs::write(
        &entry,
        format!(
            r#"import mod from "{}";
globalThis.r = mod.dir === "{}" && mod.file === "{}";"#,
            cjs_path.display(),
            dir.to_string_lossy(),
            cjs_path.to_string_lossy(),
        ),
    )
    .unwrap();

    let mut rt = oolong::runtime::OolongRuntime::new(&dir).unwrap();
    rt.eval_module_file(&entry).unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_cjs_transpiled_dirname_filename() {
    let dir = std::env::temp_dir().join("oolong_test_cjs_transpiled");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let js_path = dir.join("helper.js");
    std::fs::write(
        &js_path,
        r#"globalThis._d = __dirname; globalThis._f = __filename;"#,
    )
    .unwrap();

    let entry = dir.join("main.js");
    std::fs::write(
        &entry,
        format!(
            r#"import "{}";
globalThis.r = typeof globalThis._d === "string" && globalThis._d.length > 0 && typeof globalThis._f === "string" && globalThis._f.length > 0;"#,
            js_path.display(),
        ),
    )
    .unwrap();

    let mut rt = oolong::runtime::OolongRuntime::new(&dir).unwrap();
    rt.eval_module_file(&entry).unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");

    let _ = std::fs::remove_dir_all(dir);
}
