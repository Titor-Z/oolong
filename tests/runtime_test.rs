use std::path::Path;

#[test]
fn test_eval_script_basic() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let result = rt.eval_script("1 + 2").unwrap();
    assert_eq!(result, "3");
}

#[test]
fn test_eval_script_string() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let result = rt.eval_script("'hello' + ' ' + 'world'").unwrap();
    assert_eq!(result, "hello world");
}

#[test]
fn test_eval_script_error() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let result = rt.eval_script("throw new Error('boom')");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("boom"));
}

#[test]
fn test_eval_module_str_simple() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let result = rt.eval_module_str("export const x = 42;", None).unwrap();
    // Module evaluation returns undefined for non-main modules
    assert_eq!(result, "undefined");
}

#[test]
fn test_eval_module_file_js() {
    let dir = std::env::temp_dir().join("oolong_test_module_js");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let entry = dir.join("main.js");
    std::fs::write(&entry, "export const msg = 'hi'; export default msg;").unwrap();

    let mut rt = oolong::runtime::OolongRuntime::new(&dir).unwrap();
    let result = rt.eval_module_file(&entry).unwrap();
    // Module evaluation of a file with just exports returns undefined
    assert_eq!(result, "undefined");

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_eval_module_file_ts() {
    let dir = std::env::temp_dir().join("oolong_test_module_ts");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let entry = dir.join("main.ts");
    std::fs::write(
        &entry,
        "export const greet = (name: string): string => `hello ${name}`;",
    )
    .unwrap();

    let mut rt = oolong::runtime::OolongRuntime::new(&dir).unwrap();
    let result = rt.eval_module_file(&entry).unwrap();
    assert_eq!(result, "undefined");

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_set_timeout_returns_number() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let result = rt.eval_script("setTimeout(() => {}, 0)").unwrap();
    // setTimeout returns a positive integer ID
    let id: i32 = result.parse().unwrap();
    assert!(id > 0);
}

#[test]
fn test_set_interval_returns_number() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let result = rt.eval_script("setInterval(() => {}, 100)").unwrap();
    let id: i32 = result.parse().unwrap();
    assert!(id > 0);
}

#[test]
fn test_clear_timeout_does_not_crash() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let result = rt
        .eval_script("var id = setTimeout(() => {}, 50); clearTimeout(id); 42")
        .unwrap();
    assert_eq!(result, "42");
}

#[test]
fn test_clear_interval_does_not_crash() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let result = rt
        .eval_script("var id = setInterval(() => {}, 50); clearInterval(id); 42")
        .unwrap();
    assert_eq!(result, "42");
}

#[test]
fn test_import_path_join() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let code = r#"
import { join } from "path";
export const result = join("a", "b", "c");
"#;
    // 使用 eval_module_str 需要确保模块解析正确
    // 用 path_hint 配合 builtin 模块加载
    let result = rt
        .eval_module_str(code, Some(Path::new("__test__.js")))
        .unwrap();
    // 模块 eval 返回 undefined（module 没 default），但导出的值在命名导出中
    // 我们通过 eval_module_str 只能拿到 module 执行后的 promise 值
    // 实际验证通过第二个模块来 import
    assert_eq!(result, "undefined");
}

#[test]
fn test_import_path_join_across_modules() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    // 导入 path 模块并执行函数
    let code = r#"
import * as path from "path";
globalThis.result = path.join("a", "b", "c");
"#;
    rt.eval_module_str(code, Some(Path::new("__test__.js")))
        .unwrap();
    let out = rt.eval_script("globalThis.result").unwrap();
    assert_eq!(out, "a/b/c");
}

#[test]
fn test_import_path_dirname() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let code = r#"
import { dirname } from "path";
globalThis.r = dirname("/a/b/c.txt");
"#;
    rt.eval_module_str(code, Some(Path::new("__test__.js")))
        .unwrap();
    let out = rt.eval_script("globalThis.r").unwrap();
    assert_eq!(out, "/a/b");
}

#[test]
fn test_import_path_basename() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let code = r#"
import { basename } from "path";
globalThis.r = basename("/a/b/c.txt");
"#;
    rt.eval_module_str(code, Some(Path::new("__test__.js")))
        .unwrap();
    let out = rt.eval_script("globalThis.r").unwrap();
    assert_eq!(out, "c.txt");
}

#[test]
fn test_import_path_extname() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let code = r#"
import { extname } from "path";
globalThis.r = extname("/a/b/c.txt");
"#;
    rt.eval_module_str(code, Some(Path::new("__test__.js")))
        .unwrap();
    let out = rt.eval_script("globalThis.r").unwrap();
    assert_eq!(out, ".txt");
}

#[test]
fn test_import_path_is_absolute() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let code = r#"
import { isAbsolute } from "path";
globalThis.r1 = isAbsolute("/a/b");
globalThis.r2 = isAbsolute("a/b");
"#;
    rt.eval_module_str(code, Some(Path::new("__test__.js")))
        .unwrap();
    let r1 = rt.eval_script("globalThis.r1").unwrap();
    let r2 = rt.eval_script("globalThis.r2").unwrap();
    assert_eq!(r1, "true");
    assert_eq!(r2, "false");
}

#[test]
fn test_import_path_normalize() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let code = r#"
import { normalize } from "path";
globalThis.r = normalize("/a/../b/./c//d");
"#;
    rt.eval_module_str(code, Some(Path::new("__test__.js")))
        .unwrap();
    let out = rt.eval_script("globalThis.r").unwrap();
    assert_eq!(out, "/b/c/d");
}

#[test]
fn test_import_path_default_import() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let code = r#"
import path from "path";
globalThis.r = path.join("x", "y");
"#;
    rt.eval_module_str(code, Some(Path::new("__test__.js")))
        .unwrap();
    let out = rt.eval_script("globalThis.r").unwrap();
    assert_eq!(out, "x/y");
}

#[test]
fn test_import_path_relative() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let code = r#"
import { relative } from "path";
globalThis.r = relative("/a/b/c", "/a/d/e");
"#;
    rt.eval_module_str(code, Some(Path::new("__test__.js")))
        .unwrap();
    let out = rt.eval_script("globalThis.r").unwrap();
    assert_eq!(out, "../../d/e");
}

#[test]
fn test_import_path_sep() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { sep } from "path"; globalThis.r = sep;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "/");
}

#[test]
fn test_import_path_delimiter() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { delimiter } from "path"; globalThis.r = delimiter;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), ":");
}

#[test]
fn test_import_path_parse() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"
import { parse } from "path";
const p = parse("/a/b/c.txt");
globalThis.root = p.root;
globalThis.dir = p.dir;
globalThis.base = p.base;
globalThis.ext = p.ext;
globalThis.name = p.name;
"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.root").unwrap(), "/");
    assert_eq!(rt.eval_script("globalThis.dir").unwrap(), "/a/b");
    assert_eq!(rt.eval_script("globalThis.base").unwrap(), "c.txt");
    assert_eq!(rt.eval_script("globalThis.ext").unwrap(), ".txt");
    assert_eq!(rt.eval_script("globalThis.name").unwrap(), "c");
}

#[test]
fn test_import_path_format() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"
import { format } from "path";
globalThis.r = format({ dir: "/a/b", base: "c.txt" });
"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "/a/b/c.txt");
}

#[test]
fn test_import_process_cwd() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { cwd } from "process"; globalThis.r = cwd();"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    assert!(r.starts_with("/"), "cwd should start with /; got {r}");
}

#[test]
fn test_import_process_pid() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { pid } from "process"; globalThis.r = pid;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    let pid: u32 = r.parse().unwrap();
    assert!(pid > 0);
}

#[test]
fn test_import_process_platform() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { platform } from "process"; globalThis.r = platform;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    assert!(
        r == "darwin" || r == "linux" || r == "win32",
        "unexpected platform: {r}"
    );
}

#[test]
fn test_import_process_arch() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { arch } from "process"; globalThis.r = arch;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    assert!(r == "x64" || r == "arm64", "unexpected arch: {r}");
}

#[test]
fn test_import_process_env() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { env } from "process"; globalThis.r = typeof env.PATH;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    assert_eq!(r, "string");
}

#[test]
fn test_import_process_argv() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { argv } from "process"; globalThis.r = argv.length;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    let len: usize = r.parse().unwrap();
    assert!(len >= 1, "argv should have at least 1 entry");
}

#[test]
fn test_import_process_default_import() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import proc from "process"; globalThis.r = typeof proc.cwd;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    assert_eq!(r, "function");
}

// ── process P1~P2 ──────────────────────────────────────────────────────────

#[test]
fn test_import_process_chdir() {
    let dir = std::env::temp_dir().join("oolong_test_chdir");
    std::fs::create_dir_all(&dir).unwrap();
    let original = std::env::current_dir().unwrap();

    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let ds = dir.to_string_lossy().to_string();
    rt.eval_module_str(
        &format!(
            r#"import {{ chdir, cwd }} from "process";
chdir({ds:?});
globalThis.r = cwd();"#
        ),
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    // macOS /tmp → /private/var/tmp 符号链接，只校验结尾即可
    assert!(
        r.ends_with("/oolong_test_chdir"),
        "cwd should end with dir name; got {r}"
    );

    // 恢复
    std::env::set_current_dir(&original).unwrap();
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_import_process_ppid() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { ppid } from "process"; globalThis.r = ppid;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    let ppid: u32 = r.parse().unwrap();
    assert!(ppid > 0);
}

#[test]
fn test_import_process_version() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { version } from "process"; globalThis.r = version;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    assert!(!r.is_empty());
    assert_eq!(r, "0.1.0");
}

#[test]
fn test_import_process_versions() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { versions } from "process"; globalThis.r = versions.oolong;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    assert!(r.contains("0.1.0"));
}

#[test]
fn test_import_process_exec_path() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { execPath } from "process"; globalThis.r = execPath();"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    // execPath should be an absolute path
    assert!(r.starts_with('/'), "execPath should be absolute; got {r}");
}

#[test]
fn test_import_process_uptime() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { uptime } from "process"; globalThis.r = uptime();"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    // uptime should be a positive number
    let secs: f64 = r.parse().unwrap();
    assert!(secs >= 0.0);
}

#[test]
fn test_import_process_stdout_write() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    // stdout.write should not throw
    rt.eval_module_str(
        r#"import { stdout } from "process"; stdout.write("test"); globalThis.r = "ok";"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "ok");
}

#[test]
fn test_import_process_stderr_write() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    // stderr.write should not throw
    rt.eval_module_str(
        r#"import { stderr } from "process"; stderr.write("test"); globalThis.r = "ok";"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "ok");
}

#[test]
fn test_import_process_exec_arg_v() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { execArgv } from "process"; globalThis.r = execArgv.length;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    // execArgv should be the CLI args without the executable
    let r = rt.eval_script("globalThis.r").unwrap();
    let len: usize = r.parse().unwrap();
    // in test context, may be 0 or more
    assert!(len <= std::env::args().len());
}

#[test]
fn test_import_process_default_has_new_apis() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import proc from "process";
globalThis.r1 = typeof proc.chdir;
globalThis.r2 = typeof proc.ppid;
globalThis.r3 = typeof proc.version;
globalThis.r4 = typeof proc.uptime;
globalThis.r5 = typeof proc.execPath;
globalThis.r6 = typeof proc.stdout;
globalThis.r7 = typeof proc.stderr;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r1").unwrap(), "function");
    assert_eq!(rt.eval_script("globalThis.r2").unwrap(), "number");
    assert_eq!(rt.eval_script("globalThis.r3").unwrap(), "string");
    assert_eq!(rt.eval_script("globalThis.r4").unwrap(), "function");
    assert_eq!(rt.eval_script("globalThis.r5").unwrap(), "function");
    assert_eq!(rt.eval_script("globalThis.r6").unwrap(), "object");
    assert_eq!(rt.eval_script("globalThis.r7").unwrap(), "object");
}

#[test]
fn test_console_works() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    // console.log should not throw
    let result = rt.eval_script("console.log('test'); 42").unwrap();
    assert_eq!(result, "42");
}

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
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let p = dir.join("hello.txt").to_string_lossy().to_string();
    rt.eval_module_str(
        &format!(
            r#"import {{ writeTextFile }} from "fs";
await writeTextFile({p:?}, "hello world");
globalThis.r = "ok";"#
        ),
        Some(Path::new("__t.js")),
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
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let ps = p.to_string_lossy().to_string();
    rt.eval_module_str(
        &format!(
            r#"import {{ readTextFile }} from "fs";
const content = await readTextFile({ps:?});
globalThis.r = content;"#
        ),
        Some(Path::new("__t.js")),
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
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let ps = dir.join("x.txt").to_string_lossy().to_string();
    rt.eval_module_str(
        &format!(
            r#"import {{ exists }} from "fs";
const e = await exists({ps:?});
globalThis.r = e;"#
        ),
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_fs_exists_false() {
    let dir = fs_test_dir("oolong_fs_exists_false");
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let ps = dir.join("nonexistent.txt").to_string_lossy().to_string();
    rt.eval_module_str(
        &format!(
            r#"import {{ exists }} from "fs";
const e = await exists({ps:?});
globalThis.r = e;"#
        ),
        Some(Path::new("__t.js")),
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
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let ps = p.to_string_lossy().to_string();
    rt.eval_module_str(
        &format!(
            r#"import {{ readFileSync }} from "fs";
globalThis.r = readFileSync({ps:?}).byteLength;"#
        ),
        Some(Path::new("__t.js")),
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
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let ps = p.to_string_lossy().to_string();
    rt.eval_module_str(
        &format!(
            r#"import {{ readFile }} from "fs";
const buf = await readFile({ps:?});
globalThis.r = buf.byteLength;"#
        ),
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "3");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_fs_write_file() {
    let dir = fs_test_dir("oolong_fs_write_file");
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let ps = dir.join("out.txt").to_string_lossy().to_string();
    rt.eval_module_str(
        &format!(
            r#"import {{ writeFile }} from "fs";
await writeFile({ps:?}, "write file ok");
globalThis.r = "done";"#
        ),
        Some(Path::new("__t.js")),
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
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let ps = p.to_string_lossy().to_string();
    rt.eval_module_str(
        &format!(
            r#"import fs from "fs";
const content = await fs.readTextFile({ps:?});
globalThis.r = content;"#
        ),
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "hi");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_fs_read_not_found() {
    let dir = fs_test_dir("oolong_fs_not_found");
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let ps = dir.join("no_such.txt").to_string_lossy().to_string();
    rt.eval_module_str(
        &format!(
            r#"import {{ readTextFile }} from "fs";
try {{
  await readTextFile({ps:?});
  globalThis.r = "no_error";
}} catch(e) {{
  globalThis.r = "caught";
}}"#
        ),
        Some(Path::new("__t.js")),
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
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        &format!(
            r#"import {{ mkdir, remove, exists }} from "fs";
await mkdir({sub:?});
globalThis.r1 = await exists({sub:?});
await remove({sub:?}, {{ recursive: true }});
globalThis.r2 = await exists({sub:?});"#
        ),
        Some(Path::new("__t.js")),
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
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        &format!(
            r#"import {{ mkdir, exists }} from "fs";
await mkdir({nested:?}, {{ recursive: true }});
globalThis.r = await exists({nested:?});"#
        ),
        Some(Path::new("__t.js")),
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
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let ds = dir.to_string_lossy().to_string();
    rt.eval_module_str(
        &format!(
            r#"import {{ readdir }} from "fs";
const files = await readdir({ds:?});
globalThis.r = files.sort().join(",");"#
        ),
        Some(Path::new("__t.js")),
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
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let ps = p.to_string_lossy().to_string();
    rt.eval_module_str(
        &format!(
            r#"import {{ stat }} from "fs";
const s = await stat({ps:?});
globalThis.r1 = s.isFile;
globalThis.r2 = s.isDirectory;
globalThis.r3 = s.size;"#
        ),
        Some(Path::new("__t.js")),
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
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        &format!(
            r#"import {{ appendFile }} from "fs";
await appendFile({p:?}, "line1\n");
await appendFile({p:?}, "line2\n");
globalThis.r = "ok";"#
        ),
        Some(Path::new("__t.js")),
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
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let ss = src.to_string_lossy().to_string();
    let ds = dir.join("dst.txt").to_string_lossy().to_string();
    rt.eval_module_str(
        &format!(
            r#"import {{ copyFile }} from "fs";
await copyFile({ss:?}, {ds:?});
globalThis.r = "ok";"#
        ),
        Some(Path::new("__t.js")),
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
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let old = dir.join("old.txt").to_string_lossy().to_string();
    let new = dir.join("new.txt").to_string_lossy().to_string();
    rt.eval_module_str(
        &format!(
            r#"import {{ rename, exists }} from "fs";
await rename({old:?}, {new:?});
globalThis.r1 = await exists({old:?});
globalThis.r2 = await exists({new:?});"#
        ),
        Some(Path::new("__t.js")),
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
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let ps = dir.join("link_target.txt").to_string_lossy().to_string();
    rt.eval_module_str(
        &format!(
            r#"import {{ realpath }} from "fs";
const r = await realpath({ps:?});
globalThis.r = r.endsWith("link_target.txt");"#
        ),
        Some(Path::new("__t.js")),
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
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let ts = target.to_string_lossy().to_string();
    let ls = dir.join("link.txt").to_string_lossy().to_string();
    rt.eval_module_str(
        &format!(
            r#"import {{ symlink, stat, lstat }} from "fs";
await symlink({ts:?}, {ls:?});
globalThis.r1 = (await stat({ls:?})).isFile;
globalThis.r2 = (await lstat({ls:?})).isSymlink;"#
        ),
        Some(Path::new("__t.js")),
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
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let ps = dir.join("x.txt").to_string_lossy().to_string();
    let ns = dir.join("none").to_string_lossy().to_string();
    rt.eval_module_str(
        &format!(
            r#"import {{ existsSync }} from "fs";
globalThis.r1 = existsSync({ps:?});
globalThis.r2 = existsSync({ns:?});"#
        ),
        Some(Path::new("__t.js")),
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
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        &format!(
            r#"import {{ mkdirSync, removeSync, existsSync }} from "fs";
mkdirSync({sub:?});
globalThis.r1 = existsSync({sub:?});
removeSync({sub:?}, {{ recursive: true }});
globalThis.r2 = existsSync({sub:?});"#
        ),
        Some(Path::new("__t.js")),
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
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let ds = dir.to_string_lossy().to_string();
    rt.eval_module_str(
        &format!(
            r#"import {{ readdirSync }} from "fs";
const files = readdirSync({ds:?});
globalThis.r = files.sort().join(",");"#
        ),
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "a.txt,b.txt");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_fs_stat_sync() {
    let dir = fs_test_dir("oolong_fs_stat_sync");
    std::fs::write(dir.join("f.txt"), "stat").unwrap();
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let ps = dir.join("f.txt").to_string_lossy().to_string();
    rt.eval_module_str(
        &format!(
            r#"import {{ statSync }} from "fs";
const s = statSync({ps:?});
globalThis.r1 = s.isFile;
globalThis.r2 = s.size;"#
        ),
        Some(Path::new("__t.js")),
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
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let s = dir.join("s.txt").to_string_lossy().to_string();
    let d = dir.join("d.txt").to_string_lossy().to_string();
    let r = dir.join("r.txt").to_string_lossy().to_string();
    rt.eval_module_str(
        &format!(
            r#"import {{ copyFileSync, renameSync, existsSync }} from "fs";
copyFileSync({s:?}, {d:?});
renameSync({d:?}, {r:?});
globalThis.r1 = existsSync({s:?});
globalThis.r2 = existsSync({r:?});"#
        ),
        Some(Path::new("__t.js")),
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
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        &format!(
            r#"import {{ appendFileSync }} from "fs";
appendFileSync({p:?}, "hi\n");
appendFileSync({p:?}, "ho\n");
globalThis.r = "ok";"#
        ),
        Some(Path::new("__t.js")),
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
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let ps = p.to_string_lossy().to_string();
    rt.eval_module_str(
        &format!(
            r#"import {{ chmod }} from "fs";
await chmod({ps:?}, 0o600);
globalThis.r = "ok";"#
        ),
        Some(Path::new("__t.js")),
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
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let os = original.to_string_lossy().to_string();
    let ls = dir.join("hardlink.txt").to_string_lossy().to_string();
    rt.eval_module_str(
        &format!(
            r#"import {{ link, exists }} from "fs";
await link({os:?}, {ls:?});
globalThis.r = await exists({ls:?});"#
        ),
        Some(Path::new("__t.js")),
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
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let ps = p.to_string_lossy().to_string();
    rt.eval_module_str(
        &format!(
            r#"import {{ truncate, readTextFile }} from "fs";
await truncate({ps:?}, 5);
const c = await readTextFile({ps:?});
globalThis.r = c;"#
        ),
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "hello");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_fs_access_ok() {
    let dir = fs_test_dir("oolong_fs_access_ok");
    std::fs::write(dir.join("f.txt"), "data").unwrap();
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let ps = dir.join("f.txt").to_string_lossy().to_string();
    rt.eval_module_str(
        &format!(
            r#"import {{ access }} from "fs";
await access({ps:?});
globalThis.r = "ok";"#
        ),
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "ok");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_fs_access_enoent() {
    let dir = fs_test_dir("oolong_fs_access_enoent");
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let ps = dir.join("nonexistent").to_string_lossy().to_string();
    rt.eval_module_str(
        &format!(
            r#"import {{ access }} from "fs";
try {{
  await access({ps:?});
  globalThis.r = "ok";
}} catch {{
  globalThis.r = "caught";
}}"#
        ),
        Some(Path::new("__t.js")),
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

    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();

    // 开始监视
    rt.eval_module_str(
        &format!(
            r#"import {{ watch }} from "fs";
const w = watch({ps:?});
globalThis._w = w;"#
        ),
        Some(Path::new("__t.js")),
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
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import fs from "fs";
globalThis.r = typeof fs.watch;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "function");
}

// ── node:path 模块 ─────────────────────────────────────────────────────────────

#[test]
fn test_node_path_join() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { join } from "node:path";
globalThis.r = join("a", "b", "c");"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "a/b/c");
}

#[test]
fn test_node_path_dirname() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { dirname } from "node:path";
globalThis.r = dirname("/a/b/c.txt");"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "/a/b");
}

#[test]
fn test_node_path_basename() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { basename } from "node:path";
globalThis.r = basename("/a/b/c.txt");"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "c.txt");
}

#[test]
fn test_node_path_extname() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { extname } from "node:path";
globalThis.r = extname("/a/b/c.txt");"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), ".txt");
}

#[test]
fn test_node_path_resolve() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { resolve } from "node:path";
globalThis.r = resolve("/a", "b", "c");"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "/a/b/c");
}

#[test]
fn test_node_path_relative() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { relative } from "node:path";
globalThis.r = relative("/a/b/c", "/a/d/e");"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "../../d/e");
}

#[test]
fn test_node_path_sep() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { sep } from "node:path"; globalThis.r = sep;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "/");
}

#[test]
fn test_node_path_delimiter() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { delimiter } from "node:path"; globalThis.r = delimiter;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), ":");
}

#[test]
fn test_node_path_posix() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { posix } from "node:path";
globalThis.r1 = posix.sep;
globalThis.r2 = posix.delimiter;
globalThis.r3 = posix.join("a", "b");"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r1").unwrap(), "/");
    assert_eq!(rt.eval_script("globalThis.r2").unwrap(), ":");
    assert_eq!(rt.eval_script("globalThis.r3").unwrap(), "a/b");
}

#[test]
fn test_node_path_win32() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { win32 } from "node:path";
globalThis.r1 = win32.sep;
globalThis.r2 = win32.delimiter;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r1").unwrap(), "\\");
    assert_eq!(rt.eval_script("globalThis.r2").unwrap(), ";");
}

#[test]
fn test_node_path_default_import() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import path from "node:path";
globalThis.r = path.join("x", "y");"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "x/y");
}

#[test]
fn test_node_path_normalize() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { normalize } from "node:path";
globalThis.r = normalize("/a/../b/./c//d");"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "/b/c/d");
}

#[test]
fn test_node_path_parse() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"
import { parse } from "node:path";
const p = parse("/a/b/c.txt");
globalThis.root = p.root;
globalThis.dir = p.dir;
globalThis.base = p.base;
globalThis.ext = p.ext;
globalThis.name = p.name;
"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.root").unwrap(), "/");
    assert_eq!(rt.eval_script("globalThis.dir").unwrap(), "/a/b");
    assert_eq!(rt.eval_script("globalThis.base").unwrap(), "c.txt");
    assert_eq!(rt.eval_script("globalThis.ext").unwrap(), ".txt");
    assert_eq!(rt.eval_script("globalThis.name").unwrap(), "c");
}

#[test]
fn test_node_path_format() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { format } from "node:path";
globalThis.r = format({ dir: "/a/b", base: "c.txt" });"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "/a/b/c.txt");
}

#[test]
fn test_node_path_is_absolute() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { isAbsolute } from "node:path";
globalThis.r1 = isAbsolute("/a/b");
globalThis.r2 = isAbsolute("a/b");"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r1").unwrap(), "true");
    assert_eq!(rt.eval_script("globalThis.r2").unwrap(), "false");
}

#[test]
fn test_node_path_to_namespaced_path() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { toNamespacedPath } from "node:path";
globalThis.r = toNamespacedPath("/foo/bar");"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "/foo/bar");
}

// ── node:os 模块 ───────────────────────────────────────────────────────────────

#[test]
fn test_node_os_arch() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { arch } from "node:os";
globalThis.r = arch();"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    assert!(!r.is_empty(), "arch should not be empty");
}

#[test]
fn test_node_os_platform() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { platform } from "node:os";
globalThis.r = platform();"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    assert!(!r.is_empty());
}

#[test]
fn test_node_os_eol() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { EOL } from "node:os";
globalThis.r = EOL;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    assert!(r == "\n" || r == "\r\n");
}

#[test]
fn test_node_os_endianness() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { endianness } from "node:os";
globalThis.r = endianness();"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    assert!(
        r == "LE" || r == "BE",
        "endianness should be LE or BE; got {r}"
    );
}

#[test]
fn test_node_os_hostname() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { hostname } from "node:os";
globalThis.r = hostname();"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    assert!(!r.is_empty(), "hostname should not be empty");
}

#[test]
fn test_node_os_type() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { type } from "node:os";
globalThis.r = type();"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    assert!(!r.is_empty());
}

#[test]
fn test_node_os_release() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { release } from "node:os";
globalThis.r = release();"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    assert!(!r.is_empty());
}

#[test]
fn test_node_os_homedir() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { homedir } from "node:os";
globalThis.r = homedir();"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    assert!(r.starts_with('/'), "homedir should be absolute; got {r}");
}

#[test]
fn test_node_os_tmpdir() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { tmpdir } from "node:os";
globalThis.r = tmpdir();"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    assert!(!r.is_empty());
}

#[test]
fn test_node_os_totalmem() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { totalmem } from "node:os";
globalThis.r = totalmem();"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    let mem: f64 = r.parse().unwrap_or(0.0);
    assert!(mem > 0.0, "totalmem should be positive; got {r}");
}

#[test]
fn test_node_os_freemem() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { freemem } from "node:os";
globalThis.r = freemem();"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    let mem: f64 = r.parse().unwrap_or(-1.0);
    assert!(mem >= 0.0, "freemem should be >= 0; got {r}");
}

#[test]
fn test_node_os_uptime() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { uptime } from "node:os";
globalThis.r = uptime();"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    let secs: f64 = r.parse().unwrap_or(-1.0);
    assert!(secs >= 0.0, "uptime should be >= 0; got {r}");
}

#[test]
fn test_node_os_loadavg() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { loadavg } from "node:os";
globalThis.r = loadavg();"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    // 返回格式 "1,2,3" 或 undefined
    assert!(!r.is_empty(), "loadavg should return an array; got empty");
}

#[test]
fn test_node_os_cpus() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { cpus } from "node:os";
const arr = cpus();
globalThis.r1 = arr.length;
globalThis.r2 = arr.length > 0 ? typeof arr[0].model : "no_cpus";"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let len = rt.eval_script("globalThis.r1").unwrap();
    let count: usize = len.parse().unwrap_or(0);
    assert!(count > 0, "should have at least 1 cpu; got {len}");
    let model_type = rt.eval_script("globalThis.r2").unwrap();
    assert_eq!(model_type, "string", "cpu model should be a string");
}

#[test]
fn test_node_os_user_info() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { userInfo } from "node:os";
const info = userInfo();
globalThis.r1 = typeof info.username;
globalThis.r2 = typeof info.shell;
globalThis.r3 = typeof info.homedir;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r1").unwrap(), "string");
    assert_eq!(rt.eval_script("globalThis.r2").unwrap(), "string");
    assert_eq!(rt.eval_script("globalThis.r3").unwrap(), "string");
}

#[test]
fn test_node_os_version() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { version } from "node:os";
globalThis.r = version();"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    assert!(!r.is_empty(), "version should not be empty");
}

#[test]
fn test_node_os_machine() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { machine } from "node:os";
globalThis.r = machine();"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    assert!(!r.is_empty(), "machine should not be empty");
}

#[test]
fn test_node_os_default_import() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import os from "node:os";
globalThis.r = typeof os.platform;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "function");
}

#[test]
fn test_fs_default_has_all_apis() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import fs from "fs";
globalThis.r1 = typeof fs.mkdir;
globalThis.r2 = typeof fs.readdir;
globalThis.r3 = typeof fs.stat;
globalThis.r4 = typeof fs.mkdirSync;
globalThis.r5 = typeof fs.readdirSync;
globalThis.r6 = typeof fs.chmod;
globalThis.r7 = typeof fs.link;
globalThis.r8 = typeof fs.truncate;
globalThis.r9 = typeof fs.lstat;
globalThis.ra = typeof fs.symlinkSync;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r1").unwrap(), "function");
    assert_eq!(rt.eval_script("globalThis.r2").unwrap(), "function");
    assert_eq!(rt.eval_script("globalThis.r3").unwrap(), "function");
    assert_eq!(rt.eval_script("globalThis.r4").unwrap(), "function");
    assert_eq!(rt.eval_script("globalThis.r5").unwrap(), "function");
    assert_eq!(rt.eval_script("globalThis.r6").unwrap(), "function");
    assert_eq!(rt.eval_script("globalThis.r7").unwrap(), "function");
    assert_eq!(rt.eval_script("globalThis.r8").unwrap(), "function");
    assert_eq!(rt.eval_script("globalThis.r9").unwrap(), "function");
    assert_eq!(rt.eval_script("globalThis.ra").unwrap(), "function");
}

// ── os 模块 ─────────────────────────────────────────────────────────────────

#[test]
fn test_import_os_platform() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { platform } from "os";
globalThis.r = platform();"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    assert!(!r.is_empty());
}

#[test]
fn test_import_os_arch() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { arch } from "os";
globalThis.r = arch();"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    assert!(!r.is_empty());
}

#[test]
fn test_import_os_eol() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { EOL } from "os";
globalThis.r = EOL;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    assert!(r == "\n" || r == "\r\n");
}

#[test]
fn test_import_os_hostname() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { hostname } from "os";
globalThis.r = hostname();"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    assert!(!r.is_empty(), "hostname should not be empty");
}

#[test]
fn test_import_os_type() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { type } from "os";
globalThis.r = type();"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    assert!(!r.is_empty());
}

#[test]
fn test_import_os_release() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { release } from "os";
globalThis.r = release();"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    assert!(!r.is_empty());
}

#[test]
fn test_import_os_homedir() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { homedir } from "os";
globalThis.r = homedir();"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    assert!(r.starts_with('/'), "homedir should be absolute; got {r}");
}

#[test]
fn test_import_os_tmpdir() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { tmpdir } from "os";
globalThis.r = tmpdir();"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    assert!(!r.is_empty());
}

#[test]
fn test_import_os_totalmem() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { totalmem } from "os";
globalThis.r = totalmem();"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    let mem: f64 = r.parse().unwrap_or(0.0);
    assert!(mem > 0.0, "totalmem should be positive; got {r}");
}

#[test]
fn test_import_os_freemem() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { freemem } from "os";
globalThis.r = freemem();"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    let mem: f64 = r.parse().unwrap_or(-1.0);
    assert!(mem >= 0.0, "freemem should be >= 0; got {r}");
}

#[test]
fn test_import_os_default_import() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import os from "os";
globalThis.r = typeof os.platform;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "function");
}

// ── Blob ──────────────────────────────────────────────────────────────────────

#[test]
fn test_blob_constructor_string() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_script(
        r#"
let b = new Blob(["hello"]);
globalThis.r = b.size;
        "#,
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "5");
}

#[test]
fn test_blob_type() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_script(
        r#"
let b = new Blob(["test"], { type: "text/plain" });
globalThis.r = b.type;
        "#,
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "text/plain");
}

#[test]
fn test_blob_text() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"
let b = new Blob(["hello world"]);
b.text().then(v => { globalThis.r = v; });
        "#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    rt.eval_script("globalThis.r").ok();
    let r = rt.eval_script("globalThis.r").unwrap_or_default();
    assert_eq!(
        r, "hello world",
        "Blob.text() should resolve to 'hello world'; got {r}"
    );
}

#[test]
fn test_blob_slice() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_script(
        r#"
let b = new Blob(["abcdefgh"]);
let s = b.slice(2, 5);
globalThis.r = s.size;
        "#,
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "3");
}

#[test]
fn test_blob_multiple_parts() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_script(
        r#"
let b = new Blob(["abc", "def", "ghi"]);
globalThis.r = b.size;
        "#,
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "9");
}

// ── File ──────────────────────────────────────────────────────────────────────

#[test]
fn test_file_constructor() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_script(
        r#"
let f = new File(["data"], "test.txt");
globalThis.r = f.name + "|" + f.size;
        "#,
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "test.txt|4");
}

#[test]
fn test_file_last_modified() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_script(
        r#"
let f = new File(["x"], "x.txt", { lastModified: 1234567890000 });
globalThis.r = f.lastModified;
        "#,
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "1234567890000");
}

// ── URL ──────────────────────────────────────────────────────────────────────

#[test]
fn test_url_basic() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_script(
        r#"
let u = new URL("https://example.com/path?q=1#hash");
globalThis.r = u.href + "|" + u.hostname + "|" + u.pathname + "|" + u.search + "|" + u.hash;
        "#,
    )
    .unwrap();
    let r = rt.eval_script("globalThis.r").unwrap();
    assert!(
        r.contains("example.com"),
        "URL href should contain hostname; got {r}"
    );
    assert!(r.contains("path"), "URL should contain path; got {r}");
}

#[test]
fn test_url_relative() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_script(
        r#"
let u = new URL("/foo", "https://base.com/bar/");
globalThis.r = u.href;
        "#,
    )
    .unwrap();
    assert_eq!(
        rt.eval_script("globalThis.r").unwrap(),
        "https://base.com/foo"
    );
}

// ── URLSearchParams ───────────────────────────────────────────────────────────

#[test]
fn test_url_search_params_get() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_script(
        r#"
let p = new URLSearchParams("a=1&b=2&a=3");
globalThis.r = p.get("a") + "|" + p.get("b");
        "#,
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "1|2");
}

#[test]
fn test_url_search_params_get_all() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_script(
        r#"
let p = new URLSearchParams("a=1&a=2&a=3");
let all = p.getAll("a");
globalThis.r = all.length + "|" + all[0] + "|" + all[1] + "|" + all[2];
        "#,
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "3|1|2|3");
}

#[test]
fn test_url_search_params_has_delete_set_append() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_script(
        r#"
let p = new URLSearchParams("a=1&b=2");
p.set("a", "10");
p.append("c", "3");
p.delete("b");
globalThis.r = p.toString();
        "#,
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "a=10&c=3");
}

#[test]
fn test_url_search_params_sort() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_script(
        r#"
let p = new URLSearchParams("z=1&a=2&m=3");
p.sort();
globalThis.r = p.toString();
        "#,
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "a=2&m=3&z=1");
}

#[test]
fn test_url_search_params_no_question_mark() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_script(
        r#"
let p = new URLSearchParams("?a=1&b=2");
globalThis.r = p.get("a") + "|" + p.get("b");
        "#,
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "1|2");
}

// ── node:events ────────────────────────────────────────────────────────────────

#[test]
fn test_node_events_default_import() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import EventEmitter from "node:events";
const ee = new EventEmitter();
globalThis.r = typeof ee.on;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "function");
}

#[test]
fn test_node_events_named_import() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { EventEmitter } from "node:events";
const ee = new EventEmitter();
globalThis.r = typeof ee.emit;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "function");
}

#[test]
fn test_node_events_on_emit() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import EventEmitter from "node:events";
const ee = new EventEmitter();
ee.on("foo", (x) => { globalThis.r = x; });
ee.emit("foo", 42);"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "42");
}

#[test]
fn test_node_events_once() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import EventEmitter from "node:events";
const ee = new EventEmitter();
let count = 0;
ee.once("foo", () => { count++; });
ee.emit("foo");
ee.emit("foo");
globalThis.r = count;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "1");
}

#[test]
fn test_node_events_off() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import EventEmitter from "node:events";
const ee = new EventEmitter();
function handler() { globalThis.r = "called"; }
ee.on("foo", handler);
ee.off("foo", handler);
ee.emit("foo");
globalThis.r = globalThis.r || "not_called";"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "not_called");
}

#[test]
fn test_node_events_remove_all_listeners() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import EventEmitter from "node:events";
const ee = new EventEmitter();
ee.on("a", () => {});
ee.on("b", () => {});
ee.removeAllListeners();
globalThis.r = ee.eventNames().length;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "0");
}

#[test]
fn test_node_events_listener_count() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import EventEmitter from "node:events";
const ee = new EventEmitter();
const fn = () => {};
ee.on("foo", fn);
ee.on("foo", fn);
globalThis.r = ee.listenerCount("foo");"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "2");
}

#[test]
fn test_node_events_event_names() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import EventEmitter from "node:events";
const ee = new EventEmitter();
ee.on("a", () => {});
ee.on("b", () => {});
const names = ee.eventNames().sort().join(",");
globalThis.r = names;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "a,b");
}

#[test]
fn test_node_events_max_listeners() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import EventEmitter from "node:events";
const ee = new EventEmitter();
ee.setMaxListeners(5);
globalThis.r1 = ee.getMaxListeners();
EventEmitter.defaultMaxListeners = 20;
const ee2 = new EventEmitter();
globalThis.r2 = ee2.getMaxListeners();"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r1").unwrap(), "5");
    assert_eq!(rt.eval_script("globalThis.r2").unwrap(), "20");
}

#[test]
fn test_node_events_prepend_listener() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import EventEmitter from "node:events";
const ee = new EventEmitter();
const order = [];
ee.on("foo", () => order.push(1));
ee.prependListener("foo", () => order.push(2));
ee.emit("foo");
globalThis.r = order.join(",");"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "2,1");
}

#[test]
fn test_node_events_new_listener_event() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import EventEmitter from "node:events";
const ee = new EventEmitter();
const events = [];
ee.on("newListener", (ev, fn) => { events.push(ev); });
ee.on("foo", () => {});
ee.on("bar", () => {});
globalThis.r = events.join(",");"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "foo,bar");
}

#[test]
fn test_node_events_emit_return_value() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import EventEmitter from "node:events";
const ee = new EventEmitter();
ee.on("foo", () => {});
globalThis.r1 = ee.emit("foo");
globalThis.r2 = ee.emit("nonexistent");"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r1").unwrap(), "true");
    assert_eq!(rt.eval_script("globalThis.r2").unwrap(), "false");
}

#[test]
fn test_node_events_static_listener_count() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { EventEmitter } from "node:events";
const ee = new EventEmitter();
ee.on("foo", () => {});
globalThis.r = EventEmitter.listenerCount(ee, "foo");"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "1");
}

// ── node:util ────────────────────────────────────────────────────

#[test]
fn test_node_util_default_import() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import util from "node:util";
globalThis.r = typeof util.format;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "function");
}

#[test]
fn test_node_util_promisify() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { promisify } from "node:util";
function add(a, b, cb) { cb(null, a + b); }
const addAsync = promisify(add);
addAsync(3, 4).then(v => { globalThis.r = v; });"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "7");
}

#[test]
fn test_node_util_format() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { format } from "node:util";
globalThis.r = format("%s:%d", "hello", 42);"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "hello:42");
}

#[test]
fn test_node_util_inspect() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { inspect } from "node:util";
globalThis.r = inspect({a: 1, b: "hello"});"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let result = rt.eval_script("globalThis.r").unwrap();
    assert!(result.contains("a"));
    assert!(result.contains("hello"));
}

#[test]
fn test_node_util_types() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { types } from "node:util";
globalThis.r =
  types.isDate(new Date()) &&
  !types.isDate(42) &&
  types.isRegExp(/abc/) &&
  types.isArrayBuffer(new ArrayBuffer(8)) &&
  types.isMap(new Map()) &&
  types.isSet(new Set()) &&
  types.isNativeError(new Error()) &&
  types.isTypedArray(new Uint8Array());"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

// ── node:stream ──────────────────────────────────────────────────

#[test]
fn test_node_stream_default_import() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import stream from "node:stream";
globalThis.r = typeof stream.Readable;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "function");
}

#[test]
fn test_node_stream_named_imports() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { Readable, Writable, Transform, Duplex, PassThrough, pipeline, finished } from "node:stream";
globalThis.r =
  typeof Readable === "function" &&
  typeof Writable === "function" &&
  typeof Transform === "function" &&
  typeof Duplex === "function" &&
  typeof PassThrough === "function" &&
  typeof pipeline === "function" &&
  typeof finished === "function";"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

#[test]
fn test_node_stream_readable() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { Readable } from "node:stream";
const r = new Readable({ read() { this.push("hello"); this.push(null); } });
let data = "";
r.on("data", chunk => { data += chunk.toString(); });
r.on("end", () => { globalThis.r = data; });"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "hello");
}

#[test]
fn test_node_stream_pipeline() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { Readable, Writable, pipeline } from "node:stream";
const r = new Readable({ read() { this.push("hi"); this.push(null); } });
const w = new Writable({ write(chunk, enc, cb) { globalThis.r = chunk.toString(); cb(); } });
pipeline(r, w, () => {});"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "hi");
}

#[test]
fn test_node_stream_passthrough() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { PassThrough } from "node:stream";
const pt = new PassThrough();
pt.write("abc");
pt.end();
const data = pt.read();
globalThis.r = data ? data.toString() : "null";"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let result = rt.eval_script("globalThis.r").unwrap();
    assert_eq!(result, "abc", "got: {result}");
}

// ── node:url ─────────────────────────────────────────────────────

#[test]
fn test_node_url_default_import() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import url from "node:url";
globalThis.r = typeof url.URL;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "function");
}

#[test]
fn test_node_url_named_exports() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { URL, URLSearchParams, fileURLToPath, pathToFileURL } from "node:url";
globalThis.r =
  typeof URL === "function" &&
  typeof fileURLToPath === "function" &&
  typeof pathToFileURL === "function";"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

#[test]
fn test_node_url_file_url_to_path() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { fileURLToPath } from "node:url";
globalThis.r = fileURLToPath("file:///usr/local/bin");"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "/usr/local/bin");
}

#[test]
fn test_node_url_path_to_file_url() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { pathToFileURL } from "node:url";
const u = pathToFileURL("/usr/local/bin");
globalThis.r = u.href;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(
        rt.eval_script("globalThis.r").unwrap(),
        "file:///usr/local/bin"
    );
}

#[test]
fn test_node_url_url_class() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { URL } from "node:url";
const u = new URL("https://example.com:8080/path?q=1#hash");
globalThis.r = u.hostname + ":" + u.port;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "example.com:8080");
}

// ── Phase 5.5: node:crypto ────────────────────────────────────────────────

#[test]
fn test_node_crypto_create_hash_sha256() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
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
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
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
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
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
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
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
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
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
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
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

// ── Phase 5.5: node:child_process ──────────────────────────────────────────

#[test]
fn test_node_child_process_exec_sync() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
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
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
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
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
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

// ── Phase 5.5: node:module ─────────────────────────────────────────────────

#[test]
fn test_node_module_is_builtin() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { isBuiltin } from "node:module";
globalThis.r1 = isBuiltin("fs");
globalThis.r2 = isBuiltin("node:fs");
globalThis.r3 = isBuiltin("not-a-real-module");"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r1").unwrap(), "true");
    assert_eq!(rt.eval_script("globalThis.r2").unwrap(), "true");
    assert_eq!(rt.eval_script("globalThis.r3").unwrap(), "false");
}

#[test]
fn test_node_module_builtin_modules() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { builtinModules } from "node:module";
globalThis.r = Array.isArray(builtinModules) && builtinModules.length > 0;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

#[test]
fn test_node_module_create_require() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { createRequire } from "node:module";
const req = createRequire("/test/path/file.js");
globalThis.r = typeof req === "function";"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

#[test]
fn test_node_module_default_import() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import mod from "node:module";
globalThis.r = typeof mod.isBuiltin === "function" && Array.isArray(mod.builtinModules);"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

#[test]
fn test_node_module_resolve_filename() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { Module } from "node:module";
globalThis.r = typeof Module._resolveFilename === "function";"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}
