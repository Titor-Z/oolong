mod common;
use std::path::Path;

#[test]
fn test_node_http_import() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import http from "node:http";
globalThis.r = typeof http.createServer;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "function");
}

#[test]
fn test_node_http_create_server() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import http from "node:http";
const server = http.createServer((req, res) => { res.end("ok"); });
globalThis.r = typeof server.listen;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "function");
}

#[test]
fn test_node_net_import() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import net from "node:net";
globalThis.r = typeof net.createServer;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "function");
}

#[test]
fn test_node_http_status_codes() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { STATUS_CODES } from "node:http";
globalThis.r = STATUS_CODES[200];"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "OK");
}
