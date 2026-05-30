mod common;

#[test]
fn test_import_uuid_module() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import * as uuid from "@std/uuid"; globalThis.r = typeof uuid;"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "object");
}

#[test]
fn test_uuid_v4_format() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { v4 } from "@std/uuid";
let u = v4();
globalThis.r = u;"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    let result = rt.eval_script("globalThis.r").unwrap();
    // UUID v4 format: xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx
    assert_eq!(result.len(), 36);
    assert_eq!(&result[14..15], "4", "v4 UUID 的第 15 位应为 4");
    let c = &result[19..20];
    assert!(
        c == "8" || c == "9" || c == "a" || c == "b",
        "v4 UUID 的第 20 位应为 8/9/a/b，实际为 {c}"
    );
    assert_eq!(&result[8..9], "-");
    assert_eq!(&result[13..14], "-");
    assert_eq!(&result[18..19], "-");
    assert_eq!(&result[23..24], "-");
}

#[test]
fn test_uuid_v4_unique() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { v4 } from "@std/uuid";
let a = v4();
let b = v4();
globalThis.r = a === b ? "same" : "different";"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "different");
}

#[test]
fn test_uuid_validate_valid() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { v4, validate } from "@std/uuid";
let u = v4();
globalThis.r = validate(u);"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

#[test]
fn test_uuid_validate_invalid() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { validate } from "@std/uuid";
globalThis.r = validate("not-a-uuid");"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "false");
}

#[test]
fn test_uuid_validate_empty() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { validate } from "@std/uuid";
globalThis.r = validate("");"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "false");
}

#[test]
fn test_uuid_default_export() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import uuid from "@std/uuid";
globalThis.r = typeof uuid.v4 + " " + typeof uuid.validate;"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "function function");
}

#[test]
fn test_type_consistency_std_uuid() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import * as mod from "@std/uuid";
let keys = Object.keys(mod).sort();
globalThis.r = JSON.stringify(keys);"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    let result = rt.eval_script("globalThis.r").unwrap();
    assert!(result.contains("v4"), "缺少 v4: {result}");
    assert!(result.contains("validate"), "缺少 validate: {result}");
    assert!(result.contains("default"), "缺少 default: {result}");
}
