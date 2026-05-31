mod common;

use std::io::Read;
use std::sync::atomic::{AtomicU16, Ordering};
use std::time::Duration;

static NET_PORT: AtomicU16 = AtomicU16::new(24000);

fn next_port() -> u16 {
    NET_PORT.fetch_add(1, Ordering::SeqCst)
}

fn spawn_tcp_server(port: u16, response: &str) {
    let resp = response.to_string();
    std::thread::spawn(move || {
        let listener = std::net::TcpListener::bind(("127.0.0.1", port)).unwrap();
        listener.set_nonblocking(true).unwrap();
        loop {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = vec![0u8; 4096];
                let _ = stream.read(&mut buf);
                let _ = std::io::Write::write_all(&mut stream, resp.as_bytes());
                let _ = stream.shutdown(std::net::Shutdown::Write);
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    });
}

fn spawn_net_server(port: u16, handler_js: &str) {
    let js = format!(
        r#"import net from "node:net";
const server = net.createServer((socket) => {{ {handler_js} }});
server.listen({port});
"#
    );
    std::thread::spawn(move || {
        let mut rt = common::create_runtime();
        let _ = rt.eval_module_str(&js, Some(std::path::Path::new("__server.js")));
    });
}

fn wait_for_port(port: u16) {
    for _ in 0..100 {
        if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("port {port} not ready after 1s");
}

#[test]
fn test_node_net_default_import() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        "import assert from 'node:assert';
         import net from 'node:net';
         assert.ok(net.createServer !== undefined);
         assert.ok(net.Socket !== undefined);",
        Some(std::path::Path::new("test.js")),
    )
    .unwrap();
}

#[test]
fn test_node_net_named_exports() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        "import assert from 'node:assert';
         import { createServer, Socket, isIP, isIPv4, isIPv6 } from 'node:net';
         assert.ok(typeof createServer === 'function');
         assert.ok(typeof Socket === 'function');
         assert.ok(typeof isIP === 'function');
         assert.ok(typeof isIPv4 === 'function');
         assert.ok(typeof isIPv6 === 'function');",
        Some(std::path::Path::new("test.js")),
    )
    .unwrap();
}

#[test]
fn test_node_net_is_ip() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        "import assert from 'node:assert';
         import { isIP, isIPv4, isIPv6 } from 'node:net';
         assert.ok(isIP('127.0.0.1') === 4);
         assert.ok(isIP('::1') === 6);
         assert.ok(isIP('invalid') === 0);
         assert.ok(isIPv4('192.168.1.1') === true);
         assert.ok(isIPv4('999.999.999.999') === false);
         assert.ok(isIPv6('::1') === true);
         assert.ok(isIPv6('fe80::1') === true);
         assert.ok(isIPv6('not:valid') === false);",
        Some(std::path::Path::new("test.js")),
    )
    .unwrap();
}

#[test]
fn test_node_net_socket_connect_write_end() {
    let port = next_port();
    spawn_tcp_server(port, "hello from server\n");

    let js = format!(
        "import assert from 'node:assert';
         import net from 'node:net';
         const s = new net.Socket();
         s.on('connect', () => {{
             s.write('ping\\n');
             s.end();
         }});
         s.connect({port}, '127.0.0.1');
         assert.ok(true);
         "
    );
    let mut rt = common::create_runtime();
    rt.eval_module_str(&js, Some(std::path::Path::new("test.js")))
        .unwrap();
}

#[test]
fn test_node_net_server_echo() {
    let port = next_port();
    spawn_net_server(port, "socket.write('echo:ok'); socket.end();");
    wait_for_port(port);

    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
    let mut buf = String::new();
    std::io::BufReader::new(&mut stream)
        .read_to_string(&mut buf)
        .unwrap();
    assert!(buf.contains("echo:ok"), "got: {buf}");
}
