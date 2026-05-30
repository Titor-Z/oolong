use boa_engine::module::SyntheticModuleInitializer;
use boa_engine::object::FunctionObjectBuilder;
use boa_engine::object::builtins::JsArray;
use boa_engine::{
    Context, JsError, JsNativeError, JsObject, JsResult, JsString, JsValue, Module, NativeFunction,
    Source, js_string,
};

fn make_native<F>(f: F) -> NativeFunction
where
    F: Fn(&JsValue, &[JsValue], &mut Context) -> JsResult<JsValue> + 'static,
{
    unsafe { NativeFunction::from_closure(f) }
}

fn get_code(args: &[JsValue]) -> JsResult<String> {
    args.first()
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .ok_or_else(|| {
            JsNativeError::typ()
                .with_message("code must be a string")
                .into()
        })
}

fn run_in_this_context_impl(
    _this: &JsValue,
    args: &[JsValue],
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let code = get_code(args)?;
    let source = Source::from_bytes(code.as_bytes());
    ctx.eval(source).map_err(|e| {
        JsNativeError::typ()
            .with_message(format!("EvalError: {e}"))
            .into()
    })
}

fn eval_with_sandbox(code: &str, sandbox_val: JsValue, ctx: &mut Context) -> JsResult<JsValue> {
    let (keys, vals) = if let Some(sandbox_obj) = sandbox_val.as_object() {
        let keys = sandbox_obj
            .own_property_keys(ctx)
            .map_err(|_| JsNativeError::typ().with_message("keys failed"))?;
        let mut k = Vec::new();
        let mut v = Vec::new();
        for key in keys {
            let name = key.to_string();
            let val = sandbox_obj
                .get(key, ctx)
                .map_err(|_| JsNativeError::typ().with_message("get failed"))?;
            k.push(name);
            v.push(val);
        }
        (k, v)
    } else {
        (Vec::new(), Vec::new())
    };

    let params = keys.join(",");
    let fn_src = format!("\"use strict\"; return ({code})");
    let wrapper_src = format!("(function({params}) {{ {fn_src} }})");
    let source = Source::from_bytes(wrapper_src.as_bytes());
    let fn_val = ctx.eval(source).map_err(|e| {
        let js_err: JsError = JsNativeError::typ()
            .with_message(format!("EvalError: {e}"))
            .into();
        js_err
    })?;

    let func_obj = fn_val
        .as_object()
        .ok_or_else(|| JsNativeError::typ().with_message("Failed to create function"))?
        .clone();
    func_obj.call(&JsValue::undefined(), &vals, ctx)
}

fn run_in_new_context_impl(
    _this: &JsValue,
    args: &[JsValue],
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let code = get_code(args)?;
    let sandbox_val = args.get(1).cloned().unwrap_or(JsValue::undefined());
    eval_with_sandbox(&code, sandbox_val, ctx)
}

fn compile_function_impl(
    _this: &JsValue,
    args: &[JsValue],
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let code = get_code(args)?;

    let params_val = args.get(1).cloned().unwrap_or(JsValue::undefined());
    let param_names: Vec<String> = if let Some(arr_obj) = params_val.as_object() {
        if let Ok(arr) = JsArray::from_object(arr_obj.clone()) {
            let len = arr.length(ctx).unwrap_or(0);
            let mut names = Vec::new();
            for i in 0..len {
                if let Ok(v) = arr.get(i, ctx) {
                    if let Some(s) = v.as_string() {
                        names.push(s.to_std_string_escaped());
                    }
                }
            }
            names
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    let params = param_names.join(",");
    let fn_src = format!("(function({params}) {{ {code} }})");
    let source = Source::from_bytes(fn_src.as_bytes());
    ctx.eval(source).map_err(|e| {
        JsNativeError::typ()
            .with_message(format!("EvalError: {e}"))
            .into()
    })
}

fn script_constructor_impl(
    _this: &JsValue,
    args: &[JsValue],
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let code = get_code(args)?;
    let options = args.get(1).cloned().unwrap_or(JsValue::undefined());

    let instance = JsObject::with_object_proto(ctx.intrinsics());
    instance
        .set(
            js_string!("_code"),
            JsValue::from(js_string!(code.as_str())),
            false,
            ctx,
        )
        .map_err(|_| JsNativeError::typ().with_message("set _code failed"))?;

    let filename = options
        .as_object()
        .and_then(|o| o.get(js_string!("filename"), ctx).ok())
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();
    instance
        .set(
            js_string!("_filename"),
            JsValue::from(js_string!(filename.as_str())),
            false,
            ctx,
        )
        .map_err(|_| JsNativeError::typ().with_message("set _filename failed"))?;

    // Set methods directly on the instance
    let run_this_fn: JsValue =
        FunctionObjectBuilder::new(ctx.realm(), make_native(script_run_this_impl))
            .name("runInThisContext")
            .length(1)
            .build()
            .into();
    instance
        .set(js_string!("runInThisContext"), run_this_fn, false, ctx)
        .map_err(|_| JsNativeError::typ().with_message("set runThis"))?;

    let run_new_fn: JsValue =
        FunctionObjectBuilder::new(ctx.realm(), make_native(script_run_new_impl))
            .name("runInNewContext")
            .length(2)
            .build()
            .into();
    instance
        .set(js_string!("runInNewContext"), run_new_fn, false, ctx)
        .map_err(|_| JsNativeError::typ().with_message("set runNew"))?;

    Ok(JsValue::from(instance))
}

fn script_run_this_impl(this: &JsValue, _args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let obj = this.as_object().ok_or_else(|| {
        JsNativeError::typ().with_message("Script.runInThisContext requires Script instance")
    })?;
    let code = obj
        .get(js_string!("_code"), ctx)
        .and_then(|v| v.to_string(ctx))
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();
    let source = Source::from_bytes(code.as_bytes());
    ctx.eval(source).map_err(|e| {
        JsNativeError::typ()
            .with_message(format!("EvalError: {e}"))
            .into()
    })
}

fn script_run_new_impl(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let obj = this.as_object().ok_or_else(|| {
        JsNativeError::typ().with_message("Script.runInNewContext requires Script instance")
    })?;
    let code = obj
        .get(js_string!("_code"), ctx)
        .and_then(|v| v.to_string(ctx))
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();
    let sandbox_val = args.first().cloned().unwrap_or(JsValue::undefined());
    eval_with_sandbox(&code, sandbox_val, ctx)
}

pub fn create_node_vm_module(context: &mut Context) -> Result<Module, String> {
    let export_names: &[JsString] = &[
        js_string!("runInThisContext"),
        js_string!("runInNewContext"),
        js_string!("compileFunction"),
        js_string!("Script"),
        js_string!("default"),
    ];

    let module = Module::synthetic(
        export_names,
        unsafe {
            SyntheticModuleInitializer::from_closure(
                move |m: &boa_engine::module::SyntheticModule, ctx: &mut Context| {
                    let run_this: JsValue = FunctionObjectBuilder::new(
                        ctx.realm(),
                        make_native(run_in_this_context_impl),
                    )
                    .name("runInThisContext")
                    .length(1)
                    .build()
                    .into();

                    let run_new: JsValue = FunctionObjectBuilder::new(
                        ctx.realm(),
                        make_native(run_in_new_context_impl),
                    )
                    .name("runInNewContext")
                    .length(2)
                    .build()
                    .into();

                    let compile: JsValue =
                        FunctionObjectBuilder::new(ctx.realm(), make_native(compile_function_impl))
                            .name("compileFunction")
                            .length(2)
                            .build()
                            .into();

                    // Script class
                    let script_ctor = FunctionObjectBuilder::new(
                        ctx.realm(),
                        make_native(script_constructor_impl),
                    )
                    .name("Script")
                    .length(1)
                    .constructor(true)
                    .build();
                    let script_ctor_val: JsValue = script_ctor.into();

                    m.set_export(&js_string!("runInThisContext"), run_this.clone())?;
                    m.set_export(&js_string!("runInNewContext"), run_new.clone())?;
                    m.set_export(&js_string!("compileFunction"), compile.clone())?;
                    m.set_export(&js_string!("Script"), script_ctor_val.clone())?;

                    let default_obj = JsObject::with_object_proto(ctx.intrinsics());
                    default_obj
                        .set(js_string!("runInThisContext"), run_this, false, ctx)
                        .map_err(|_| JsNativeError::typ().with_message("set failed"))?;
                    default_obj
                        .set(js_string!("runInNewContext"), run_new, false, ctx)
                        .map_err(|_| JsNativeError::typ().with_message("set failed"))?;
                    default_obj
                        .set(js_string!("compileFunction"), compile, false, ctx)
                        .map_err(|_| JsNativeError::typ().with_message("set failed"))?;
                    default_obj
                        .set(js_string!("Script"), script_ctor_val, false, ctx)
                        .map_err(|_| JsNativeError::typ().with_message("set failed"))?;
                    m.set_export(&js_string!("default"), JsValue::from(default_obj))?;

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
