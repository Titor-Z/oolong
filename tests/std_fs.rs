mod common;

// ── fs 模块 ─────────────────────────────────────────────────────────────────

fn fs_test_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn test_fs_write_text_file() {
    let dir = fs_test_dir("oolong_fs_write_text");
    let mut rt = common::create_runtime();
    let p = dir.join("hello.txt").to_string_lossy().to_string();
    rt.eval_module_str(
        &format!(
            r#"import {{ writeTextFile }} from "@std/fs";
await writeTextFile({p:?}, "hello world");
globalThis.r = "ok";"#
        ),
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    assert_eq!(r, "ok");
    let content = std::fs::read_to_string(dir.join("hello.txt")).unwrap();
    assert_eq!(content, "hello world");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_fs_read_text_file() {
    let dir = fs_test_dir("oolong_fs_read_text");
    let p = dir.join("data.txt");
    std::fs::write(&p, "hello fs").unwrap();
    let mut rt = common::create_runtime();
    let ps = p.to_string_lossy().to_string();
    rt.eval_module_str(
        &format!(
            r#"import {{ readTextFile }} from "@std/fs";
const content = await readTextFile({ps:?});
globalThis.r = content;"#
        ),
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    assert_eq!(r, "hello fs");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_fs_exists_true() {
    let dir = fs_test_dir("oolong_fs_exists");
    std::fs::write(dir.join("x.txt"), "").unwrap();
    let mut rt = common::create_runtime();
    let ps = dir.join("x.txt").to_string_lossy().to_string();
    rt.eval_module_str(
        &format!(
            r#"import {{ exists }} from "@std/fs";
const e = await exists({ps:?});
globalThis.r = e;"#
        ),
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_fs_exists_false() {
    let dir = fs_test_dir("oolong_fs_exists_false");
    let mut rt = common::create_runtime();
    let ps = dir.join("nonexistent.txt").to_string_lossy().to_string();
    rt.eval_module_str(
        &format!(
            r#"import {{ exists }} from "@std/fs";
const e = await exists({ps:?});
globalThis.r = e;"#
        ),
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "false");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_fs_read_file_sync() {
    let dir = fs_test_dir("oolong_fs_read_sync");
    let p = dir.join("data.bin");
    std::fs::write(&p, b"binary\x00data").unwrap();
    let mut rt = common::create_runtime();
    let ps = p.to_string_lossy().to_string();
    rt.eval_module_str(
        &format!(
            r#"import {{ readFileSync }} from "@std/fs";
globalThis.r = readFileSync({ps:?}).byteLength;"#
        ),
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    let len: usize = r.parse().unwrap_or(0);
    assert_eq!(
        len, 11,
        "readFileSync should return ArrayBuffer with correct byteLength; got {r}"
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_fs_read_file() {
    let dir = fs_test_dir("oolong_fs_read_file");
    let p = dir.join("data.bin");
    std::fs::write(&p, b"abc").unwrap();
    let mut rt = common::create_runtime();
    let ps = p.to_string_lossy().to_string();
    rt.eval_module_str(
        &format!(
            r#"import {{ readFile }} from "@std/fs";
const buf = await readFile({ps:?});
globalThis.r = buf.byteLength;"#
        ),
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "3");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_fs_write_file() {
    let dir = fs_test_dir("oolong_fs_write_file");
    let mut rt = common::create_runtime();
    let ps = dir.join("out.txt").to_string_lossy().to_string();
    rt.eval_module_str(
        &format!(
            r#"import {{ writeFile }} from "@std/fs";
await writeFile({ps:?}, "write file ok");
globalThis.r = "done";"#
        ),
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    let content = std::fs::read_to_string(dir.join("out.txt")).unwrap();
    assert_eq!(content, "write file ok");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_fs_default_import() {
    let dir = fs_test_dir("oolong_fs_default");
    let p = dir.join("a.txt");
    std::fs::write(&p, "hi").unwrap();
    let mut rt = common::create_runtime();
    let ps = p.to_string_lossy().to_string();
    rt.eval_module_str(
        &format!(
            r#"import fs from "@std/fs";
const content = await fs.readTextFile({ps:?});
globalThis.r = content;"#
        ),
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "hi");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_fs_read_not_found() {
    let dir = fs_test_dir("oolong_fs_not_found");
    let mut rt = common::create_runtime();
    let ps = dir.join("no_such.txt").to_string_lossy().to_string();
    rt.eval_module_str(
        &format!(
            r#"import {{ readTextFile }} from "@std/fs";
try {{
  await readTextFile({ps:?});
  globalThis.r = "no_error";
}} catch(e) {{
  globalThis.r = "caught";
}}"#
        ),
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "caught");
    let _ = std::fs::remove_dir_all(dir);
}

// ── fs P1 ───────────────────────────────────────────────────────────────────

#[test]
fn test_fs_mkdir_remove() {
    let dir = fs_test_dir("oolong_fs_mkdir");
    let sub = dir.join("sub").to_string_lossy().to_string();
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        &format!(
            r#"import {{ mkdir, remove, exists }} from "@std/fs";
await mkdir({sub:?});
globalThis.r1 = await exists({sub:?});
await remove({sub:?}, {{ recursive: true }});
globalThis.r2 = await exists({sub:?});"#
        ),
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r1").unwrap(), "true");
    assert_eq!(rt.eval_script("globalThis.r2").unwrap(), "false");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_fs_mkdir_recursive() {
    let dir = fs_test_dir("oolong_fs_mkdir_rec");
    let nested = dir
        .join("a")
        .join("b")
        .join("c")
        .to_string_lossy()
        .to_string();
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        &format!(
            r#"import {{ mkdir, exists }} from "@std/fs";
await mkdir({nested:?}, {{ recursive: true }});
globalThis.r = await exists({nested:?});"#
        ),
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_fs_readdir() {
    let dir = fs_test_dir("oolong_fs_readdir");
    std::fs::write(dir.join("a.txt"), "").unwrap();
    std::fs::write(dir.join("b.txt"), "").unwrap();
    let mut rt = common::create_runtime();
    let ds = dir.to_string_lossy().to_string();
    rt.eval_module_str(
        &format!(
            r#"import {{ readdir }} from "@std/fs";
const files = await readdir({ds:?});
globalThis.r = files.sort().join(",");"#
        ),
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "a.txt,b.txt");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_fs_stat() {
    let dir = fs_test_dir("oolong_fs_stat");
    let p = dir.join("f.txt");
    std::fs::write(&p, "hello").unwrap();
    let mut rt = common::create_runtime();
    let ps = p.to_string_lossy().to_string();
    rt.eval_module_str(
        &format!(
            r#"import {{ stat }} from "@std/fs";
const s = await stat({ps:?});
globalThis.r1 = s.isFile;
globalThis.r2 = s.isDirectory;
globalThis.r3 = s.size;"#
        ),
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r1").unwrap(), "true");
    assert_eq!(rt.eval_script("globalThis.r2").unwrap(), "false");
    assert_eq!(rt.eval_script("globalThis.r3").unwrap(), "5");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_fs_append_file() {
    let dir = fs_test_dir("oolong_fs_append");
    let p = dir.join("log.txt").to_string_lossy().to_string();
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        &format!(
            r#"import {{ appendFile }} from "@std/fs";
await appendFile({p:?}, "line1\n");
await appendFile({p:?}, "line2\n");
globalThis.r = "ok";"#
        ),
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    let content = std::fs::read_to_string(dir.join("log.txt")).unwrap();
    assert_eq!(content, "line1\nline2\n");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_fs_copy_file() {
    let dir = fs_test_dir("oolong_fs_copy");
    let src = dir.join("src.txt");
    std::fs::write(&src, "copy me").unwrap();
    let mut rt = common::create_runtime();
    let ss = src.to_string_lossy().to_string();
    let ds = dir.join("dst.txt").to_string_lossy().to_string();
    rt.eval_module_str(
        &format!(
            r#"import {{ copyFile }} from "@std/fs";
await copyFile({ss:?}, {ds:?});
globalThis.r = "ok";"#
        ),
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    let content = std::fs::read_to_string(dir.join("dst.txt")).unwrap();
    assert_eq!(content, "copy me");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_fs_rename() {
    let dir = fs_test_dir("oolong_fs_rename");
    std::fs::write(dir.join("old.txt"), "rename").unwrap();
    let mut rt = common::create_runtime();
    let old = dir.join("old.txt").to_string_lossy().to_string();
    let new = dir.join("new.txt").to_string_lossy().to_string();
    rt.eval_module_str(
        &format!(
            r#"import {{ rename, exists }} from "@std/fs";
await rename({old:?}, {new:?});
globalThis.r1 = await exists({old:?});
globalThis.r2 = await exists({new:?});"#
        ),
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r1").unwrap(), "false");
    assert_eq!(rt.eval_script("globalThis.r2").unwrap(), "true");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_fs_realpath() {
    let dir = fs_test_dir("oolong_fs_realpath");
    std::fs::write(dir.join("link_target.txt"), "real").unwrap();
    let mut rt = common::create_runtime();
    let ps = dir.join("link_target.txt").to_string_lossy().to_string();
    rt.eval_module_str(
        &format!(
            r#"import {{ realpath }} from "@std/fs";
const r = await realpath({ps:?});
globalThis.r = r.endsWith("link_target.txt");"#
        ),
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
    let _ = std::fs::remove_dir_all(dir);
}

#[cfg(unix)]
#[test]
fn test_fs_symlink() {
    let dir = fs_test_dir("oolong_fs_symlink");
    let target = dir.join("target.txt");
    std::fs::write(&target, "link").unwrap();
    let mut rt = common::create_runtime();
    let ts = target.to_string_lossy().to_string();
    let ls = dir.join("link.txt").to_string_lossy().to_string();
    rt.eval_module_str(
        &format!(
            r#"import {{ symlink, stat, lstat }} from "@std/fs";
await symlink({ts:?}, {ls:?});
globalThis.r1 = (await stat({ls:?})).isFile;
globalThis.r2 = (await lstat({ls:?})).isSymlink;"#
        ),
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r1").unwrap(), "true");
    assert_eq!(rt.eval_script("globalThis.r2").unwrap(), "true");
    let _ = std::fs::remove_dir_all(dir);
}

// ── fs P2 ───────────────────────────────────────────────────────────────────

#[test]
fn test_fs_exists_sync() {
    let dir = fs_test_dir("oolong_fs_exists_sync");
    std::fs::write(dir.join("x.txt"), "").unwrap();
    let mut rt = common::create_runtime();
    let ps = dir.join("x.txt").to_string_lossy().to_string();
    let ns = dir.join("none").to_string_lossy().to_string();
    rt.eval_module_str(
        &format!(
            r#"import {{ existsSync }} from "@std/fs";
globalThis.r1 = existsSync({ps:?});
globalThis.r2 = existsSync({ns:?});"#
        ),
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r1").unwrap(), "true");
    assert_eq!(rt.eval_script("globalThis.r2").unwrap(), "false");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_fs_mkdir_remove_sync() {
    let dir = fs_test_dir("oolong_fs_mkdir_sync");
    let sub = dir.join("sub").to_string_lossy().to_string();
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        &format!(
            r#"import {{ mkdirSync, removeSync, existsSync }} from "@std/fs";
mkdirSync({sub:?});
globalThis.r1 = existsSync({sub:?});
removeSync({sub:?}, {{ recursive: true }});
globalThis.r2 = existsSync({sub:?});"#
        ),
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r1").unwrap(), "true");
    assert_eq!(rt.eval_script("globalThis.r2").unwrap(), "false");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_fs_readdir_sync() {
    let dir = fs_test_dir("oolong_fs_readdir_sync");
    std::fs::write(dir.join("a.txt"), "").unwrap();
    std::fs::write(dir.join("b.txt"), "").unwrap();
    let mut rt = common::create_runtime();
    let ds = dir.to_string_lossy().to_string();
    rt.eval_module_str(
        &format!(
            r#"import {{ readdirSync }} from "@std/fs";
const files = readdirSync({ds:?});
globalThis.r = files.sort().join(",");"#
        ),
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "a.txt,b.txt");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_fs_stat_sync() {
    let dir = fs_test_dir("oolong_fs_stat_sync");
    std::fs::write(dir.join("f.txt"), "stat").unwrap();
    let mut rt = common::create_runtime();
    let ps = dir.join("f.txt").to_string_lossy().to_string();
    rt.eval_module_str(
        &format!(
            r#"import {{ statSync }} from "@std/fs";
const s = statSync({ps:?});
globalThis.r1 = s.isFile;
globalThis.r2 = s.size;"#
        ),
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r1").unwrap(), "true");
    assert_eq!(rt.eval_script("globalThis.r2").unwrap(), "4");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_fs_copy_rename_sync() {
    let dir = fs_test_dir("oolong_fs_cr_sync");
    std::fs::write(dir.join("s.txt"), "sync").unwrap();
    let mut rt = common::create_runtime();
    let s = dir.join("s.txt").to_string_lossy().to_string();
    let d = dir.join("d.txt").to_string_lossy().to_string();
    let r = dir.join("r.txt").to_string_lossy().to_string();
    rt.eval_module_str(
        &format!(
            r#"import {{ copyFileSync, renameSync, existsSync }} from "@std/fs";
copyFileSync({s:?}, {d:?});
renameSync({d:?}, {r:?});
globalThis.r1 = existsSync({s:?});
globalThis.r2 = existsSync({r:?});"#
        ),
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r1").unwrap(), "true");
    assert_eq!(rt.eval_script("globalThis.r2").unwrap(), "true");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_fs_append_file_sync() {
    let dir = fs_test_dir("oolong_fs_append_sync");
    let p = dir.join("log.txt").to_string_lossy().to_string();
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        &format!(
            r#"import {{ appendFileSync }} from "@std/fs";
appendFileSync({p:?}, "hi\n");
appendFileSync({p:?}, "ho\n");
globalThis.r = "ok";"#
        ),
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    let content = std::fs::read_to_string(dir.join("log.txt")).unwrap();
    assert_eq!(content, "hi\nho\n");
    let _ = std::fs::remove_dir_all(dir);
}

// ── fs P3 ───────────────────────────────────────────────────────────────────

#[cfg(unix)]
#[test]
fn test_fs_chmod() {
    let dir = fs_test_dir("oolong_fs_chmod");
    let p = dir.join("f.txt");
    std::fs::write(&p, "mod").unwrap();
    let mut rt = common::create_runtime();
    let ps = p.to_string_lossy().to_string();
    rt.eval_module_str(
        &format!(
            r#"import {{ chmod }} from "@std/fs";
await chmod({ps:?}, 0o600);
globalThis.r = "ok";"#
        ),
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "ok");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_fs_link() {
    let dir = fs_test_dir("oolong_fs_link");
    let original = dir.join("original.txt");
    std::fs::write(&original, "hard link").unwrap();
    let mut rt = common::create_runtime();
    let os = original.to_string_lossy().to_string();
    let ls = dir.join("hardlink.txt").to_string_lossy().to_string();
    rt.eval_module_str(
        &format!(
            r#"import {{ link, exists }} from "@std/fs";
await link({os:?}, {ls:?});
globalThis.r = await exists({ls:?});"#
        ),
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_fs_truncate() {
    let dir = fs_test_dir("oolong_fs_truncate");
    let p = dir.join("f.txt");
    std::fs::write(&p, "hello world").unwrap();
    let mut rt = common::create_runtime();
    let ps = p.to_string_lossy().to_string();
    rt.eval_module_str(
        &format!(
            r#"import {{ truncate, readTextFile }} from "@std/fs";
await truncate({ps:?}, 5);
const c = await readTextFile({ps:?});
globalThis.r = c;"#
        ),
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "hello");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_fs_access_ok() {
    let dir = fs_test_dir("oolong_fs_access_ok");
    std::fs::write(dir.join("f.txt"), "data").unwrap();
    let mut rt = common::create_runtime();
    let ps = dir.join("f.txt").to_string_lossy().to_string();
    rt.eval_module_str(
        &format!(
            r#"import {{ access }} from "@std/fs";
await access({ps:?});
globalThis.r = "ok";"#
        ),
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "ok");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_fs_access_enoent() {
    let dir = fs_test_dir("oolong_fs_access_enoent");
    let mut rt = common::create_runtime();
    let ps = dir.join("nonexistent").to_string_lossy().to_string();
    rt.eval_module_str(
        &format!(
            r#"import {{ access }} from "@std/fs";
try {{
  await access({ps:?});
  globalThis.r = "ok";
}} catch {{
  globalThis.r = "caught";
}}"#
        ),
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "caught");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_fs_watch_detects_change() {
    let dir = fs_test_dir("oolong_fs_watch");
    let p = dir.join("watch.txt");
    std::fs::write(&p, "before").unwrap();
    let ps = p.to_string_lossy().to_string();

    let mut rt = common::create_runtime();

    // 开始监视
    rt.eval_module_str(
        &format!(
            r#"import {{ watch }} from "@std/fs";
const w = watch({ps:?});
globalThis._w = w;"#
        ),
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();

    // 调用 next() 并挂载 .then 回调
    let _ = rt.eval_script(
        "globalThis._prom = globalThis._w.next();
globalThis._prom.then(v => { globalThis._result_value = v; });",
    );

    // 修改文件（确保 mtime 变化）
    std::thread::sleep(std::time::Duration::from_millis(100));
    std::fs::write(&p, "after").unwrap();
    std::thread::sleep(std::time::Duration::from_millis(500));

    // 运行 jobs：轮询 job 触发 → 检测变化 → 解决 promise → .then 回调
    let _ = rt.context.run_jobs();

    // 验证结果
    let r = rt.eval_script("globalThis._result_value && globalThis._result_value.value && globalThis._result_value.value.type").unwrap();
    assert_eq!(r, "modify");
    let done = rt
        .eval_script("globalThis._result_value && globalThis._result_value.done")
        .unwrap();
    assert_eq!(done, "false");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_fs_watch_default_import() {
    let mut rt = common::create_runtime();
    rt.eval_module_str(
        r#"import fs from "@std/fs";
globalThis.r = typeof fs.watch;"#,
        Some(std::path::Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "function");
}
