mod common;

#[test]
fn test_import_semver_module() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import * as semver from "@std/semver"; globalThis.r = typeof semver;"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "object");
}

#[test]
fn test_parse_basic() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { parse } from "@std/semver";
let sv = parse("1.2.3");
globalThis.r = sv.major + " " + sv.minor + " " + sv.patch;"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "1 2 3");
}

#[test]
fn test_parse_with_prerelease() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { parse } from "@std/semver";
let sv = parse("1.2.3-alpha.1");
globalThis.r = sv.major + "." + sv.minor + "." + sv.patch + " " + sv.prerelease.join(".");"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "1.2.3 alpha.1");
}

#[test]
fn test_parse_with_build() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { parse } from "@std/semver";
let sv = parse("1.2.3+build.42");
globalThis.r = sv.major + "." + sv.build.join(".");"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "1.build.42");
}

#[test]
fn test_parse_with_both() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { parse } from "@std/semver";
let sv = parse("2.0.0-rc.1+build.001");
globalThis.r = sv.major + " " + sv.prerelease[0] + " " + sv.build[0];"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "2 rc build");
}

#[test]
fn test_parse_invalid_throws() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { parse } from "@std/semver";
try { parse("not-a-version"); globalThis.r = "no-error"; }
catch (e) { globalThis.r = "error"; }"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "error");
}

#[test]
fn test_parse_empty_throws() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { parse } from "@std/semver";
try { parse(""); globalThis.r = "no-error"; }
catch (e) { globalThis.r = "error"; }"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "error");
}

#[test]
fn test_format_roundtrip() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { parse, format } from "@std/semver";
let sv = parse("3.4.5-pre+build");
globalThis.r = format(sv);"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "3.4.5-pre+build");
}

#[test]
fn test_format_from_object() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { format } from "@std/semver";
let sv = { major: 1, minor: 2, patch: 3, prerelease: [], build: [] };
globalThis.r = format(sv);"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "1.2.3");
}

#[test]
fn test_compare_equal() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { compare } from "@std/semver";
globalThis.r = compare("1.2.3", "1.2.3");"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "0");
}

#[test]
fn test_compare_greater() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { compare } from "@std/semver";
globalThis.r = compare("2.0.0", "1.9.9");"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "1");
}

#[test]
fn test_compare_less() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { compare } from "@std/semver";
globalThis.r = compare("1.0.0", "1.0.1");"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "-1");
}

#[test]
fn test_compare_prerelease() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { compare } from "@std/semver";
// release > prerelease
// prerelease: numeric < string
let a = compare("1.0.0-alpha", "1.0.0");
let b = compare("1.0.0-1", "1.0.0-alpha");
globalThis.r = a + " " + b;"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "-1 -1");
}

#[test]
fn test_greater_than() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { greaterThan } from "@std/semver";
globalThis.r = greaterThan("2.0.0", "1.0.0") + " " + greaterThan("1.0.0", "2.0.0");"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true false");
}

#[test]
fn test_less_than() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { lessThan } from "@std/semver";
globalThis.r = lessThan("1.0.0", "2.0.0") + " " + lessThan("2.0.0", "1.0.0");"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true false");
}

#[test]
fn test_equals() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { equals } from "@std/semver";
globalThis.r = equals("1.2.3", "1.2.3") + " " + equals("1.2.3", "1.2.4");"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true false");
}

#[test]
fn test_equals_with_object() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { equals } from "@std/semver";
let sv = { major: 1, minor: 2, patch: 3, prerelease: [], build: [] };
globalThis.r = equals("1.2.3", sv);"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

#[test]
fn test_satisfies_caret() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { satisfies } from "@std/semver";
let a = satisfies("1.2.3", "^1.2.3");
let b = satisfies("1.9.9", "^1.2.3");
let c = satisfies("2.0.0", "^1.2.3");
globalThis.r = a + " " + b + " " + c;"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true true false");
}

#[test]
fn test_satisfies_tilde() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { satisfies } from "@std/semver";
let a = satisfies("1.2.3", "~1.2.3");
let b = satisfies("1.2.9", "~1.2.3");
let c = satisfies("1.3.0", "~1.2.3");
globalThis.r = a + " " + b + " " + c;"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true true false");
}

#[test]
fn test_satisfies_gte() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { satisfies } from "@std/semver";
let a = satisfies("2.0.0", ">=1.2.3");
let b = satisfies("1.2.3", ">=1.2.3");
let c = satisfies("1.2.2", ">=1.2.3");
globalThis.r = a + " " + b + " " + c;"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true true false");
}

#[test]
fn test_satisfies_wildcard() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { satisfies } from "@std/semver";
globalThis.r = satisfies("99.99.99", "*");"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

#[test]
fn test_satisfies_invalid_version() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { satisfies } from "@std/semver";
globalThis.r = satisfies("not-a-version", "^1.0.0");"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "false");
}

#[test]
fn test_default_export() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import semver from "@std/semver";
globalThis.r = typeof semver.parse + " " + typeof semver.compare;"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "function function");
}

#[test]
fn test_type_consistency_std_semver() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import * as mod from "@std/semver";
let keys = Object.keys(mod).sort();
globalThis.r = JSON.stringify(keys);"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    let result = rt.eval_script("globalThis.r").unwrap();
    assert!(result.contains("parse"), "missing parse: {result}");
    assert!(result.contains("format"), "missing format: {result}");
    assert!(result.contains("compare"), "missing compare: {result}");
    assert!(
        result.contains("greaterThan"),
        "missing greaterThan: {result}"
    );
    assert!(result.contains("lessThan"), "missing lessThan: {result}");
    assert!(result.contains("equals"), "missing equals: {result}");
    assert!(result.contains("satisfies"), "missing satisfies: {result}");
    assert!(result.contains("default"), "missing default: {result}");
}

#[test]
fn test_to_string_method() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { parse } from "@std/semver";
let sv = parse("4.5.6-rc.2+build.3");
globalThis.r = sv.toString();"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(
        rt.eval_script("globalThis.r").unwrap(),
        "4.5.6-rc.2+build.3"
    );
}
