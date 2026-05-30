mod common;
use std::path::Path;
// ── Phase 5.5: node:child_process ──────────────────────────────────────────

#[test]
fn test_node_child_process_exec_sync() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { execSync } from "node:child_process";
const out = execSync("echo hello");
globalThis.r = out.trim();"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let result = rt.eval_script("globalThis.r").unwrap();
    assert_eq!(result, "hello");
}

#[test]
fn test_node_child_process_spawn_sync() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { spawnSync } from "node:child_process";
const r = spawnSync("echo", ["hello"]);
globalThis.r = r.status + ":" + r.stdout.trim();"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let result = rt.eval_script("globalThis.r").unwrap();
    assert_eq!(result, "0:hello");
}

#[test]
fn test_node_child_process_default_import() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import cp from "node:child_process";
const out = cp.execSync("echo ok");
globalThis.r = out.trim();"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let result = rt.eval_script("globalThis.r").unwrap();
    assert_eq!(result, "ok");
}
