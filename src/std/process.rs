use std::io::Read;
use std::time::Instant;

use boa_engine::module::SyntheticModuleInitializer;
use boa_engine::{
    Context, IntoJsFunctionCopied, JsError, JsNativeError, JsObject, JsResult, JsString, JsValue,
    Module, js_string,
    object::FunctionObjectBuilder,
    object::builtins::{JsArray, JsArrayBuffer, JsPromise},
};

static START_TIME: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();

fn start_time() -> Instant {
    *START_TIME.get_or_init(Instant::now)
}

fn make_fn(
    native: boa_engine::NativeFunction,
    name: &str,
    len: usize,
    ctx: &mut Context,
) -> JsValue {
    FunctionObjectBuilder::new(ctx.realm(), native)
        .name(JsString::from(name))
        .length(len)
        .build()
        .into()
}

fn platform_str() -> &'static str {
    if cfg!(target_os = "macos") {
        "darwin"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "windows") {
        "win32"
    } else {
        "unknown"
    }
}

fn arch_str() -> &'static str {
    if cfg!(target_arch = "x86_64") {
        "x64"
    } else if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        "unknown"
    }
}

/// 创建 "process" 内置模块
pub fn create_process_module(context: &mut Context) -> Result<Module, String> {
    let export_names: &[JsString] = &[
        js_string!("cwd"),
        js_string!("pid"),
        js_string!("ppid"),
        js_string!("platform"),
        js_string!("arch"),
        js_string!("title"),
        js_string!("version"),
        js_string!("versions"),
        js_string!("env"),
        js_string!("argv"),
        js_string!("execArgv"),
        js_string!("execPath"),
        js_string!("exit"),
        js_string!("chdir"),
        js_string!("uptime"),
        js_string!("memoryUsage"),
        js_string!("stdout"),
        js_string!("stderr"),
        js_string!("stdin"),
        js_string!("default"),
    ];

    let module = Module::synthetic(
        export_names,
        SyntheticModuleInitializer::from_copy_closure(
            |m: &boa_engine::module::SyntheticModule, ctx: &mut Context| {
                // ── P0: 已实现 ──────────────────────────────────────────────────────

                // cwd()
                let cwd_fn = make_fn(
                    (|_: &mut Context| -> JsResult<JsValue> {
                        match std::env::current_dir() {
                            Ok(p) => Ok(JsValue::from(JsString::from(
                                p.to_string_lossy().to_string(),
                            ))),
                            Err(e) => Err(JsNativeError::typ()
                                .with_message(format!("cwd failed: {e}"))
                                .into()),
                        }
                    })
                    .into_js_function_copied(ctx),
                    "cwd",
                    0,
                    ctx,
                );
                m.set_export(&js_string!("cwd"), cwd_fn.clone())?;

                // pid
                let pid_val = JsValue::from(std::process::id());
                m.set_export(&js_string!("pid"), pid_val)?;

                // platform
                m.set_export(
                    &js_string!("platform"),
                    JsValue::from(js_string!(platform_str())),
                )?;

                // arch
                m.set_export(&js_string!("arch"), JsValue::from(js_string!(arch_str())))?;

                // env
                let env_obj = JsObject::with_object_proto(ctx.intrinsics());
                for (k, v) in std::env::vars() {
                    let _ = env_obj.set(
                        JsString::from(k),
                        JsValue::from(JsString::from(v)),
                        false,
                        ctx,
                    );
                }
                m.set_export(&js_string!("env"), env_obj.clone().into())?;

                // argv
                let argv_arr = JsArray::new(ctx);
                for (i, a) in crate::runtime::get_argv().iter().enumerate() {
                    let _ = argv_arr
                        .set(i as u32, JsValue::from(JsString::from(a.as_str())), false, ctx);
                }
                m.set_export(&js_string!("argv"), argv_arr.clone().into())?;

                // exit(code?)
                let exit_fn = make_fn(
                    (|code: Option<i32>, _ctx: &mut Context| -> JsResult<JsValue> {
                        std::process::exit(code.unwrap_or(0));
                    })
                    .into_js_function_copied(ctx),
                    "exit",
                    1,
                    ctx,
                );
                m.set_export(&js_string!("exit"), exit_fn.clone())?;

                // ── P1 ──────────────────────────────────────────────────────────────

                // chdir(dir)
                let chdir_fn = make_fn(
                    (|dir: JsString, _ctx: &mut Context| -> JsResult<JsValue> {
                        match std::env::set_current_dir(dir.to_std_string_escaped()) {
                            Ok(()) => Ok(JsValue::undefined()),
                            Err(e) => Err(JsError::from(
                                JsNativeError::typ().with_message(format!("chdir: {e}")),
                            )),
                        }
                    })
                    .into_js_function_copied(ctx),
                    "chdir",
                    1,
                    ctx,
                );
                m.set_export(&js_string!("chdir"), chdir_fn.clone())?;

                // ppid
                let ppid_val = {
                    #[cfg(unix)]
                    {
                        std::os::unix::process::parent_id()
                    }
                    #[cfg(not(unix))]
                    {
                        0u32
                    }
                };
                m.set_export(&js_string!("ppid"), JsValue::from(ppid_val))?;

                // version
                m.set_export(&js_string!("version"), JsValue::from(js_string!("0.1.0")))?;

                // versions
                let versions_obj = JsObject::with_object_proto(ctx.intrinsics());
                let _ = versions_obj.set(
                    js_string!("oolong"),
                    JsValue::from(js_string!("0.1.0-dev.1")),
                    false,
                    ctx,
                );
                m.set_export(&js_string!("versions"), versions_obj.clone().into())?;

                // execPath
                let exec_path_fn = make_fn(
                    (|_: &mut Context| -> JsResult<JsValue> {
                        match std::env::current_exe() {
                            Ok(p) => Ok(JsValue::from(JsString::from(
                                p.to_string_lossy().to_string(),
                            ))),
                            Err(e) => Err(JsNativeError::typ()
                                .with_message(format!("execPath: {e}"))
                                .into()),
                        }
                    })
                    .into_js_function_copied(ctx),
                    "execPath",
                    0,
                    ctx,
                );
                m.set_export(&js_string!("execPath"), exec_path_fn.clone())?;

                // stdout / stderr — 简单 .write() 包装
                let stdout_write_fn = make_fn(
                    (|text: JsString, _ctx: &mut Context| -> JsResult<JsValue> {
                        use std::io::Write;
                        let s = text.to_std_string_escaped();
                        let mut out = std::io::stdout().lock();
                        match out.write_all(s.as_bytes()) {
                            Ok(()) => {
                                out.flush().ok();
                                Ok(JsValue::undefined())
                            }
                            Err(e) => Err(JsError::from(
                                JsNativeError::typ().with_message(format!("stdout.write: {e}")),
                            )),
                        }
                    })
                    .into_js_function_copied(ctx),
                    "write",
                    1,
                    ctx,
                );
                let stdout_obj = JsObject::with_object_proto(ctx.intrinsics());
                let _ = stdout_obj.set(js_string!("write"), stdout_write_fn, false, ctx);
                m.set_export(&js_string!("stdout"), stdout_obj.clone().into())?;

                let stderr_write_fn = make_fn(
                    (|text: JsString, _ctx: &mut Context| -> JsResult<JsValue> {
                        use std::io::Write;
                        let s = text.to_std_string_escaped();
                        let mut out = std::io::stderr().lock();
                        match out.write_all(s.as_bytes()) {
                            Ok(()) => {
                                out.flush().ok();
                                Ok(JsValue::undefined())
                            }
                            Err(e) => Err(JsError::from(
                                JsNativeError::typ().with_message(format!("stderr.write: {e}")),
                            )),
                        }
                    })
                    .into_js_function_copied(ctx),
                    "write",
                    1,
                    ctx,
                );
                let stderr_obj = JsObject::with_object_proto(ctx.intrinsics());
                let _ = stderr_obj.set(js_string!("write"), stderr_write_fn, false, ctx);
                m.set_export(&js_string!("stderr"), stderr_obj.clone().into())?;

                // ── stdin ───────────────────────────────────────────────────────────

                // stdin.read() → Promise<string | null>
                let stdin_read_fn = make_fn(
                    (|ctx: &mut Context| -> JsResult<JsValue> {
                        // Read entire stdin as text (like Bun.stdin.text())
                        let mut input = String::new();
                        match std::io::stdin().lock().read_to_string(&mut input) {
                            Ok(0) => Ok(JsPromise::resolve(JsValue::null(), ctx).into()),
                            Ok(_) => Ok(JsPromise::resolve(
                                JsValue::from(JsString::from(input)),
                                ctx,
                            )
                            .into()),
                            Err(e) => {
                                let (promise, resolvers) = JsPromise::new_pending(ctx);
                                let _ = resolvers.reject.call(
                                    &JsValue::undefined(),
                                    &[JsValue::from(JsString::from(format!("stdin.read: {e}")))],
                                    ctx,
                                );
                                Ok(promise.into())
                            }
                        }
                    })
                    .into_js_function_copied(ctx),
                    "read",
                    0,
                    ctx,
                );
                // stdin.readAsBytes() → Promise<ArrayBuffer | null>
                let stdin_read_bytes_fn = make_fn(
                    (|ctx: &mut Context| -> JsResult<JsValue> {
                        let mut buf = Vec::new();
                        match std::io::stdin().lock().read_to_end(&mut buf) {
                            Ok(0) => Ok(JsPromise::resolve(JsValue::null(), ctx).into()),
                            Ok(_) => match JsArrayBuffer::new(buf.len(), ctx) {
                                Ok(ab) => {
                                    if let Some(mut data) = ab.data_mut() {
                                        data.copy_from_slice(&buf);
                                    }
                                    let ab_val: JsValue = ab.into();
                                    Ok(JsPromise::resolve(ab_val, ctx).into())
                                }
                                Err(e) => {
                                    let (promise, resolvers) = JsPromise::new_pending(ctx);
                                    let _ = resolvers.reject.call(
                                        &JsValue::undefined(),
                                        &[JsValue::from(JsString::from(format!(
                                            "stdin.readAsBytes: buffer alloc: {e}"
                                        )))],
                                        ctx,
                                    );
                                    Ok(promise.into())
                                }
                            },
                            Err(e) => {
                                let (promise, resolvers) = JsPromise::new_pending(ctx);
                                let _ = resolvers.reject.call(
                                    &JsValue::undefined(),
                                    &[JsValue::from(JsString::from(format!(
                                        "stdin.readAsBytes: {e}"
                                    )))],
                                    ctx,
                                );
                                Ok(promise.into())
                            }
                        }
                    })
                    .into_js_function_copied(ctx),
                    "readAsBytes",
                    0,
                    ctx,
                );

                let stdin_obj = JsObject::with_object_proto(ctx.intrinsics());
                let _ = stdin_obj.set(js_string!("read"), stdin_read_fn, false, ctx);
                let _ = stdin_obj.set(js_string!("readAsBytes"), stdin_read_bytes_fn, false, ctx);
                m.set_export(&js_string!("stdin"), stdin_obj.clone().into())?;

                // ── P2 ──────────────────────────────────────────────────────────────

                // title (getter/setter via function)
                let title_fn = make_fn(
                    (|new_title: Option<JsString>, _ctx: &mut Context| -> JsResult<JsValue> {
                        match new_title {
                            Some(t) => {
                                let s = t.to_std_string_escaped();
                                #[cfg(unix)]
                                std::process::Command::new("sh")
                                    .arg("-c")
                                    .arg(format!(
                                        "echo -n ''; printf '\\e]0;{}\\a'",
                                        s.replace('\'', "'\\''")
                                    ))
                                    .status()
                                    .ok();
                                Ok(JsValue::from(t))
                            }
                            None => Ok(JsValue::from(js_string!("oolong"))),
                        }
                    })
                    .into_js_function_copied(ctx),
                    "title",
                    0,
                    ctx,
                );
                m.set_export(&js_string!("title"), title_fn.clone())?;

                // execArgv — argv[1..]
                let exec_argv_arr = JsArray::new(ctx);
                for (i, a) in crate::runtime::get_argv().iter().enumerate() {
                    if i > 0 {
                        let _ = exec_argv_arr.push(JsValue::from(JsString::from(a.as_str())), ctx);
                    }
                }
                m.set_export(&js_string!("execArgv"), exec_argv_arr.clone().into())?;

                // uptime()
                let uptime_fn = make_fn(
                    (|_: &mut Context| -> JsResult<JsValue> {
                        let secs = start_time().elapsed().as_secs_f64();
                        Ok(JsValue::from(secs))
                    })
                    .into_js_function_copied(ctx),
                    "uptime",
                    0,
                    ctx,
                );
                m.set_export(&js_string!("uptime"), uptime_fn.clone())?;

                // memoryUsage() — 返回 { rss, heapTotal, heapUsed }
                let mem_usage_fn = make_fn(
                    (|ctx: &mut Context| -> JsResult<JsValue> {
                        let obj = JsObject::with_object_proto(ctx.intrinsics());
                        let _ = obj.set(js_string!("rss"), JsValue::from(0.0), false, ctx);
                        let _ = obj.set(js_string!("heapTotal"), JsValue::from(0.0), false, ctx);
                        let _ = obj.set(js_string!("heapUsed"), JsValue::from(0.0), false, ctx);
                        Ok(obj.into())
                    })
                    .into_js_function_copied(ctx),
                    "memoryUsage",
                    0,
                    ctx,
                );
                // Note: rss 真实值需要 libc::getrusage 或 procfs，后续补充
                m.set_export(&js_string!("memoryUsage"), mem_usage_fn.clone())?;

                // ── default — 整个 process 对象 ─────────────────────────────────────

                let pobj = JsObject::with_object_proto(ctx.intrinsics());
                let _ = pobj.set(js_string!("cwd"), cwd_fn, false, ctx);
                let _ = pobj.set(
                    js_string!("pid"),
                    JsValue::from(std::process::id()),
                    false,
                    ctx,
                );
                let _ = pobj.set(js_string!("ppid"), JsValue::from(ppid_val), false, ctx);
                let _ = pobj.set(
                    js_string!("platform"),
                    JsValue::from(js_string!(platform_str())),
                    false,
                    ctx,
                );
                let _ = pobj.set(
                    js_string!("arch"),
                    JsValue::from(js_string!(arch_str())),
                    false,
                    ctx,
                );
                let _ = pobj.set(js_string!("title"), title_fn, false, ctx);
                let _ = pobj.set(
                    js_string!("version"),
                    JsValue::from(js_string!("0.1.0")),
                    false,
                    ctx,
                );
                let _ = pobj.set(js_string!("versions"), versions_obj, false, ctx);
                let _ = pobj.set(js_string!("chdir"), chdir_fn, false, ctx);
                let _ = pobj.set(js_string!("exit"), exit_fn, false, ctx);
                let _ = pobj.set(js_string!("execPath"), exec_path_fn, false, ctx);
                let _ = pobj.set(js_string!("uptime"), uptime_fn, false, ctx);
                let _ = pobj.set(js_string!("memoryUsage"), mem_usage_fn, false, ctx);
                let _ = pobj.set(js_string!("env"), env_obj, false, ctx);
                let _ = pobj.set(js_string!("argv"), argv_arr, false, ctx);
                let _ = pobj.set(js_string!("execArgv"), exec_argv_arr, false, ctx);
                let _ = pobj.set(js_string!("stdout"), stdout_obj, false, ctx);
                let _ = pobj.set(js_string!("stderr"), stderr_obj, false, ctx);
                let _ = pobj.set(js_string!("stdin"), stdin_obj, false, ctx);
                m.set_export(&js_string!("default"), pobj.into())?;

                Ok(())
            },
        ),
        None,
        None,
        context,
    );

    Ok(module)
}
