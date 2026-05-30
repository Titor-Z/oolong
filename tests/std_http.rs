mod common;

#[test]
fn test_http_serve_default_import() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import http from "@std/http";
globalThis.r = typeof http.serve;"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "function");
}

#[test]
fn test_http_serve_named_import() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { serve } from "@std/http";
globalThis.r = typeof serve;"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "function");
}

// ── .d.ts 类型一致性校验 ──────────────────────────────────────────
// 验证 Rust 实现的导出与 types/ 中的声明一致

#[test]
fn test_type_consistency_std_http() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import http from "@std/http";
globalThis._names = Object.keys(http).sort();"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    let names = rt.eval_script("JSON.stringify(globalThis._names)").unwrap();
    assert_eq!(
        names, r#"["serve"]"#,
        "@std/http 导出名与 types/std/http.d.ts 不一致"
    );
}

#[test]
fn test_type_consistency_std_path() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import path from "@std/path";
globalThis._names = Object.keys(path).sort();"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    let names = rt.eval_script("JSON.stringify(globalThis._names)").unwrap();
    // 根据 types/std/path.d.ts 列出的导出
    let expected = r#"["basename","delimiter","dirname","extname","format","isAbsolute","join","normalize","parse","relative","resolve","sep"]"#;
    assert_eq!(
        names, expected,
        "@std/path 导出名与 types/std/path.d.ts 不一致"
    );
}

#[test]
fn test_type_consistency_std_process() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import proc from "@std/process";
globalThis._keys = Object.keys(proc).sort();
globalThis._types = {};
for (const k of Object.keys(proc)) {
  globalThis._types[k] = typeof proc[k];
}"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    let types_json = rt.eval_script("JSON.stringify(globalThis._types)").unwrap();
    let types: serde_json::Value = serde_json::from_str(&types_json).unwrap();
    let obj = types.as_object().unwrap();

    // 验证函数类型导出
    assert_eq!(
        obj.get("cwd").and_then(|v| v.as_str()),
        Some("function"),
        "process.cwd 应为 function"
    );
    assert_eq!(
        obj.get("chdir").and_then(|v| v.as_str()),
        Some("function"),
        "process.chdir 应为 function"
    );
    assert_eq!(
        obj.get("exit").and_then(|v| v.as_str()),
        Some("function"),
        "process.exit 应为 function"
    );
    assert_eq!(
        obj.get("uptime").and_then(|v| v.as_str()),
        Some("function"),
        "process.uptime 应为 function"
    );
    assert_eq!(
        obj.get("memoryUsage").and_then(|v| v.as_str()),
        Some("function"),
        "process.memoryUsage 应为 function"
    );

    // 验证字符串/数字类型导出
    assert_eq!(
        obj.get("pid").and_then(|v| v.as_str()),
        Some("number"),
        "process.pid 应为 number"
    );
    assert_eq!(
        obj.get("ppid").and_then(|v| v.as_str()),
        Some("number"),
        "process.ppid 应为 number"
    );
    assert_eq!(
        obj.get("platform").and_then(|v| v.as_str()),
        Some("string"),
        "process.platform 应为 string"
    );
    assert_eq!(
        obj.get("arch").and_then(|v| v.as_str()),
        Some("string"),
        "process.arch 应为 string"
    );
}

#[test]
fn test_type_consistency_node_path() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import path from "node:path";
import { posix, win32 } from "node:path";
globalThis._names = Object.keys(path).sort();
globalThis._hasPosix = typeof posix === "object";
globalThis._hasWin32 = typeof win32 === "object";
globalThis._posixSep = posix && posix.sep;
globalThis._win32Sep = win32 && win32.sep;"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    let names = rt.eval_script("JSON.stringify(globalThis._names)").unwrap();
    let expected = r#"["basename","delimiter","dirname","extname","format","isAbsolute","join","normalize","parse","relative","resolve","sep","toNamespacedPath"]"#;
    assert_eq!(names, expected, "node:path 默认导出 API 列表不匹配");
    assert_eq!(
        rt.eval_script("globalThis._hasPosix").unwrap(),
        "true",
        "node:path 应有 posix 命名导出"
    );
    assert_eq!(
        rt.eval_script("globalThis._hasWin32").unwrap(),
        "true",
        "node:path 应有 win32 命名导出"
    );
    assert_eq!(
        rt.eval_script("globalThis._posixSep").unwrap(),
        "/",
        "posix.sep 应为 /"
    );
    assert_eq!(
        rt.eval_script("globalThis._win32Sep").unwrap(),
        "\\",
        "win32.sep 应为 \\"
    );
}

// ── HTTP serve integration tests ─────────────────────────────────
// Phase A: @std/http server

use std::io::{BufReader, Read, Write as IoWrite};
use std::sync::atomic::{AtomicU16, Ordering};

static HTTP_PORT: AtomicU16 = AtomicU16::new(21000);

fn next_port() -> u16 {
    HTTP_PORT.fetch_add(1, Ordering::SeqCst)
}

fn http_get(port: u16, path: &str) -> String {
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    stream.flush().unwrap();
    let mut response = String::new();
    BufReader::new(&mut stream)
        .read_to_string(&mut response)
        .unwrap();
    response
}

fn http_post(port: u16, path: &str, body: &str, content_type: &str) -> String {
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
    write!(
        stream,
        "POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .unwrap();
    stream.flush().unwrap();
    let mut response = String::new();
    BufReader::new(&mut stream)
        .read_to_string(&mut response)
        .unwrap();
    response
}

fn spawn_serve(port: u16, handler_body: &str) {
    let dir = std::env::temp_dir().join(format!("oolong_http_{port}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let code = format!(
        r#"import {{ serve }} from "@std/http";
        serve({{
            port: {port},
            hostname: "127.0.0.1",
            handler: async (req) => {{{handler_body}}}
        }});"#
    );
    let dir_c = dir.clone();

    std::thread::spawn(move || {
        let mut rt = oolong::runtime::OolongRuntime::new(&dir_c).unwrap();
        let _ = rt.eval_module_str(&code, Some(std::path::Path::new("__serve.js")));
    });
}

fn wait_for_server(port: u16) {
    std::thread::sleep(std::time::Duration::from_millis(300));
    for _ in 0..30 {
        if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

fn extract_body(response: &str) -> &str {
    response.split("\r\n\r\n").nth(1).unwrap_or("")
}

fn assert_status(response: &str, expected: u16) {
    let status_line = response.lines().next().unwrap_or("");
    assert!(
        status_line.contains(&format!(" {expected} ")),
        "expected status {expected}, got: {status_line}"
    );
}

// ── Basic GET ────────────────────────────────────────────────────

#[test]
fn test_http_serve_get_hello() {
    let port = next_port();
    spawn_serve(port, r#"return { status: 200, body: "Hello World" };"#);
    wait_for_server(port);
    let res = http_get(port, "/");
    assert_status(&res, 200);
    assert_eq!(extract_body(&res), "Hello World");
}

#[test]
fn test_http_serve_get_path() {
    let port = next_port();
    spawn_serve(port, r#"return { status: 200, body: req.url };"#);
    wait_for_server(port);
    let res = http_get(port, "/foo/bar");
    assert_eq!(extract_body(&res), "/foo/bar");
}

#[test]
fn test_http_serve_get_method() {
    let port = next_port();
    spawn_serve(port, r#"return { status: 200, body: req.method };"#);
    wait_for_server(port);
    let res = http_get(port, "/test");
    assert_eq!(extract_body(&res), "GET");
}

// ── POST ─────────────────────────────────────────────────────────

#[test]
fn test_http_serve_post_body() {
    let port = next_port();
    spawn_serve(
        port,
        r#"const text = await req.text(); return new Response(text);"#,
    );
    wait_for_server(port);
    let res = http_post(port, "/", "Hello POST", "text/plain");
    assert_status(&res, 200);
    assert_eq!(extract_body(&res), "Hello POST");
}

#[test]
fn test_http_serve_post_json() {
    let port = next_port();
    spawn_serve(
        port,
        r#"const text = await req.text(); return new Response(text);"#,
    );
    wait_for_server(port);
    let res = http_post(port, "/", r#"{"msg":"hi"}"#, "application/json");
    assert_eq!(extract_body(&res), r#"{"msg":"hi"}"#);
}

#[test]
fn test_http_serve_post_empty_body() {
    let port = next_port();
    spawn_serve(
        port,
        r#"const text = await req.text(); return new Response(text || "empty");"#,
    );
    wait_for_server(port);
    let res = http_get(port, "/");
    assert_eq!(extract_body(&res), "empty");
}

// ── Status codes ─────────────────────────────────────────────────

#[test]
fn test_http_serve_status_201() {
    let port = next_port();
    spawn_serve(port, r#"return new Response("created", { status: 201 });"#);
    wait_for_server(port);
    let res = http_get(port, "/");
    assert_status(&res, 201);
    assert_eq!(extract_body(&res), "created");
}

#[test]
fn test_http_serve_status_404() {
    let port = next_port();
    spawn_serve(
        port,
        r#"return new Response("not found", { status: 404 });"#,
    );
    wait_for_server(port);
    let res = http_get(port, "/missing");
    assert_status(&res, 404);
    assert_eq!(extract_body(&res), "not found");
}

#[test]
fn test_http_serve_status_500() {
    let port = next_port();
    spawn_serve(port, r#"return new Response("error", { status: 500 });"#);
    wait_for_server(port);
    let res = http_get(port, "/");
    assert_status(&res, 500);
}

#[test]
fn test_http_serve_status_204() {
    let port = next_port();
    spawn_serve(port, r#"return new Response(null, { status: 204 });"#);
    wait_for_server(port);
    let res = http_get(port, "/");
    assert_status(&res, 204);
}

#[test]
fn test_http_serve_status_301() {
    let port = next_port();
    spawn_serve(port, r#"return new Response("moved", { status: 301 });"#);
    wait_for_server(port);
    let res = http_get(port, "/");
    assert_status(&res, 301);
}

// ── Content-Type ─────────────────────────────────────────────────

#[test]
fn test_http_serve_content_type_default() {
    let port = next_port();
    spawn_serve(port, r#"return new Response("ok");"#);
    wait_for_server(port);
    let res = http_get(port, "/");
    assert!(
        res.contains("Content-Type: text/plain; charset=utf-8"),
        "expected default content-type"
    );
}

#[test]
fn test_http_serve_content_type_custom() {
    let port = next_port();
    spawn_serve(
        port,
        r#"return new Response("{}", { headers: { "Content-Type": "application/json" } });"#,
    );
    wait_for_server(port);
    let res = http_get(port, "/");
    assert!(res.contains("Content-Type: application/json"), "got: {res}");
}

// ── Headers ──────────────────────────────────────────────────────

#[test]
fn test_http_serve_request_headers() {
    let port = next_port();
    spawn_serve(
        port,
        r#"return new Response(req.headers.get("X-Custom") ?? "none");"#,
    );
    wait_for_server(port);
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
    write!(
        stream,
        "GET / HTTP/1.1\r\nHost: localhost\r\nX-Custom: myval\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    stream.flush().unwrap();
    let mut res = String::new();
    BufReader::new(&mut stream)
        .read_to_string(&mut res)
        .unwrap();
    assert_eq!(extract_body(&res), "myval");
}

// ─── Handler error ───────────────────────────────────────────────

#[test]
fn test_http_serve_handler_error_caught() {
    let port = next_port();
    spawn_serve(port, r#"throw new Error("boom");"#);
    wait_for_server(port);
    let res = http_get(port, "/");
    assert_status(&res, 500);
}

// ── String return (auto-body) ────────────────────────────────────

#[test]
fn test_http_serve_return_string() {
    let port = next_port();
    spawn_serve(port, r#"return "string body";"#);
    wait_for_server(port);
    let res = http_get(port, "/");
    assert_status(&res, 200);
    assert_eq!(extract_body(&res), "string body");
}

// ── Response with new Response() ─────────────────────────────────

#[test]
fn test_http_serve_new_response() {
    let port = next_port();
    spawn_serve(port, r#"return new Response("Hello from Response");"#);
    wait_for_server(port);
    let res = http_get(port, "/");
    assert_status(&res, 200);
    assert_eq!(extract_body(&res), "Hello from Response");
}
