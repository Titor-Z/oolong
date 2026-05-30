mod common;
use std::path::Path;
// ── Phase 5.5: node:crypto ────────────────────────────────────────────────

#[test]
fn test_node_crypto_create_hash_sha256() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { createHash } from "node:crypto";
const h = createHash("sha256");
h.update("hello");
globalThis.r = h.digest("hex");"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let result = rt.eval_script("globalThis.r").unwrap();
    // sha256("hello") = 2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824
    assert_eq!(
        result,
        "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
    );
}

#[test]
fn test_node_crypto_create_hash_md5() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { createHash } from "node:crypto";
const h = createHash("md5");
h.update("hello");
globalThis.r = h.digest("hex");"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let result = rt.eval_script("globalThis.r").unwrap();
    // md5("hello") = 5d41402abc4b2a76b9719d911017c592
    assert_eq!(result, "5d41402abc4b2a76b9719d911017c592");
}

#[test]
fn test_node_crypto_random_uuid() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { randomUUID } from "node:crypto";
globalThis.r = randomUUID();"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let result = rt.eval_script("globalThis.r").unwrap();
    // UUID format: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
    assert_eq!(result.len(), 36);
    assert_eq!(&result[8..9], "-");
    assert_eq!(&result[13..14], "-");
    assert_eq!(&result[18..19], "-");
    assert_eq!(&result[23..24], "-");
}

#[test]
fn test_node_crypto_random_bytes() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { randomBytes } from "node:crypto";
const buf = randomBytes(16);
globalThis.r = buf.length;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let result = rt.eval_script("globalThis.r").unwrap();
    assert_eq!(result, "16");
}

#[test]
fn test_node_crypto_default_import() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import crypto from "node:crypto";
const h = crypto.createHash("sha256");
h.update("world");
globalThis.r = h.digest("hex");"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let result = rt.eval_script("globalThis.r").unwrap();
    // sha256("world") = 486ea46224d1bb4fb680f34f7c9ad96a8f24ec88be73ea8e5a6c65260e9cb8a7
    assert_eq!(
        result,
        "486ea46224d1bb4fb680f34f7c9ad96a8f24ec88be73ea8e5a6c65260e9cb8a7"
    );
}

#[test]
fn test_node_crypto_hash_multiple_updates() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import { createHash } from "node:crypto";
const h = createHash("sha256");
h.update("hel");
h.update("lo");
globalThis.r = h.digest("hex");"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let result = rt.eval_script("globalThis.r").unwrap();
    // sha256("hello") same as single update
    assert_eq!(
        result,
        "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
    );
}
