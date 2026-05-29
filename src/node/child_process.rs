use std::process;

use boa_engine::module::SyntheticModuleInitializer;
use boa_engine::object::builtins::{JsArray, JsFunction};
use boa_engine::{
    Context, JsNativeError, JsObject, JsResult, JsString, JsValue, Module, NativeFunction,
    js_string, object::FunctionObjectBuilder,
};

fn make_fn<F>(f: F, name: &str, len: usize, ctx: &mut Context) -> JsValue
where
    F: Fn(&JsValue, &[JsValue], &mut Context) -> JsResult<JsValue> + 'static,
{
    let native = unsafe { NativeFunction::from_closure(f) };
    FunctionObjectBuilder::new(ctx.realm(), native)
        .name(JsString::from(name))
        .length(len)
        .build()
        .into()
}

fn string_val(s: String) -> JsValue {
    JsValue::from(JsString::from(s))
}

fn get_args(v: &JsValue, ctx: &mut Context) -> Vec<String> {
    let mut list = Vec::new();
    if let Some(obj) = v.as_object()
        && let Ok(arr) = JsArray::from_object(obj.clone())
        && let Ok(len) = arr.length(ctx)
    {
        for i in 0..len {
            if let Ok(v) = arr.get(i, ctx)
                && let Some(s) = v.as_string()
            {
                list.push(s.to_std_string_escaped());
            }
        }
    }
    list
}

fn find_cb(args: &[JsValue]) -> Option<JsFunction> {
    args.iter()
        .rev()
        .find(|v| v.is_callable())
        .and_then(|v| v.as_object())
        .and_then(|o| JsFunction::from_object(o.clone()))
}

pub fn create_node_child_process_module(context: &mut Context) -> Result<Module, String> {
    let export_names = &[
        js_string!("exec"),
        js_string!("execSync"),
        js_string!("spawn"),
        js_string!("spawnSync"),
        js_string!("execFile"),
        js_string!("default"),
    ];

    let module = Module::synthetic(
        export_names,
        unsafe {
            SyntheticModuleInitializer::from_closure(
                move |m: &boa_engine::module::SyntheticModule, ctx: &mut Context| {
                    let exec_sync = make_fn(
                        |_: &JsValue, args: &[JsValue], _ctx: &mut Context| -> JsResult<JsValue> {
                            let command = args
                                .first()
                                .and_then(|v| v.as_string())
                                .map(|s| s.to_std_string_escaped())
                                .ok_or_else(|| {
                                    JsNativeError::typ()
                                        .with_message("execSync: command 必须是字符串")
                                })?;

                            let output = process::Command::new("sh")
                                .arg("-c")
                                .arg(&command)
                                .output()
                                .map_err(|e| {
                                    JsNativeError::typ().with_message(format!("execSync: {e}"))
                                })?;

                            if output.status.success() {
                                Ok(string_val(
                                    String::from_utf8_lossy(&output.stdout).to_string(),
                                ))
                            } else {
                                let err_msg = String::from_utf8_lossy(&output.stderr).to_string();
                                Err(JsNativeError::typ()
                                    .with_message(format!("execSync: 命令失败: {err_msg}"))
                                    .into())
                            }
                        },
                        "execSync",
                        2,
                        ctx,
                    );

                    let spawn_sync = make_fn(
                        |_: &JsValue, args: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
                            let cmd = args
                                .first()
                                .and_then(|v| v.as_string())
                                .map(|s| s.to_std_string_escaped())
                                .ok_or_else(|| {
                                    JsNativeError::typ()
                                        .with_message("spawnSync: command 必须是字符串")
                                })?;

                            let args_list = if args.len() > 1 {
                                get_args(&args[1], ctx)
                            } else {
                                Vec::new()
                            };

                            let result = process::Command::new(&cmd).args(&args_list).output();

                            let obj = JsObject::with_object_proto(ctx.intrinsics());

                            match result {
                                Ok(output) => {
                                    let _ = obj.set(
                                        js_string!("status"),
                                        JsValue::from(output.status.code().unwrap_or(-1)),
                                        false,
                                        ctx,
                                    );
                                    let _ = obj.set(
                                        js_string!("stdout"),
                                        string_val(
                                            String::from_utf8_lossy(&output.stdout).to_string(),
                                        ),
                                        false,
                                        ctx,
                                    );
                                    let _ = obj.set(
                                        js_string!("stderr"),
                                        string_val(
                                            String::from_utf8_lossy(&output.stderr).to_string(),
                                        ),
                                        false,
                                        ctx,
                                    );
                                    let _ = obj.set(
                                        js_string!("error"),
                                        JsValue::undefined(),
                                        false,
                                        ctx,
                                    );
                                }
                                Err(e) => {
                                    let _ = obj.set(
                                        js_string!("status"),
                                        JsValue::from(-1),
                                        false,
                                        ctx,
                                    );
                                    let _ = obj.set(
                                        js_string!("stdout"),
                                        string_val(String::new()),
                                        false,
                                        ctx,
                                    );
                                    let _ = obj.set(
                                        js_string!("stderr"),
                                        string_val(String::new()),
                                        false,
                                        ctx,
                                    );
                                    let _ = obj.set(
                                        js_string!("error"),
                                        string_val(format!("{e}")),
                                        false,
                                        ctx,
                                    );
                                }
                            }

                            Ok(obj.into())
                        },
                        "spawnSync",
                        3,
                        ctx,
                    );

                    let exec = make_fn(
                        |_: &JsValue, args: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
                            let command = args
                                .first()
                                .and_then(|v| v.as_string())
                                .map(|s| s.to_std_string_escaped())
                                .unwrap_or_default();

                            let cb = find_cb(args);

                            let output =
                                process::Command::new("sh").arg("-c").arg(&command).output();

                            match output {
                                Ok(out) => {
                                    if let Some(func) = cb {
                                        let _ = func.call(
                                            &JsValue::undefined(),
                                            &[
                                                JsValue::null(),
                                                string_val(
                                                    String::from_utf8_lossy(&out.stdout)
                                                        .to_string(),
                                                ),
                                            ],
                                            ctx,
                                        );
                                    }
                                    Ok(string_val(String::from_utf8_lossy(&out.stdout).to_string()))
                                }
                                Err(e) => {
                                    if let Some(func) = cb {
                                        let _ = func.call(
                                            &JsValue::undefined(),
                                            &[string_val(format!("{e}")), JsValue::undefined()],
                                            ctx,
                                        );
                                    }
                                    Err(JsNativeError::typ()
                                        .with_message(format!("exec: {e}"))
                                        .into())
                                }
                            }
                        },
                        "exec",
                        3,
                        ctx,
                    );

                    let spawn = make_fn(
                        |_: &JsValue, args: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
                            let cmd = args
                                .first()
                                .and_then(|v| v.as_string())
                                .map(|s| s.to_std_string_escaped())
                                .ok_or_else(|| {
                                    JsNativeError::typ().with_message("spawn: command 必须是字符串")
                                })?;

                            let args_list = if args.len() > 1 {
                                get_args(&args[1], ctx)
                            } else {
                                Vec::new()
                            };

                            let child = process::Command::new(&cmd).args(&args_list).spawn();

                            let cp = JsObject::with_object_proto(ctx.intrinsics());

                            match child {
                                Ok(mut c) => {
                                    let _ = cp.set(
                                        js_string!("pid"),
                                        JsValue::from(c.id() as i32),
                                        false,
                                        ctx,
                                    );
                                    let _ = cp.set(
                                        js_string!("killed"),
                                        JsValue::from(false),
                                        false,
                                        ctx,
                                    );
                                    let _ = c.wait();
                                }
                                Err(e) => {
                                    let _ = cp.set(
                                        js_string!("error"),
                                        string_val(format!("{e}")),
                                        false,
                                        ctx,
                                    );
                                }
                            }

                            Ok(cp.into())
                        },
                        "spawn",
                        3,
                        ctx,
                    );

                    let exec_file = make_fn(
                        |_: &JsValue, args: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
                            let file = args
                                .first()
                                .and_then(|v| v.as_string())
                                .map(|s| s.to_std_string_escaped())
                                .unwrap_or_default();

                            let args_list = if args.len() > 1 {
                                get_args(&args[1], ctx)
                            } else {
                                Vec::new()
                            };

                            let cb = find_cb(args);

                            let output = process::Command::new(&file).args(&args_list).output();

                            match output {
                                Ok(out) => {
                                    if let Some(func) = cb {
                                        let _ = func.call(
                                            &JsValue::undefined(),
                                            &[
                                                JsValue::null(),
                                                string_val(
                                                    String::from_utf8_lossy(&out.stdout)
                                                        .to_string(),
                                                ),
                                            ],
                                            ctx,
                                        );
                                    }
                                    Ok(string_val(String::from_utf8_lossy(&out.stdout).to_string()))
                                }
                                Err(e) => {
                                    if let Some(func) = cb {
                                        let _ = func.call(
                                            &JsValue::undefined(),
                                            &[string_val(format!("{e}")), JsValue::undefined()],
                                            ctx,
                                        );
                                    }
                                    Err(JsNativeError::typ()
                                        .with_message(format!("execFile: {e}"))
                                        .into())
                                }
                            }
                        },
                        "execFile",
                        4,
                        ctx,
                    );

                    let _ = m.set_export(&js_string!("exec"), exec.clone());
                    let _ = m.set_export(&js_string!("execSync"), exec_sync.clone());
                    let _ = m.set_export(&js_string!("spawn"), spawn.clone());
                    let _ = m.set_export(&js_string!("spawnSync"), spawn_sync.clone());
                    let _ = m.set_export(&js_string!("execFile"), exec_file.clone());

                    let default_obj = JsObject::with_object_proto(ctx.intrinsics());
                    let _ = default_obj.set(js_string!("exec"), exec, false, ctx);
                    let _ = default_obj.set(js_string!("execSync"), exec_sync, false, ctx);
                    let _ = default_obj.set(js_string!("spawn"), spawn, false, ctx);
                    let _ = default_obj.set(js_string!("spawnSync"), spawn_sync, false, ctx);
                    let _ = default_obj.set(js_string!("execFile"), exec_file, false, ctx);
                    let _ = m.set_export(&js_string!("default"), default_obj.into());

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
