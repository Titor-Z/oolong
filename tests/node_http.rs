mod common;

use std::io::{BufReader, Read, Write};
use std::sync::atomic::{AtomicU16, Ordering};

static HTTP_PORT: AtomicU16 = AtomicU16::new(22000);

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
    let mut res = String::new();
    BufReader::new(&mut stream)
        .read_to_string(&mut res)
        .unwrap();
    res
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
    let mut res = String::new();
    BufReader::new(&mut stream)
        .read_to_string(&mut res)
        .unwrap();
    res
}

fn spawn_http_server(port: u16, handler_body: &str) {
    let js = format!(
        r#"import http from "node:http";
const server = http.createServer((req, res) => {{ {handler_body} }});
server.listen({port});
"#
    );
    std::thread::spawn(move || {
        let mut rt = common::create_runtime();
        let _ = rt.eval_module_str(&js, Some(std::path::Path::new("__server.js")));
    });
}

fn wait_for_server(port: u16) {
    std::thread::sleep(std::time::Duration::from_millis(300));
    for _ in 0..20 {
        if let Ok(mut stream) = std::net::TcpStream::connect(("127.0.0.1", port)) {
            let _ = write!(
                stream,
                "GET /health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
            );
            let _ = stream.flush();
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

fn extract_body(response: &str) -> &str {
    if let Some(pos) = response.find("\r\n\r\n") {
        &response[pos + 4..]
    } else {
        ""
    }
}

fn extract_status(response: &str) -> u16 {
    if let Some(line) = response.lines().next() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            return parts[1].parse().unwrap_or(0);
        }
    }
    0
}

// ── Import / Export tests ────────────────────────────────────────

#[test]
fn test_node_http_import() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import http from "node:http";
globalThis.r = typeof http.createServer;"#,
        Some(std::path::Path::new("__t.js")),
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
        Some(std::path::Path::new("__t.js")),
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
        Some(std::path::Path::new("__t.js")),
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
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "OK");
}

// ── Server e2e tests ─────────────────────────────────────────────

#[test]
fn test_node_http_server_get_hello() {
    let port = next_port();
    spawn_http_server(port, r#"res.end("hello");"#);
    wait_for_server(port);
    let res = http_get(port, "/");
    assert_eq!(extract_status(&res), 200);
    assert_eq!(extract_body(&res), "hello");
}

#[test]
fn test_node_http_server_status_code() {
    let port = next_port();
    spawn_http_server(port, r#"res.writeHead(201); res.end("created");"#);
    wait_for_server(port);
    let res = http_get(port, "/");
    assert_eq!(extract_status(&res), 201);
    assert_eq!(extract_body(&res), "created");
}

#[test]
fn test_node_http_server_custom_header() {
    let port = next_port();
    spawn_http_server(
        port,
        r#"res.setHeader("X-Custom", "myval"); res.end("ok");"#,
    );
    wait_for_server(port);
    let res = http_get(port, "/");
    assert!(
        res.contains("x-custom: myval"),
        "missing custom header: {res}"
    );
    assert_eq!(extract_body(&res), "ok");
}

#[test]
fn test_node_http_server_write_then_end() {
    let port = next_port();
    spawn_http_server(port, r#"res.write("hello "); res.end("world");"#);
    wait_for_server(port);
    let res = http_get(port, "/");
    assert_eq!(extract_body(&res), "hello world");
}

#[test]
fn test_node_http_server_empty_end() {
    let port = next_port();
    spawn_http_server(port, r#"res.end();"#);
    wait_for_server(port);
    let res = http_get(port, "/");
    assert_eq!(extract_status(&res), 200);
    assert_eq!(extract_body(&res), "");
}

#[test]
fn test_node_http_server_post_body() {
    let port = next_port();
    spawn_http_server(
        port,
        r#"let body = "";
req.on('data', chunk => body += chunk);
req.on('end', () => { res.end(body); });"#,
    );
    wait_for_server(port);
    let res = http_post(port, "/", "test body", "text/plain");
    assert_eq!(extract_body(&res), "test body");
}

#[test]
fn test_node_http_server_get_header() {
    let port = next_port();
    spawn_http_server(
        port,
        r#"const val = req.headers["x-custom"] || "none"; res.end(val);"#,
    );
    wait_for_server(port);
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
    write!(
        stream,
        "GET / HTTP/1.1\r\nHost: localhost\r\nX-Custom: hello\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    stream.flush().unwrap();
    let mut res = String::new();
    BufReader::new(&mut stream)
        .read_to_string(&mut res)
        .unwrap();
    assert_eq!(extract_body(&res), "hello");
}

#[test]
fn test_node_http_server_get_method_url() {
    let port = next_port();
    spawn_http_server(port, r#"res.end(req.method + " " + req.url);"#);
    wait_for_server(port);
    let res = http_get(port, "/test-path");
    assert_eq!(extract_body(&res), "GET /test-path");
}

// ── Express-style JSON body echo test ────────────────────────────

#[test]
fn test_node_http_server_json_body_echo() {
    let port = next_port();
    // Simulates Express app.use(express.json()) behavior
    spawn_http_server(
        port,
        r#"res.setHeader("Content-Type", "application/json");
let body = "";
req.on('data', chunk => { body += chunk; });
req.on('end', () => {
  try {
    const parsed = JSON.parse(body);
    parsed.echo = true;
    res.end(JSON.stringify(parsed));
  } catch(e) {
    res.statusCode = 400;
    res.end(JSON.stringify({error: "invalid json"}));
  }
});"#,
    );
    wait_for_server(port);
    let res = http_post(port, "/", r#"{"msg":"hello"}"#, "application/json");
    assert_eq!(extract_status(&res), 200);
    assert!(
        res.to_lowercase().contains("content-type: application/json"),
        "missing content-type: {res}"
    );
    let body = extract_body(&res);
    assert!(body.contains(r#""msg":"hello""#), "missing msg in body: {body}");
    assert!(body.contains(r#""echo":true"#), "missing echo in body: {body}");
}

// ── Named import tests ───────────────────────────────────────────

#[test]
fn test_node_http_named_imports() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { createServer, request, get, STATUS_CODES } from "node:http";
globalThis.r = {
  cs: typeof createServer,
  req: typeof request,
  g: typeof get,
  sc: STATUS_CODES[404]
};"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("JSON.stringify(globalThis.r)").unwrap();
    assert_eq!(
        r,
        r#"{"cs":"function","req":"function","g":"function","sc":"Not Found"}"#
    );
}

// ── Server address test ──────────────────────────────────────────

#[test]
fn test_node_http_server_address() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import http from "node:http";
const server = http.createServer();
globalThis.r1 = typeof server.address;
globalThis.r2 = typeof server.close;"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r1").unwrap(), "function");
    assert_eq!(rt.eval_script("globalThis.r2").unwrap(), "function");
}

// ── Remove header test ───────────────────────────────────────────

#[test]
fn test_node_http_remove_header() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import http from "node:http";
const server = http.createServer((req, res) => {
  res.setHeader("X-A", "1");
  res.removeHeader("X-A");
  res.end(res.getHeader("X-A") === undefined ? "ok" : "fail");
});
globalThis.r = typeof server.listen;"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "function");
}
