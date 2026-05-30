mod common;
use std::path::Path;
// ── node:assert ──────────────────────────────────────────────────────────────

#[test]
fn test_node_assert_ok() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import assert from "node:assert";
globalThis.r = "ok";
assert.ok(true);
globalThis.r = "passed";"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "passed");
}

#[test]
fn test_node_assert_ok_throws() {
    let mut rt = common::create_runtime();
    let result = rt.eval_module_str(
        r#"import assert from "node:assert";
assert.ok(false);"#,
        Some(Path::new("__t.js")),
    );
    assert!(result.is_err());
}

#[test]
fn test_node_assert_equal() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import assert from "node:assert";
assert.equal(3, 3);
assert.equal("hello", "hello");"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
}

#[test]
fn test_node_assert_equal_throws() {
    let mut rt = common::create_runtime();
    let result = rt.eval_module_str(
        r#"import assert from "node:assert";
assert.equal(1, 2);"#,
        Some(Path::new("__t.js")),
    );
    assert!(result.is_err());
}

#[test]
fn test_node_assert_strict_equal() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import assert from "node:assert";
assert.strictEqual(1, 1);"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
}

#[test]
fn test_node_assert_strict_equal_throws() {
    let mut rt = common::create_runtime();
    let result = rt.eval_module_str(
        r#"import assert from "node:assert";
assert.strictEqual(1, "1");"#,
        Some(Path::new("__t.js")),
    );
    assert!(result.is_err());
}

#[test]
fn test_node_assert_not_strict_equal() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import assert from "node:assert";
assert.notStrictEqual(1, "1");"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
}

#[test]
fn test_node_assert_deep_equal() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import assert from "node:assert";
assert.deepEqual({ a: 1 }, { a: 1 });"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
}

#[test]
fn test_node_assert_throws_basic() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import assert from "node:assert";
assert.throws(() => { throw new Error("boom"); });"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
}

#[test]
fn test_node_assert_throws_missing() {
    let mut rt = common::create_runtime();
    let result = rt.eval_module_str(
        r#"import assert from "node:assert";
assert.throws(() => {});"#,
        Some(Path::new("__t.js")),
    );
    assert!(result.is_err());
}

#[test]
fn test_node_assert_throws_instanceof() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import assert from "node:assert";
assert.throws(() => { throw new TypeError("bad"); }, TypeError);"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
}

#[test]
fn test_node_assert_if_error() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import assert from "node:assert";
assert.ifError(null);
assert.ifError(undefined);"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
}

#[test]
fn test_node_assert_if_error_throws() {
    let mut rt = common::create_runtime();
    let result = rt.eval_module_str(
        r#"import assert from "node:assert";
assert.ifError(new Error("err"));"#,
        Some(Path::new("__t.js")),
    );
    assert!(result.is_err());
}

#[test]
fn test_node_assert_fail() {
    let mut rt = common::create_runtime();
    let result = rt.eval_module_str(
        r#"import assert from "node:assert";
assert.fail("intentional");"#,
        Some(Path::new("__t.js")),
    );
    assert!(result.is_err());
}

#[test]
fn test_node_assert_strict_namespace() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { strict } from "node:assert";
strict.equal(1, 1);
var threw = false;
try { strict.equal(1, "1"); } catch (e) { threw = true; }
globalThis.r = threw;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

#[test]
fn test_node_assert_assertion_error() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { AssertionError } from "node:assert";
var e = new AssertionError({ message: "test", actual: 1, expected: 2 });
globalThis.r = e.name === "AssertionError" && e.message === "test" && e.actual === 1 && e.expected === 2;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

#[test]
fn test_node_assert_default_import() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import assert from "node:assert";
globalThis.r = typeof assert.ok === "function" && typeof assert.strictEqual === "function" && typeof assert.throws === "function";"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}
