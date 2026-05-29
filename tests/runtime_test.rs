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
import { join } from "@std/path";
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
import * as path from "@std/path";
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
import { dirname } from "@std/path";
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
import { basename } from "@std/path";
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
import { extname } from "@std/path";
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
import { isAbsolute } from "@std/path";
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
import { normalize } from "@std/path";
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
import path from "@std/path";
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
import { relative } from "@std/path";
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
        r#"import { sep } from "@std/path"; globalThis.r = sep;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "/");
}

#[test]
fn test_import_path_delimiter() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { delimiter } from "@std/path"; globalThis.r = delimiter;"#,
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
import { parse } from "@std/path";
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
import { format } from "@std/path";
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
        r#"import { cwd } from "@std/process"; globalThis.r = cwd();"#,
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
        r#"import { pid } from "@std/process"; globalThis.r = pid;"#,
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
        r#"import { platform } from "@std/process"; globalThis.r = platform;"#,
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
        r#"import { arch } from "@std/process"; globalThis.r = arch;"#,
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
        r#"import { env } from "@std/process"; globalThis.r = typeof env.PATH;"#,
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
        r#"import { argv } from "@std/process"; globalThis.r = argv.length;"#,
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
        r#"import proc from "@std/process"; globalThis.r = typeof proc.cwd;"#,
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
            r#"import {{ chdir, cwd }} from "@std/process";
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
        r#"import { ppid } from "@std/process"; globalThis.r = ppid;"#,
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
        r#"import { version } from "@std/process"; globalThis.r = version;"#,
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
        r#"import { versions } from "@std/process"; globalThis.r = versions.oolong;"#,
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
        r#"import { execPath } from "@std/process"; globalThis.r = execPath();"#,
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
        r#"import { uptime } from "@std/process"; globalThis.r = uptime();"#,
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
        r#"import { stdout } from "@std/process"; stdout.write("test"); globalThis.r = "ok";"#,
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
        r#"import { stderr } from "@std/process"; stderr.write("test"); globalThis.r = "ok";"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "ok");
}

#[test]
fn test_import_process_exec_arg_v() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { execArgv } from "@std/process"; globalThis.r = execArgv.length;"#,
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
        r#"import proc from "@std/process";
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
            r#"import {{ writeTextFile }} from "@std/fs";
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
            r#"import {{ readTextFile }} from "@std/fs";
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
            r#"import {{ exists }} from "@std/fs";
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
            r#"import {{ exists }} from "@std/fs";
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
            r#"import {{ readFileSync }} from "@std/fs";
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
            r#"import {{ readFile }} from "@std/fs";
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
            r#"import {{ writeFile }} from "@std/fs";
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
            r#"import fs from "@std/fs";
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
            r#"import {{ readTextFile }} from "@std/fs";
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
            r#"import {{ mkdir, remove, exists }} from "@std/fs";
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
            r#"import {{ mkdir, exists }} from "@std/fs";
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
            r#"import {{ readdir }} from "@std/fs";
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
            r#"import {{ stat }} from "@std/fs";
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
            r#"import {{ appendFile }} from "@std/fs";
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
            r#"import {{ copyFile }} from "@std/fs";
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
            r#"import {{ rename, exists }} from "@std/fs";
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
            r#"import {{ realpath }} from "@std/fs";
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
            r#"import {{ symlink, stat, lstat }} from "@std/fs";
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
            r#"import {{ existsSync }} from "@std/fs";
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
            r#"import {{ mkdirSync, removeSync, existsSync }} from "@std/fs";
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
            r#"import {{ readdirSync }} from "@std/fs";
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
            r#"import {{ statSync }} from "@std/fs";
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
            r#"import {{ copyFileSync, renameSync, existsSync }} from "@std/fs";
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
            r#"import {{ appendFileSync }} from "@std/fs";
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
            r#"import {{ chmod }} from "@std/fs";
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
            r#"import {{ link, exists }} from "@std/fs";
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
            r#"import {{ truncate, readTextFile }} from "@std/fs";
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
            r#"import {{ access }} from "@std/fs";
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
            r#"import {{ access }} from "@std/fs";
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
            r#"import {{ watch }} from "@std/fs";
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
        r#"import fs from "@std/fs";
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
        r#"import fs from "@std/fs";
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
        r#"import { platform } from "@std/os";
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
        r#"import { arch } from "@std/os";
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
        r#"import { EOL } from "@std/os";
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
        r#"import { hostname } from "@std/os";
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
        r#"import { type } from "@std/os";
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
        r#"import { release } from "@std/os";
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
        r#"import { homedir } from "@std/os";
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
        r#"import { tmpdir } from "@std/os";
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
        r#"import { totalmem } from "@std/os";
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
        r#"import { freemem } from "@std/os";
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
        r#"import os from "@std/os";
globalThis.r = typeof os.platform;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "function");
}

#[test]
fn test_import_os_cpus() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import os from "@std/os";
const cpus = os.cpus();
globalThis._r = JSON.stringify({ count: cpus.length, hasModel: typeof cpus[0]?.model === 'string', hasSpeed: typeof cpus[0]?.speed === 'number' });"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let result = rt.eval_script("globalThis._r").unwrap();
    let v: serde_json::Value = serde_json::from_str(&result).unwrap();
    let count = v["count"].as_i64().unwrap();
    assert!(count >= 1, "expected at least 1 CPU, got {count}");
    assert_eq!(v["hasModel"], true);
    assert_eq!(v["hasSpeed"], true);
}

#[test]
fn test_import_os_uptime() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import os from "@std/os";
globalThis._r = os.uptime();"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let result = rt.eval_script("globalThis._r").unwrap();
    let uptime: f64 = result.parse().unwrap();
    assert!(uptime > 0.0, "uptime should be positive, got {uptime}");
}

#[test]
fn test_import_os_loadavg() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import os from "@std/os";
globalThis._r = JSON.stringify(os.loadavg());"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let result = rt.eval_script("globalThis._r").unwrap();
    let v: Vec<f64> = serde_json::from_str(&result).unwrap();
    assert_eq!(v.len(), 3);
    for &val in &v {
        assert!(val >= 0.0);
    }
}

#[test]
fn test_import_os_endianness() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import os from "@std/os";
globalThis._r = os.endianness();"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let result = rt.eval_script("globalThis._r").unwrap();
    assert!(result == "LE" || result == "BE", "expected LE or BE, got {result}");
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

// ── node:querystring ─────────────────────────────────────────────────────────

#[test]
fn test_node_querystring_parse() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { parse } from "node:querystring";
globalThis.r = JSON.stringify(parse("foo=bar&baz=qux"));"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), r#"{"foo":"bar","baz":"qux"}"#);
}

#[test]
fn test_node_querystring_parse_with_eq_and_sep() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { parse } from "node:querystring";
globalThis.r = JSON.stringify(parse("foo=bar;baz=qux", ";", "="));"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), r#"{"foo":"bar","baz":"qux"}"#);
}

#[test]
fn test_node_querystring_parse_array_duplicate() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { parse } from "node:querystring";
globalThis.r = JSON.stringify(parse("a=1&a=2&a=3"));"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), r#"{"a":["1","2","3"]}"#);
}

#[test]
fn test_node_querystring_parse_empty() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { parse } from "node:querystring";
globalThis.r = JSON.stringify(parse(""));"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "{}");
}

#[test]
fn test_node_querystring_parse_no_value() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { parse } from "node:querystring";
globalThis.r = JSON.stringify(parse("foo&bar=baz"));"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), r#"{"foo":"","bar":"baz"}"#);
}

#[test]
fn test_node_querystring_stringify() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { stringify } from "node:querystring";
globalThis.r = stringify({ foo: "bar", baz: "qux" });"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "foo=bar&baz=qux");
}

#[test]
fn test_node_querystring_stringify_array() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { stringify } from "node:querystring";
globalThis.r = stringify({ a: [1, 2, 3] });"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "a=1&a=2&a=3");
}

#[test]
fn test_node_querystring_escape_unescape() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { escape, unescape } from "node:querystring";
globalThis.r = unescape(escape("hello world"));"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "hello world");
}

#[test]
fn test_node_querystring_default_import() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import qs from "node:querystring";
globalThis.r = typeof qs.parse === "function" && typeof qs.stringify === "function";"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

#[test]
fn test_node_querystring_decode_encode_aliases() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { decode, encode } from "node:querystring";
globalThis.r = decode("a=1").a === "1" && encode({b:2}) === "b=2";"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

// ── node:assert ──────────────────────────────────────────────────────────────

#[test]
fn test_node_assert_ok() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import assert from "node:assert";
globalThis.r = "ok";
assert.ok(true);
globalThis.r = "passed";"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "passed");
}

#[test]
fn test_node_assert_ok_throws() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let result = rt.eval_module_str(
        r#"import assert from "node:assert";
assert.ok(false);"#,
        Some(Path::new("__t.js")),
    );
    assert!(result.is_err());
}

#[test]
fn test_node_assert_equal() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import assert from "node:assert";
assert.equal(3, 3);
assert.equal("hello", "hello");"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
}

#[test]
fn test_node_assert_equal_throws() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let result = rt.eval_module_str(
        r#"import assert from "node:assert";
assert.equal(1, 2);"#,
        Some(Path::new("__t.js")),
    );
    assert!(result.is_err());
}

#[test]
fn test_node_assert_strict_equal() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import assert from "node:assert";
assert.strictEqual(1, 1);"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
}

#[test]
fn test_node_assert_strict_equal_throws() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let result = rt.eval_module_str(
        r#"import assert from "node:assert";
assert.strictEqual(1, "1");"#,
        Some(Path::new("__t.js")),
    );
    assert!(result.is_err());
}

#[test]
fn test_node_assert_not_strict_equal() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import assert from "node:assert";
assert.notStrictEqual(1, "1");"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
}

#[test]
fn test_node_assert_deep_equal() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import assert from "node:assert";
assert.deepEqual({ a: 1 }, { a: 1 });"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
}

#[test]
fn test_node_assert_throws_basic() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import assert from "node:assert";
assert.throws(() => { throw new Error("boom"); });"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
}

#[test]
fn test_node_assert_throws_missing() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let result = rt.eval_module_str(
        r#"import assert from "node:assert";
assert.throws(() => {});"#,
        Some(Path::new("__t.js")),
    );
    assert!(result.is_err());
}

#[test]
fn test_node_assert_throws_instanceof() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import assert from "node:assert";
assert.throws(() => { throw new TypeError("bad"); }, TypeError);"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
}

#[test]
fn test_node_assert_if_error() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import assert from "node:assert";
assert.ifError(null);
assert.ifError(undefined);"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
}

#[test]
fn test_node_assert_if_error_throws() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let result = rt.eval_module_str(
        r#"import assert from "node:assert";
assert.ifError(new Error("err"));"#,
        Some(Path::new("__t.js")),
    );
    assert!(result.is_err());
}

#[test]
fn test_node_assert_fail() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let result = rt.eval_module_str(
        r#"import assert from "node:assert";
assert.fail("intentional");"#,
        Some(Path::new("__t.js")),
    );
    assert!(result.is_err());
}

#[test]
fn test_node_assert_strict_namespace() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { strict } from "node:assert";
strict.equal(1, 1);
var threw = false;
try { strict.equal(1, "1"); } catch (e) { threw = true; }
globalThis.r = threw;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

#[test]
fn test_node_assert_assertion_error() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { AssertionError } from "node:assert";
var e = new AssertionError({ message: "test", actual: 1, expected: 2 });
globalThis.r = e.name === "AssertionError" && e.message === "test" && e.actual === 1 && e.expected === 2;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

#[test]
fn test_node_assert_default_import() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import assert from "node:assert";
globalThis.r = typeof assert.ok === "function" && typeof assert.strictEqual === "function" && typeof assert.throws === "function";"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

// ── node:timers ──────────────────────────────────────────────────────────────

#[test]
fn test_node_timers_set_timeout() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { setTimeout } from "node:timers";
globalThis.r = typeof setTimeout === "function";"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

#[test]
fn test_node_timers_set_interval() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { setInterval } from "node:timers";
globalThis.r = typeof setInterval === "function";"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

#[test]
fn test_node_timers_set_immediate() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { setImmediate } from "node:timers";
globalThis.r = typeof setImmediate === "function";"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

#[test]
fn test_node_timers_default_import() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import timers from "node:timers";
globalThis.r = typeof timers.setTimeout === "function" && typeof timers.setInterval === "function";"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

#[test]
fn test_node_timers_promises_set_timeout() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import timers from "node:timers";
var p = timers.promises.setTimeout(0, "ok");
globalThis.r = (typeof p.then === "function").toString();"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

#[test]
fn test_node_timers_promises_set_immediate() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import timers from "node:timers";
var p = timers.promises.setImmediate("done");
p.then(function(v) { globalThis.r = v; });
globalThis.r = "pending";"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let _ = rt.context.run_jobs();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "done");
}

#[test]
fn test_node_timers_clear_timeout() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { clearTimeout } from "node:timers";
globalThis.r = typeof clearTimeout === "function";"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

// ── node:tty ─────────────────────────────────────────────────────────────────

#[test]
fn test_node_tty_isatty_function() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
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
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
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
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
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
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
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
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import tty from "node:tty";
globalThis.r = typeof tty.isatty === "function" && typeof tty.WriteStream === "function";"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

// ── node:perf_hooks ──────────────────────────────────────────────────────────

#[test]
fn test_node_perf_hooks_performance_now() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { performance } from "node:perf_hooks";
var n = performance.now();
globalThis.r = typeof n === "number" && n >= 0;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

#[test]
fn test_node_perf_hooks_performance_now_increasing() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { performance } from "node:perf_hooks";
var a = performance.now();
var b = performance.now();
globalThis.r = b >= a;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

#[test]
fn test_node_perf_hooks_performance_time_origin() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { performance } from "node:perf_hooks";
globalThis.r = typeof performance.timeOrigin === "number" && performance.timeOrigin > 0;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

#[test]
fn test_node_perf_hooks_performance_entry() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { PerformanceEntry } from "node:perf_hooks";
var e = new PerformanceEntry("test", "mark", 0, 10);
globalThis.r = e.name === "test" && e.entryType === "mark" && e.duration === 10;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

#[test]
fn test_node_perf_hooks_performance_mark() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { performance, PerformanceMark } from "node:perf_hooks";
var m = performance.mark("test");
globalThis.r = m instanceof PerformanceMark && m.name === "test" && m.entryType === "mark";"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

#[test]
fn test_node_perf_hooks_default_import() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import perf_hooks from "node:perf_hooks";
globalThis.r = typeof perf_hooks.performance === "object" && typeof perf_hooks.PerformanceEntry === "function";"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

// ── node:vm ──────────────────────────────────────────────────────────────────

#[test]
fn test_node_vm_run_in_this_context() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { runInThisContext } from "node:vm";
globalThis.r = runInThisContext("1 + 2");"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "3");
}

#[test]
fn test_node_vm_run_in_new_context() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { runInNewContext } from "node:vm";
globalThis.r = runInNewContext("x + y", { x: 1, y: 2 });"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "3");
}

#[test]
fn test_node_vm_script() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { Script } from "node:vm";
var s = new Script("a + b");
globalThis.r = typeof s.runInThisContext === "function" && typeof s.runInNewContext === "function";"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

#[test]
fn test_node_vm_script_run_in_new_context() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { Script } from "node:vm";
var s = new Script("x * y");
globalThis.r = s.runInNewContext({ x: 3, y: 4 });"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "12");
}

#[test]
fn test_node_vm_compile_function() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { compileFunction } from "node:vm";
var fn = compileFunction("return a + b", ["a", "b"]);
globalThis.r = fn(3, 4);"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "7");
}

#[test]
fn test_node_vm_default_import() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import vm from "node:vm";
globalThis.r = typeof vm.runInThisContext === "function" && typeof vm.Script === "function";"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

// ── node:zlib ────────────────────────────────────────────────────────────────

#[test]
fn test_node_zlib_gzip_roundtrip() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { gzipSync, gunzipSync } from "node:zlib";
var original = new TextEncoder().encode("hello zlib");
var compressed = gzipSync(original);
var decompressed = gunzipSync(compressed);
var dec = new TextDecoder();
globalThis.r = dec.decode(decompressed);"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "hello zlib");
}

#[test]
fn test_node_zlib_deflate_roundtrip() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { deflateSync, inflateSync } from "node:zlib";
var original = new TextEncoder().encode("hello deflate");
var compressed = deflateSync(original);
var decompressed = inflateSync(compressed);
var dec = new TextDecoder();
globalThis.r = dec.decode(decompressed);"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "hello deflate");
}

#[test]
fn test_node_zlib_deflate_raw_roundtrip() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { deflateRawSync, inflateRawSync } from "node:zlib";
var original = new TextEncoder().encode("hello raw");
var compressed = deflateRawSync(original);
var decompressed = inflateRawSync(compressed);
var dec = new TextDecoder();
globalThis.r = dec.decode(decompressed);"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "hello raw");
}

#[test]
fn test_node_zlib_gzip_not_empty() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { gzipSync } from "node:zlib";
var original = new TextEncoder().encode("test");
var compressed = gzipSync(original);
globalThis.r = compressed.byteLength > 0;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

#[test]
fn test_node_zlib_default_import() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import zlib from "node:zlib";
globalThis.r = typeof zlib.gzipSync === "function" && typeof zlib.gunzipSync === "function";"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

#[test]
fn test_node_zlib_constants() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { constants } from "node:zlib";
globalThis.r = constants.Z_OK === 0 && constants.ZLIB_VERNUM === 0x12a0;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

// ── W3C 全局 API ─────────────────────────────────────────────────────────────

#[test]
fn test_global_atob() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let result = rt.eval_script("atob('SGVsbG8=')").unwrap();
    assert_eq!(result, "Hello");
}

#[test]
fn test_global_btoa() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let result = rt.eval_script("btoa('Hello')").unwrap();
    assert_eq!(result, "SGVsbG8=");
}

#[test]
fn test_global_atob_invalid() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let result = rt.eval_script(
        r#"try { atob("!!!") } catch(e) { "error:" + e.message }"#,
    );
    assert!(result.unwrap().contains("error:"));
}

#[test]
fn test_global_performance_now() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let result = rt.eval_script("typeof performance.now === 'function' && performance.now() >= 0").unwrap();
    assert_eq!(result, "true");
}

#[test]
#[allow(non_snake_case)]
fn test_global_performance_timeOrigin() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let result = rt.eval_script("typeof performance.timeOrigin === 'number' && performance.timeOrigin > 0").unwrap();
    assert_eq!(result, "true");
}

#[test]
fn test_global_performance_mark_measure() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let result = rt
        .eval_script(
            r#"performance.mark("a");
performance.mark("b");
performance.measure("x", "a", "b");
var entries = performance.getEntries();
entries.length === 3 && entries[0].name === "a" && entries[2].name === "x""#,
        )
        .unwrap();
    assert_eq!(result, "true");
}

#[test]
fn test_global_performance_clear() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let result = rt
        .eval_script(
            r#"performance.mark("x");
performance.clearMarks("x");
performance.getEntries().length === 0"#,
        )
        .unwrap();
    assert_eq!(result, "true");
}

#[test]
fn test_global_abort_controller() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let result = rt
        .eval_script(
            r#"var c = new AbortController();
c.abort();
c.signal.aborted"#,
        )
        .unwrap();
    assert_eq!(result, "true");
}

#[test]
fn test_global_abort_signal_event() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let result = rt
        .eval_script(
            r#"var c = new AbortController();
var called = false;
c.signal.addEventListener("abort", function() { called = true; });
c.abort();
called"#,
        )
        .unwrap();
    assert_eq!(result, "true");
}

#[test]
fn test_global_performance_class_global() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let result = rt
        .eval_script(
            r#"typeof PerformanceMark === 'function' && typeof Performance === 'function' && typeof PerformanceEntry === 'function'"#,
        )
        .unwrap();
    assert_eq!(result, "true");
}

#[test]
fn test_import_perf_hooks_from_node() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { performance } from "node:perf_hooks";
globalThis.r = performance === globalThis.performance;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");
}

// ── Event / EventTarget ─────────────────────────────────────────────────────

#[test]
fn test_event_constructor() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let result = rt
        .eval_script(
            r#"var e = new Event("click");
(e.type === "click" && e.target === undefined && e.defaultPrevented === false && e.cancelable === false && e.bubbles === false)"#,
        )
        .unwrap();
    assert_eq!(result, "true");
}

#[test]
fn test_event_constructor_with_options() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let result = rt
        .eval_script(
            r#"var e = new Event("custom", { bubbles: true, cancelable: true });
(e.bubbles === true && e.cancelable === true)"#,
        )
        .unwrap();
    assert_eq!(result, "true");
}

#[test]
fn test_event_prevent_default() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let result = rt
        .eval_script(
            r#"var e = new Event("test", { cancelable: true });
e.preventDefault();
e.defaultPrevented"#,
        )
        .unwrap();
    assert_eq!(result, "true");
}

#[test]
fn test_event_prevent_default_non_cancelable() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let result = rt
        .eval_script(
            r#"var e = new Event("test", { cancelable: false });
e.preventDefault();
e.defaultPrevented"#,
        )
        .unwrap();
    assert_eq!(result, "false");
}

#[test]
fn test_event_stop_propagation() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let result = rt
        .eval_script(
            r#"var e = new Event("test");
e.stopPropagation();
typeof e.stopPropagation === 'function'"#,
        )
        .unwrap();
    assert_eq!(result, "true");
}

#[test]
fn test_event_target_constructor() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let result = rt
        .eval_script(
            r#"var t = new EventTarget();
typeof t.addEventListener === 'function' && typeof t.removeEventListener === 'function' && typeof t.dispatchEvent === 'function'"#,
        )
        .unwrap();
    assert_eq!(result, "true");
}

#[test]
fn test_event_target_dispatch() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let result = rt
        .eval_script(
            r#"var t = new EventTarget();
var called = false;
t.addEventListener("test", function() { called = true; });
t.dispatchEvent(new Event("test"));
called"#,
        )
        .unwrap();
    assert_eq!(result, "true");
}

#[test]
fn test_event_target_dispatch_with_data() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let result = rt
        .eval_script(
            r#"var t = new EventTarget();
var result = [];
t.addEventListener("foo", function(e) { result.push("foo:" + e.type); });
t.addEventListener("bar", function(e) { result.push("bar:" + e.type); });
t.dispatchEvent(new Event("foo"));
result.join(",")"#,
        )
        .unwrap();
    assert_eq!(result, "foo:foo");
}

#[test]
fn test_event_target_remove_listener() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let result = rt
        .eval_script(
            r#"var t = new EventTarget();
var called = false;
function handler() { called = true; }
t.addEventListener("test", handler);
t.removeEventListener("test", handler);
t.dispatchEvent(new Event("test"));
called"#,
        )
        .unwrap();
    assert_eq!(result, "false");
}

#[test]
fn test_event_target_dispatch_return_value() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let result = rt
        .eval_script(
            r#"var t = new EventTarget();
var r1 = t.dispatchEvent(new Event("ok", { cancelable: true }));
t.addEventListener("ok", function(e) { e.preventDefault(); });
var r2 = t.dispatchEvent(new Event("ok", { cancelable: true }));
(r1 === true && r2 === false)"#,
        )
        .unwrap();
    assert_eq!(result, "true");
}

#[test]
fn test_event_target_non_callable_listener_ignored() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let result = rt
        .eval_script(
            r#"var t = new EventTarget();
t.addEventListener("test", "not a function");
t.dispatchEvent(new Event("test"));
true"#,
        )
        .unwrap();
    assert_eq!(result, "true");
}

#[test]
fn test_event_target_multiple_events() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    let result = rt
        .eval_script(
            r#"var t = new EventTarget();
var count = 0;
t.addEventListener("a", function() { count++; });
t.addEventListener("b", function() { count++; });
t.dispatchEvent(new Event("a"));
t.dispatchEvent(new Event("b"));
t.dispatchEvent(new Event("a"));
count"#,
        )
        .unwrap();
    assert_eq!(result, "3");
}

// ── CJS __dirname / __filename ────────────────────────────────────────────────

#[test]
fn test_cjs_file_dirname_filename() {
    let dir = std::env::temp_dir().join("oolong_test_cjs_dirname");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let cjs_path = dir.join("module.cjs");
    std::fs::write(
        &cjs_path,
        r#"module.exports = { dir: __dirname, file: __filename };"#,
    )
    .unwrap();

    let entry = dir.join("main.js");
    std::fs::write(
        &entry,
        format!(
            r#"import mod from "{}";
globalThis.r = mod.dir === "{}" && mod.file === "{}";"#,
            cjs_path.display(),
            dir.to_string_lossy(),
            cjs_path.to_string_lossy(),
        ),
    )
    .unwrap();

    let mut rt = oolong::runtime::OolongRuntime::new(&dir).unwrap();
    rt.eval_module_file(&entry).unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn test_cjs_transpiled_dirname_filename() {
    let dir = std::env::temp_dir().join("oolong_test_cjs_transpiled");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let js_path = dir.join("helper.js");
    std::fs::write(
        &js_path,
        r#"globalThis._d = __dirname; globalThis._f = __filename;"#,
    )
    .unwrap();

    let entry = dir.join("main.js");
    std::fs::write(
        &entry,
        format!(
            r#"import "{}";
globalThis.r = typeof globalThis._d === "string" && globalThis._d.length > 0 && typeof globalThis._f === "string" && globalThis._f.length > 0;"#,
            js_path.display(),
        ),
    )
    .unwrap();

    let mut rt = oolong::runtime::OolongRuntime::new(&dir).unwrap();
    rt.eval_module_file(&entry).unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "true");

    let _ = std::fs::remove_dir_all(dir);
}

// ── HTTP server (import + module resolved) ─────────────────────────────────

#[test]
fn test_http_serve_default_import() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import http from "@std/http";
globalThis.r = typeof http.serve;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "function");
}

#[test]
fn test_http_serve_named_import() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import { serve } from "@std/http";
globalThis.r = typeof serve;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    assert_eq!(rt.eval_script("globalThis.r").unwrap(), "function");
}

// ── .d.ts 类型一致性校验 ──────────────────────────────────────────
// 验证 Rust 实现的导出与 types/ 中的声明一致

#[test]
fn test_type_consistency_std_http() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import http from "@std/http";
globalThis._names = Object.keys(http).sort();"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let names = rt.eval_script("JSON.stringify(globalThis._names)").unwrap();
    assert_eq!(names, r#"["serve"]"#, "@std/http 导出名与 types/std/http.d.ts 不一致");
}

#[test]
fn test_type_consistency_std_path() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import path from "@std/path";
globalThis._names = Object.keys(path).sort();"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let names = rt.eval_script("JSON.stringify(globalThis._names)").unwrap();
    // 根据 types/std/path.d.ts 列出的导出
    let expected = r#"["basename","delimiter","dirname","extname","format","isAbsolute","join","normalize","parse","relative","resolve","sep"]"#;
    assert_eq!(names, expected, "@std/path 导出名与 types/std/path.d.ts 不一致");
}

#[test]
fn test_type_consistency_std_process() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import proc from "@std/process";
globalThis._keys = Object.keys(proc).sort();
globalThis._types = {};
for (const k of Object.keys(proc)) {
  globalThis._types[k] = typeof proc[k];
}"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let types_json = rt.eval_script("JSON.stringify(globalThis._types)").unwrap();
    let types: serde_json::Value = serde_json::from_str(&types_json).unwrap();
    let obj = types.as_object().unwrap();

    // 验证函数类型导出
    assert_eq!(obj.get("cwd").and_then(|v| v.as_str()), Some("function"), "process.cwd 应为 function");
    assert_eq!(obj.get("chdir").and_then(|v| v.as_str()), Some("function"), "process.chdir 应为 function");
    assert_eq!(obj.get("exit").and_then(|v| v.as_str()), Some("function"), "process.exit 应为 function");
    assert_eq!(obj.get("uptime").and_then(|v| v.as_str()), Some("function"), "process.uptime 应为 function");
    assert_eq!(obj.get("memoryUsage").and_then(|v| v.as_str()), Some("function"), "process.memoryUsage 应为 function");

    // 验证字符串/数字类型导出
    assert_eq!(obj.get("pid").and_then(|v| v.as_str()), Some("number"), "process.pid 应为 number");
    assert_eq!(obj.get("ppid").and_then(|v| v.as_str()), Some("number"), "process.ppid 应为 number");
    assert_eq!(obj.get("platform").and_then(|v| v.as_str()), Some("string"), "process.platform 应为 string");
    assert_eq!(obj.get("arch").and_then(|v| v.as_str()), Some("string"), "process.arch 应为 string");
}

#[test]
fn test_type_consistency_node_path() {
    let mut rt = oolong::runtime::OolongRuntime::new(Path::new(".")).unwrap();
    rt.eval_module_str(
        r#"import path from "node:path";
import { posix, win32 } from "node:path";
globalThis._names = Object.keys(path).sort();
globalThis._hasPosix = typeof posix === "object";
globalThis._hasWin32 = typeof win32 === "object";
globalThis._posixSep = posix && posix.sep;
globalThis._win32Sep = win32 && win32.sep;"#,
        Some(Path::new("__t.js")),
    )
    .unwrap();
    let names = rt.eval_script("JSON.stringify(globalThis._names)").unwrap();
    let expected = r#"["basename","delimiter","dirname","extname","format","isAbsolute","join","normalize","parse","relative","resolve","sep","toNamespacedPath"]"#;
    assert_eq!(names, expected, "node:path 默认导出 API 列表不匹配");
    assert_eq!(rt.eval_script("globalThis._hasPosix").unwrap(), "true", "node:path 应有 posix 命名导出");
    assert_eq!(rt.eval_script("globalThis._hasWin32").unwrap(), "true", "node:path 应有 win32 命名导出");
    assert_eq!(rt.eval_script("globalThis._posixSep").unwrap(), "/", "posix.sep 应为 /");
    assert_eq!(rt.eval_script("globalThis._win32Sep").unwrap(), "\\", "win32.sep 应为 \\");
}
