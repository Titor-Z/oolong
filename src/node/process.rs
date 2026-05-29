use std::io::Read;

use boa_engine::{
    Context, IntoJsFunctionCopied, JsNativeError, JsObject, JsResult, JsString, JsValue, Module,
    NativeFunction, js_string, module::SyntheticModuleInitializer, object::FunctionObjectBuilder,
    object::builtins::JsArray,
};

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

fn make_fn(native: NativeFunction, name: &str, len: usize, ctx: &mut Context) -> JsValue {
    FunctionObjectBuilder::new(ctx.realm(), native)
        .name(JsString::from(name))
        .length(len)
        .build()
        .into()
}

/// 创建 "node:process" 内置模块 — Node.js 兼容
pub fn create_node_process_module(context: &mut Context) -> Result<Module, String> {
    let export_names: &[JsString] = &[
        js_string!("cwd"),
        js_string!("chdir"),
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
        js_string!("uptime"),
        js_string!("memoryUsage"),
        js_string!("stdout"),
        js_string!("stderr"),
        js_string!("stdin"),
        js_string!("nextTick"),
        js_string!("hrtime"),
        js_string!("default"),
    ];

    let module = Module::synthetic(
        export_names,
        unsafe {
            SyntheticModuleInitializer::from_closure(
                |m: &boa_engine::module::SyntheticModule, ctx: &mut Context| {
                    let pobj = JsObject::with_object_proto(ctx.intrinsics());

                    // ── cwd() ────────────────────────────────────────────────────────
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
                    let _ = pobj.set(js_string!("cwd"), cwd_fn, false, ctx);

                    // ── pid ──────────────────────────────────────────────────────────
                    let pid_val = JsValue::from(std::process::id());
                    m.set_export(&js_string!("pid"), pid_val.clone())?;
                    let _ = pobj.set(js_string!("pid"), pid_val, false, ctx);

                    // ── ppid ─────────────────────────────────────────────────────────
                    let ppid: u32 = {
                        #[cfg(unix)]
                        {
                            std::os::unix::process::parent_id()
                        }
                        #[cfg(not(unix))]
                        {
                            0
                        }
                    };
                    let ppid_val = JsValue::from(ppid);
                    m.set_export(&js_string!("ppid"), ppid_val.clone())?;
                    let _ = pobj.set(js_string!("ppid"), ppid_val, false, ctx);

                    // ── platform ─────────────────────────────────────────────────────
                    let plat = JsValue::from(js_string!(platform_str()));
                    m.set_export(&js_string!("platform"), plat.clone())?;
                    let _ = pobj.set(js_string!("platform"), plat, false, ctx);

                    // ── arch ─────────────────────────────────────────────────────────
                    let arch = JsValue::from(js_string!(arch_str()));
                    m.set_export(&js_string!("arch"), arch.clone())?;
                    let _ = pobj.set(js_string!("arch"), arch, false, ctx);

                    // ── version ──────────────────────────────────────────────────────
                    let ver = JsValue::from(js_string!("v0.1.0"));
                    m.set_export(&js_string!("version"), ver.clone())?;
                    let _ = pobj.set(js_string!("version"), ver, false, ctx);

                    // ── versions ─────────────────────────────────────────────────────
                    let versions_obj = JsObject::with_object_proto(ctx.intrinsics());
                    let _ = versions_obj.set(
                        js_string!("oolong"),
                        JsValue::from(js_string!("0.1.0-dev.2")),
                        false,
                        ctx,
                    );
                    m.set_export(&js_string!("versions"), versions_obj.clone().into())?;
                    let _ = pobj.set(js_string!("versions"), versions_obj, false, ctx);

                    // ── env ──────────────────────────────────────────────────────────
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
                    let _ = pobj.set(js_string!("env"), env_obj, false, ctx);

                    // ── argv ─────────────────────────────────────────────────────────
                    let argv_arr = JsArray::new(ctx);
                    for (i, a) in crate::runtime::get_argv().iter().enumerate() {
                        let _ =
                            argv_arr.set(i as u32, JsValue::from(JsString::from(a.as_str())), false, ctx);
                    }
                    m.set_export(&js_string!("argv"), argv_arr.clone().into())?;
                    let _ = pobj.set(js_string!("argv"), argv_arr, false, ctx);

                    // ── exit(code?) ──────────────────────────────────────────────────
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
                    let _ = pobj.set(js_string!("exit"), exit_fn, false, ctx);

                    // ── chdir(dir) ───────────────────────────────────────────────────
                    let chdir_fn = make_fn(
                        (|dir: JsString, _ctx: &mut Context| -> JsResult<JsValue> {
                            match std::env::set_current_dir(dir.to_std_string_escaped()) {
                                Ok(()) => Ok(JsValue::undefined()),
                                Err(e) => Err(JsNativeError::typ()
                                    .with_message(format!("chdir: {e}"))
                                    .into()),
                            }
                        })
                        .into_js_function_copied(ctx),
                        "chdir",
                        1,
                        ctx,
                    );
                    m.set_export(&js_string!("chdir"), chdir_fn.clone())?;
                    let _ = pobj.set(js_string!("chdir"), chdir_fn, false, ctx);

                    // ── execPath ─────────────────────────────────────────────────────
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
                    let _ = pobj.set(js_string!("execPath"), exec_path_fn, false, ctx);

                    // ── title ────────────────────────────────────────────────────────
                    let title_fn = make_fn(
                        (|new_title: Option<JsString>, _ctx: &mut Context| -> JsResult<JsValue> {
                            match new_title {
                                Some(t) => {
                                    #[cfg(unix)]
                                    {
                                        std::process::Command::new("sh")
                                            .arg("-c")
                                            .arg(format!(
                                                "printf '\\e]0;{}\\a'",
                                                t.to_std_string_escaped().replace('\'', "'\\''")
                                            ))
                                            .status()
                                            .ok();
                                    }
                                    Ok(t.into())
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
                    let _ = pobj.set(js_string!("title"), title_fn, false, ctx);

                    // ── stdout / stderr ──────────────────────────────────────────────
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
                                Err(e) => Err(JsNativeError::typ()
                                    .with_message(format!("stdout.write: {e}"))
                                    .into()),
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
                    let _ = pobj.set(js_string!("stdout"), stdout_obj, false, ctx);

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
                                Err(e) => Err(JsNativeError::typ()
                                    .with_message(format!("stderr.write: {e}"))
                                    .into()),
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
                    let _ = pobj.set(js_string!("stderr"), stderr_obj, false, ctx);

                    // ── stdin ────────────────────────────────────────────────────────
                    let stdin_read_fn = make_fn(
                        (|ctx: &mut Context| -> JsResult<JsValue> {
                            use boa_engine::object::builtins::JsPromise;
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
                                        &[JsValue::from(JsString::from(format!(
                                            "stdin.read: {e}"
                                        )))],
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
                    let stdin_obj = JsObject::with_object_proto(ctx.intrinsics());
                    let _ = stdin_obj.set(js_string!("read"), stdin_read_fn, false, ctx);
                    m.set_export(&js_string!("stdin"), stdin_obj.clone().into())?;
                    let _ = pobj.set(js_string!("stdin"), stdin_obj, false, ctx);

                    // ── nextTick ─────────────────────────────────────────────────────
                    let next_tick_fn = make_fn(
                        (|cb: JsValue, ctx: &mut Context| -> JsResult<JsValue> {
                            // 用 queueMicrotask 模拟 nextTick
                            if let Some(obj) = cb.as_object() {
                                let realm = ctx.realm().clone();
                                ctx.enqueue_job(
                                    boa_engine::job::GenericJob::new(
                                        move |ctx| {
                                            let _ = obj.call(&JsValue::undefined(), &[], ctx);
                                            Ok(JsValue::undefined())
                                        },
                                        realm,
                                    )
                                    .into(),
                                );
                            }
                            Ok(JsValue::undefined())
                        })
                        .into_js_function_copied(ctx),
                        "nextTick",
                        1,
                        ctx,
                    );
                    m.set_export(&js_string!("nextTick"), next_tick_fn.clone())?;
                    let _ = pobj.set(js_string!("nextTick"), next_tick_fn, false, ctx);

                    // ── uptime ───────────────────────────────────────────────────────
                    static START: std::sync::OnceLock<std::time::Instant> =
                        std::sync::OnceLock::new();
                    let uptime_fn = make_fn(
                        (|_: &mut Context| -> JsResult<JsValue> {
                            let start = START.get_or_init(std::time::Instant::now);
                            Ok(JsValue::from(start.elapsed().as_secs_f64()))
                        })
                        .into_js_function_copied(ctx),
                        "uptime",
                        0,
                        ctx,
                    );
                    m.set_export(&js_string!("uptime"), uptime_fn.clone())?;
                    let _ = pobj.set(js_string!("uptime"), uptime_fn, false, ctx);

                    // ── memoryUsage ──────────────────────────────────────────────────
                    let mem_usage_fn = make_fn(
                        (|ctx: &mut Context| -> JsResult<JsValue> {
                            let obj = JsObject::with_object_proto(ctx.intrinsics());
                            let _ = obj.set(js_string!("rss"), JsValue::from(0.0), false, ctx);
                            let _ =
                                obj.set(js_string!("heapTotal"), JsValue::from(0.0), false, ctx);
                            let _ = obj.set(js_string!("heapUsed"), JsValue::from(0.0), false, ctx);
                            Ok(obj.into())
                        })
                        .into_js_function_copied(ctx),
                        "memoryUsage",
                        0,
                        ctx,
                    );
                    m.set_export(&js_string!("memoryUsage"), mem_usage_fn.clone())?;
                    let _ = pobj.set(js_string!("memoryUsage"), mem_usage_fn, false, ctx);

                    // ── hrtime ───────────────────────────────────────────────────────
                    let hrtime_fn = make_fn(
                        (|ctx: &mut Context| -> JsResult<JsValue> {
                            let now = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default();
                            let arr = JsArray::new(ctx);
                            let _ = arr.push(JsValue::from(now.as_secs() as f64), ctx);
                            let _ = arr.push(JsValue::from(now.subsec_nanos() as f64), ctx);
                            Ok(arr.into())
                        })
                        .into_js_function_copied(ctx),
                        "hrtime",
                        0,
                        ctx,
                    );
                    m.set_export(&js_string!("hrtime"), hrtime_fn.clone())?;
                    let _ = pobj.set(js_string!("hrtime"), hrtime_fn, false, ctx);

                    // ── execArgv ─────────────────────────────────────────────────────
                    let exec_argv_arr = JsArray::new(ctx);
                    for (i, a) in crate::runtime::get_argv().iter().enumerate() {
                        if i > 0 {
                            let _ = exec_argv_arr.push(JsValue::from(JsString::from(a.as_str())), ctx);
                        }
                    }
                    m.set_export(&js_string!("execArgv"), exec_argv_arr.clone().into())?;
                    let _ = pobj.set(js_string!("execArgv"), exec_argv_arr, false, ctx);

                    // ── default — 整个 process 对象 ─────────────────────────────────────
                    m.set_export(&js_string!("default"), pobj.into())?;

                    Ok(())
                },
            )
        },
        None,
        None,
        context,
    );

    Ok(module)
}
