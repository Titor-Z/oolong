mod common;
use std::path::Path;
// ── node:tty ─────────────────────────────────────────────────────────────────

#[test]
fn test_node_tty_isatty_function() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { isatty } from "node:tty";
globalThis.r = typeof isatty === "function";"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

#[test]
fn test_node_tty_isatty_fd() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { isatty } from "node:tty";
// stdout in test runner likely not a TTY
globalThis.r = isatty(1);"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    // In test runner, stdout may or may not be a TTY - just check it returns a boolean
    let r = rt.eval_script("globalThis.r").unwrap();
    assert!(r == "true" || r == "false", "expected boolean, got {r}");
}

#[test]
fn test_node_tty_write_stream() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { WriteStream } from "node:tty";
var ws = new WriteStream(1);
globalThis.r = ws.isTTY === true && typeof ws.getWindowSize === "function" && typeof ws.setRawMode === "function";"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

#[test]
fn test_node_tty_read_stream() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { ReadStream } from "node:tty";
var rs = new ReadStream(0);
globalThis.r = rs.isTTY === true && typeof rs.setRawMode === "function";"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

#[test]
fn test_node_tty_default_import() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import tty from "node:tty";
globalThis.r = typeof tty.isatty === "function" && typeof tty.WriteStream === "function";"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}
