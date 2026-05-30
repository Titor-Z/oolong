mod common;

#[test]
fn test_eval_script_basic() {
    let mut rt = common::create_runtime();
    let result = rt.eval_script("1 + 2").unwrap();
    assert_eq!(result, "3");
}

#[test]
fn test_eval_script_string() {
    let mut rt = common::create_runtime();
    let result = rt.eval_script("'hello' + ' ' + 'world'").unwrap();
    assert_eq!(result, "hello world");
}

#[test]
fn test_eval_script_error() {
    let mut rt = common::create_runtime();
    let result = rt.eval_script("throw new Error('boom')");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("boom"));
}

#[test]
fn test_eval_module_str_simple() {
    let mut rt = common::create_runtime();
    let result = rt.eval_module_str("export const x = 42;", None).unwrap();
    // Module evaluation returns undefined for non-main modules
    assert_eq!(result, "undefined");
}

#[test]
fn test_eval_module_file_js() {
    let dir = std::env::temp_dir().join("oolong_test_module_js");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let entry = dir.join("main.js");
    std::fs::write(&entry, "export const msg = 'hi'; export default msg;").unwrap();

    let mut rt = oolong::runtime::OolongRuntime::new(&dir).unwrap();
    let result = rt.eval_module_file(&entry).unwrap();
    // Module evaluation of a file with just exports returns undefined
    assert_eq!(result, "undefined");

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_eval_module_file_ts() {
    let dir = std::env::temp_dir().join("oolong_test_module_ts");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let entry = dir.join("main.ts");
    std::fs::write(
        &entry,
        "export const greet = (name: string): string => `hello ${name}`;",
    )
    .unwrap();

    let mut rt = oolong::runtime::OolongRuntime::new(&dir).unwrap();
    let result = rt.eval_module_file(&entry).unwrap();
    assert_eq!(result, "undefined");

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_set_timeout_returns_number() {
    let mut rt = common::create_runtime();
    let result = rt.eval_script("setTimeout(() => {}, 0)").unwrap();
    // setTimeout returns a positive integer ID
    let id: i32 = result.parse().unwrap();
    assert!(id > 0);
}

#[test]
fn test_set_interval_returns_number() {
    let mut rt = common::create_runtime();
    let result = rt.eval_script("setInterval(() => {}, 100)").unwrap();
    let id: i32 = result.parse().unwrap();
    assert!(id > 0);
}

#[test]
fn test_clear_timeout_does_not_crash() {
    let mut rt = common::create_runtime();
    let result = rt
        .eval_script("var id = setTimeout(() => {}, 50); clearTimeout(id); 42")
        .unwrap();
    assert_eq!(result, "42");
}

#[test]
fn test_clear_interval_does_not_crash() {
    let mut rt = common::create_runtime();
    let result = rt
        .eval_script("var id = setInterval(() => {}, 50); clearInterval(id); 42")
        .unwrap();
    assert_eq!(result, "42");
}
