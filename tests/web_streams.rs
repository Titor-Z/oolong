mod common;

// ── CountQueuingStrategy ─────────────────────────────────────────────────────

#[test]
fn test_count_strategy_high_water_mark() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"let s = new CountQueuingStrategy({ highWaterMark: 5 });
globalThis.r = s.highWaterMark;"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "5");
}

#[test]
fn test_count_strategy_size() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"let s = new CountQueuingStrategy({ highWaterMark: 3 });
globalThis.r = s.size("hello");"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "1");
}

#[test]
fn test_count_strategy_size_any_chunk() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"let s = new CountQueuingStrategy({ highWaterMark: 10 });
globalThis.r = s.size(new Uint8Array(100)) + " " + s.size(null) + " " + s.size(undefined);"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "1 1 1");
}

#[test]
fn test_count_strategy_global_exists() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"globalThis.r = typeof CountQueuingStrategy;"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "function");
}

// ── ByteLengthQueuingStrategy ────────────────────────────────────────────────

#[test]
fn test_byte_length_strategy_high_water_mark() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"let s = new ByteLengthQueuingStrategy({ highWaterMark: 1024 });
globalThis.r = s.highWaterMark;"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "1024");
}

#[test]
fn test_byte_length_strategy_size_uint8array() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"let s = new ByteLengthQueuingStrategy({ highWaterMark: 1024 });
globalThis.r = s.size(new Uint8Array(42));"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "42");
}

#[test]
fn test_byte_length_strategy_size_string() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"let s = new ByteLengthQueuingStrategy({ highWaterMark: 1024 });
globalThis.r = s.size("hello");"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    // String has no byteLength property → returns 1
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "1");
}

#[test]
fn test_byte_length_strategy_size_arraybuffer() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"let s = new ByteLengthQueuingStrategy({ highWaterMark: 1024 });
let buf = new ArrayBuffer(128);
globalThis.r = s.size(buf);"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "128");
}

#[test]
fn test_byte_length_strategy_global_exists() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"globalThis.r = typeof ByteLengthQueuingStrategy;"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "function");
}

// ── ReadableStream ───────────────────────────────────────────────────────────

#[test]
fn test_readable_stream_global_exists() {
    let mut rt = common::create_runtime();
    rt.eval_script("globalThis.r = typeof ReadableStream;")
        .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "function");
}

#[test]
fn test_readable_stream_constructor() {
    let mut rt = common::create_runtime();
    rt.eval_script(
        r#"let s = new ReadableStream();
globalThis.r = s instanceof ReadableStream;"#,
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

#[test]
fn test_readable_stream_locked() {
    let mut rt = common::create_runtime();
    rt.eval_script(
        r#"let s = new ReadableStream();
globalThis.r = s.locked;"#,
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "false");
}

#[test]
fn test_readable_stream_locked_after_get_reader() {
    let mut rt = common::create_runtime();
    rt.eval_script(
        r#"let s = new ReadableStream();
s.getReader();
globalThis.r = s.locked;"#,
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

#[test]
fn test_readable_stream_get_reader_returns_reader() {
    let mut rt = common::create_runtime();
    rt.eval_script(
        r#"let s = new ReadableStream();
let r = s.getReader();
globalThis.r = typeof r.read;"#,
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "function");
}

#[test]
fn test_readable_stream_enqueue_and_read() {
    let mut rt = common::create_runtime();
    rt.eval_script(
        r#"let s = new ReadableStream({
  start(ctrl) { ctrl.enqueue("hello"); ctrl.enqueue("world"); }
});
let r = s.getReader();
let a = r.read();
let b = r.read();
globalThis.r = a.value + " " + b.value;"#,
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "hello world");
}

#[test]
fn test_readable_stream_close_and_read() {
    let mut rt = common::create_runtime();
    rt.eval_script(
        r#"let s = new ReadableStream({
  start(ctrl) { ctrl.enqueue("data"); ctrl.close(); }
});
let r = s.getReader();
let first = r.read();
let second = r.read();
globalThis.r = first.value + "|" + second.done;"#,
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "data|true");
}

#[test]
fn test_readable_stream_no_source() {
    let mut rt = common::create_runtime();
    let result = rt.eval_script(
        r#"let s = new ReadableStream();
globalThis.r = typeof s.getReader;"#,
    );
    assert!(result.is_ok());
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "function");
}

// ── ReadableStreamDefaultController ──────────────────────────────────────────

#[test]
fn test_readable_stream_controller_global_exists() {
    let mut rt = common::create_runtime();
    rt.eval_script("globalThis.r = typeof ReadableStreamDefaultController;")
        .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "function");
}

#[test]
fn test_readable_stream_controller_desired_size() {
    let mut rt = common::create_runtime();
    rt.eval_script(
        r#"let s = new ReadableStream({
  start(ctrl) {
    globalThis.r = typeof ctrl.desiredSize;
    ctrl.enqueue("a");
  }
});
s.getReader();"#,
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "number");
}

// ── ReadableStreamDefaultReader ──────────────────────────────────────────────

#[test]
fn test_readable_stream_reader_global_exists() {
    let mut rt = common::create_runtime();
    rt.eval_script("globalThis.r = typeof ReadableStreamDefaultReader;")
        .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "function");
}

#[test]
fn test_readable_stream_reader_read_after_release() {
    let mut rt = common::create_runtime();
    rt.eval_script(
        r#"let s = new ReadableStream({ start(ctrl) { ctrl.enqueue("x"); } });
let r = s.getReader();
r.releaseLock();
// releaseLock 断开读取器，后续 read 返回 { done: false, value: undefined }
let result = r.read();
globalThis.r = result.value === undefined;"#,
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

// ── 类型一致性校验 ─────────────────────────────────────────────────

#[test]
fn test_type_consistency_web_streams() {
    let mut rt = common::create_runtime();
    rt.eval_script(
        r#"
let classes = [
  "CountQueuingStrategy", "ByteLengthQueuingStrategy",
  "ReadableStream", "ReadableStreamDefaultReader", "ReadableStreamDefaultController"
];
let results = classes.map(c => ({ name: c, type: typeof globalThis[c] }));
globalThis.r = JSON.stringify(results);
"#,
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    assert!(
        r.contains(r#""type":"function""#),
        "所有 streams 全局类应为 function: {r}"
    );
}
