mod common;
use std::path::Path;

// ── node:url ─────────────────────────────────────────────────────

#[test]
fn test_node_url_default_import() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import url from "node:url";
globalThis.r = typeof url.URL;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "function");
}

#[test]
fn test_node_url_named_exports() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { URL, URLSearchParams, fileURLToPath, pathToFileURL } from "node:url";
globalThis.r =
  typeof URL === "function" &&
  typeof fileURLToPath === "function" &&
  typeof pathToFileURL === "function";"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

#[test]
fn test_node_url_file_url_to_path() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { fileURLToPath } from "node:url";
globalThis.r = fileURLToPath("file:///usr/local/bin");"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "/usr/local/bin");
}

#[test]
fn test_node_url_path_to_file_url() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { pathToFileURL } from "node:url";
const u = pathToFileURL("/usr/local/bin");
globalThis.r = u.href;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(
        rt.eval_script("globalThis.r").unwrap(),
        "file:///usr/local/bin"
    );
}

#[test]
fn test_node_url_url_class() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { URL } from "node:url";
const u = new URL("https://example.com:8080/path?q=1#hash");
globalThis.r = u.hostname + ":" + u.port;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "example.com:8080");
}
