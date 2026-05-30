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

// ── 类型一致性校验 ─────────────────────────────────────────────────

#[test]
fn test_type_consistency_web_streams() {
    let mut rt = common::create_runtime();
    rt.eval_script(
        r#"
let classes = ["CountQueuingStrategy", "ByteLengthQueuingStrategy"];
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
