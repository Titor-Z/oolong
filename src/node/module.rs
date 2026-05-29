use std::path::Path;

use boa_engine::module::SyntheticModuleInitializer;
use boa_engine::object::builtins::JsArray;
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

fn str_val(s: &str) -> JsValue {
    JsValue::from(JsString::from(s))
}

pub fn create_node_module_module(context: &mut Context) -> Result<Module, String> {
    let export_names = &[
        js_string!("builtinModules"),
        js_string!("isBuiltin"),
        js_string!("createRequire"),
        js_string!("Module"),
        js_string!("default"),
    ];

    let module = Module::synthetic(
        export_names,
        unsafe {
            SyntheticModuleInitializer::from_closure(
                move |m: &boa_engine::module::SyntheticModule, ctx: &mut Context| {
                    let builtins = JsArray::from_iter(
                        [
                            "_http_agent",
                            "_http_client",
                            "_http_common",
                            "_http_incoming",
                            "_http_outgoing",
                            "_http_server",
                            "_stream_duplex",
                            "_stream_passthrough",
                            "_stream_readable",
                            "_stream_transform",
                            "_stream_writable",
                            "_tls_common",
                            "_tls_wrap",
                            "assert",
                            "assert/strict",
                            "async_hooks",
                            "buffer",
                            "child_process",
                            "cluster",
                            "console",
                            "constants",
                            "crypto",
                            "dgram",
                            "diagnostics_channel",
                            "dns",
                            "domain",
                            "events",
                            "fs",
                            "fs/promises",
                            "http",
                            "http2",
                            "https",
                            "inspector",
                            "module",
                            "net",
                            "os",
                            "path",
                            "perf_hooks",
                            "process",
                            "punycode",
                            "querystring",
                            "readline",
                            "repl",
                            "stream",
                            "stream/consumers",
                            "stream/promises",
                            "stream/web",
                            "string_decoder",
                            "sys",
                            "timers",
                            "timers/promises",
                            "tls",
                            "trace_events",
                            "tty",
                            "url",
                            "util",
                            "v8",
                            "vm",
                            "wasi",
                            "worker_threads",
                            "zlib",
                        ]
                        .into_iter()
                        .map(str_val),
                        ctx,
                    );

                    let is_builtin = make_fn(
                        |_: &JsValue, args: &[JsValue], _ctx: &mut Context| -> JsResult<JsValue> {
                            let name = args
                                .first()
                                .and_then(|v| v.as_string())
                                .map(|s| s.to_std_string_escaped())
                                .unwrap_or_default();
                            Ok(JsValue::from(crate::module_loader::is_builtin_module(
                                &name,
                            )))
                        },
                        "isBuiltin",
                        1,
                        ctx,
                    );

                    let create_require = make_fn(
                        |_: &JsValue, args: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
                            let filename = args
                                .first()
                                .and_then(|v| v.as_string())
                                .map(|s| s.to_std_string_escaped())
                                .unwrap_or_default();
                            let parent = Path::new(&filename);
                            let dir = parent.parent().unwrap_or(Path::new("/")).to_path_buf();

                            let require_fn = {
                                let dir = dir.clone();
                                #[allow(unused_unsafe)]
                                unsafe {
                                    NativeFunction::from_closure(
                                        move |_: &JsValue, args2: &[JsValue], ctx2: &mut Context| -> JsResult<JsValue> {
                                            let spec = args2
                                                .first()
                                                .and_then(|v| v.as_string())
                                                .map(|s| s.to_std_string_escaped())
                                                .unwrap_or_default();

                                            if crate::module_loader::is_builtin_module(&spec) {
                                                return Err(JsNativeError::typ()
                                                    .with_message(format!(
                                                        "Cannot require built-in module '{}'. Use `import \"{}\"` instead.",
                                                        spec, spec
                                                    ))
                                                    .into());
                                            }

                                            let resolver = crate::resolver::ModuleResolver::new();
                                            match resolver.resolve(&spec, &dir.join("__placeholder__.js")) {
                                                Ok(resolved) => {
                                                    match crate::cjs::load_cjs_file(&resolved, None, ctx2) {
                                                        Ok(val) => Ok(val),
                                                        Err(e) => Err(JsNativeError::typ()
                                                            .with_message(format!("require error: {e}"))
                                                            .into()),
                                                    }
                                                }
                                                Err(e) => Err(JsNativeError::typ()
                                                    .with_message(format!("Cannot find module '{spec}': {e}"))
                                                    .into()),
                                            }
                                        },
                                    )
                                }
                            };

                            let func = FunctionObjectBuilder::new(ctx.realm(), require_fn)
                                .name(JsString::from("require"))
                                .length(1)
                                .build();

                            Ok(func.into())
                        },
                        "createRequire",
                        1,
                        ctx,
                    );

                    let module_class = JsObject::with_object_proto(ctx.intrinsics());

                    let resolve_filename = make_fn(
                        |_: &JsValue, args: &[JsValue], _ctx: &mut Context| -> JsResult<JsValue> {
                            let request = args
                                .first()
                                .and_then(|v| v.as_string())
                                .map(|s| s.to_std_string_escaped())
                                .unwrap_or_default();
                            let parent_path = args
                                .get(1)
                                .and_then(|v| v.as_string())
                                .map(|s| s.to_std_string_escaped())
                                .unwrap_or_default();

                            let resolver = crate::resolver::ModuleResolver::new();
                            match resolver.resolve(&request, Path::new(&parent_path)) {
                                Ok(p) => Ok(str_val(&p.to_string_lossy())),
                                Err(e) => Err(JsNativeError::typ()
                                    .with_message(format!("Cannot resolve '{request}': {e}"))
                                    .into()),
                            }
                        },
                        "_resolveFilename",
                        2,
                        ctx,
                    );
                    let _ = module_class.set(
                        js_string!("_resolveFilename"),
                        resolve_filename,
                        false,
                        ctx,
                    );
                    let _ = module_class.set(
                        js_string!("builtinModules"),
                        builtins.clone(),
                        false,
                        ctx,
                    );

                    let builtins_val: JsValue = builtins.clone().into();
                    let _ = m.set_export(&js_string!("builtinModules"), builtins_val);
                    let _ = m.set_export(&js_string!("isBuiltin"), is_builtin.clone());
                    let _ = m.set_export(&js_string!("createRequire"), create_require.clone());
                    let _ = m.set_export(&js_string!("Module"), module_class.clone().into());

                    let default_obj = JsObject::with_object_proto(ctx.intrinsics());
                    let _ = default_obj.set(js_string!("builtinModules"), builtins, false, ctx);
                    let _ = default_obj.set(js_string!("isBuiltin"), is_builtin, false, ctx);
                    let _ =
                        default_obj.set(js_string!("createRequire"), create_require, false, ctx);
                    let _ = default_obj.set(js_string!("Module"), module_class, false, ctx);
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
