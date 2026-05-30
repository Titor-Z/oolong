mod common;
use std::path::Path;

#[test]
fn test_import_process_cwd() {
    let mut rt = common::create_runtime();
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
    let mut rt = common::create_runtime();
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
    let mut rt = common::create_runtime();
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
    let mut rt = common::create_runtime();
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
    let mut rt = common::create_runtime();
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
    let mut rt = common::create_runtime();
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
    let mut rt = common::create_runtime();
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

    let mut rt = common::create_runtime();
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
    let mut rt = common::create_runtime();
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
    let mut rt = common::create_runtime();
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
    let mut rt = common::create_runtime();
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
    let mut rt = common::create_runtime();
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
    let mut rt = common::create_runtime();
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
    let mut rt = common::create_runtime();
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
    let mut rt = common::create_runtime();
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
    let mut rt = common::create_runtime();
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
    let mut rt = common::create_runtime();
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
    let mut rt = common::create_runtime();
    // console.log should not throw
    let result = rt.eval_script("console.log('test'); 42").unwrap();
    assert_eq!(result, "42");
}
