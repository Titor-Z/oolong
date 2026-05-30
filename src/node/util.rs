use std::collections::HashSet;

use boa_engine::builtins::array_buffer::ArrayBuffer;
use boa_engine::builtins::error::Error;
use boa_engine::builtins::regexp::RegExp;
use boa_engine::module::SyntheticModuleInitializer;
use boa_engine::object::FunctionObjectBuilder;
use boa_engine::object::builtins::{JsArray, JsPromise};
use boa_engine::{
    Context, JsNativeError, JsObject, JsResult, JsString, JsValue, Module, NativeFunction,
    js_string,
};

fn make_native<F>(f: F) -> NativeFunction
where
    F: Fn(&JsValue, &[JsValue], &mut Context) -> JsResult<JsValue> + 'static,
{
    unsafe { NativeFunction::from_closure(f) }
}

fn build_fn(f: NativeFunction, name: &str, len: usize, ctx: &mut Context) -> JsValue {
    FunctionObjectBuilder::new(ctx.realm(), f)
        .name(name)
        .length(len)
        .build()
        .into()
}

fn to_str(v: &JsValue, ctx: &mut Context) -> String {
    v.to_string(ctx)
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default()
}

fn is_object_of_type<T: boa_engine::object::NativeObject>(v: &JsValue) -> bool {
    v.as_object().is_some_and(|o| o.is::<T>())
}

// ── promisify ────────────────────────────────────────────────────

fn promisify_impl(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let original = args.first().cloned().unwrap_or(JsValue::undefined());
    if !original.is_object() || !original.as_object().is_some_and(|o| o.is_callable()) {
        return Err(JsNativeError::typ()
            .with_message("original must be a function")
            .into());
    }

    let original_obj = original.as_object().unwrap().clone();
    let fn_val = unsafe {
        NativeFunction::from_closure(
            move |this: &JsValue, args: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
                let (promise, resolvers) = JsPromise::new_pending(ctx);
                let resolve_val = resolvers.resolve;
                let reject_val = resolvers.reject;

                let cb = make_native(
                    move |_: &JsValue,
                          cb_args: &[JsValue],
                          ctx: &mut Context|
                          -> JsResult<JsValue> {
                        let err = cb_args.first().cloned().unwrap_or(JsValue::undefined());
                        if err.is_null_or_undefined() {
                            let results = if cb_args.len() > 2 {
                                JsValue::from(JsArray::from_iter(cb_args[1..].iter().cloned(), ctx))
                            } else if cb_args.len() == 2 {
                                cb_args[1].clone()
                            } else {
                                JsValue::undefined()
                            };
                            resolve_val
                                .call(&JsValue::undefined(), &[results], ctx)
                                .map_err(|_| JsNativeError::typ().with_message("resolve failed"))?;
                        } else {
                            reject_val
                                .call(&JsValue::undefined(), &[err], ctx)
                                .map_err(|_| JsNativeError::typ().with_message("reject failed"))?;
                        }
                        Ok(JsValue::undefined())
                    },
                );
                let cb_fn = FunctionObjectBuilder::new(ctx.realm(), cb)
                    .name("callback")
                    .length(1)
                    .build();

                let mut call_args: Vec<JsValue> = args.to_vec();
                call_args.push(JsValue::from(cb_fn));
                original_obj
                    .call(this, &call_args, ctx)
                    .map_err(|_| JsNativeError::typ().with_message("original call failed"))?;

                Ok(JsValue::from(promise))
            },
        )
    };

    let result = FunctionObjectBuilder::new(ctx.realm(), fn_val)
        .name("promisified")
        .length(1)
        .build();
    Ok(JsValue::from(result))
}

// ── callbackify ──────────────────────────────────────────────────

fn callbackify_impl(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let original = args.first().cloned().unwrap_or(JsValue::undefined());
    if !original.is_object() || !original.as_object().is_some_and(|o| o.is_callable()) {
        return Err(JsNativeError::typ()
            .with_message("original must be a function")
            .into());
    }

    let original_obj = original.as_object().unwrap().clone();
    let fn_val = unsafe {
        NativeFunction::from_closure(
            move |this: &JsValue, args: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
                let args_len = args.len();
                if args_len == 0 {
                    return Err(JsNativeError::typ()
                        .with_message("last argument must be a callback function")
                        .into());
                }

                let cb = args.last().cloned().unwrap_or(JsValue::undefined());
                let call_args: Vec<JsValue> = args[..args_len - 1].to_vec();

                let result = original_obj.call(this, &call_args, ctx);
                match result {
                    Ok(val) => {
                        if let Some(promise_obj) = val.as_object() {
                            if let Ok(promise) = JsPromise::from_object(promise_obj.clone()) {
                                let cb_on_fulfilled = cb.clone();
                                let on_fulfilled = make_native(
                                    move |_: &JsValue,
                                          then_args: &[JsValue],
                                          ctx: &mut Context|
                                          -> JsResult<JsValue> {
                                        let result_val = then_args
                                            .first()
                                            .cloned()
                                            .unwrap_or(JsValue::undefined());
                                        let cb_obj =
                                            cb_on_fulfilled.as_object().ok_or_else(|| {
                                                JsNativeError::typ()
                                                    .with_message("callback not callable")
                                            })?;
                                        cb_obj.call(
                                            &JsValue::undefined(),
                                            &[JsValue::null(), result_val],
                                            ctx,
                                        )?;
                                        Ok(JsValue::undefined())
                                    },
                                );
                                let on_fulfilled_val =
                                    FunctionObjectBuilder::new(ctx.realm(), on_fulfilled)
                                        .name("onFulfilled")
                                        .length(1)
                                        .build();
                                let _ = promise.then(Some(on_fulfilled_val), None, ctx);

                                let cb_on_rejected = cb.clone();
                                let on_rejected = make_native(
                                    move |_: &JsValue,
                                          catch_args: &[JsValue],
                                          ctx: &mut Context|
                                          -> JsResult<JsValue> {
                                        let err = catch_args
                                            .first()
                                            .cloned()
                                            .unwrap_or(JsValue::undefined());
                                        let cb_obj =
                                            cb_on_rejected.as_object().ok_or_else(|| {
                                                JsNativeError::typ()
                                                    .with_message("callback not callable")
                                            })?;
                                        cb_obj.call(&JsValue::undefined(), &[err], ctx)?;
                                        Ok(JsValue::undefined())
                                    },
                                );
                                let on_rejected_val =
                                    FunctionObjectBuilder::new(ctx.realm(), on_rejected)
                                        .name("onRejected")
                                        .length(1)
                                        .build();
                                let _ = promise.then(None, Some(on_rejected_val), ctx);
                            }
                        }
                    }
                    Err(e) => {
                        let cb_obj = cb.as_object().ok_or_else(|| {
                            JsNativeError::typ().with_message("callback not callable")
                        })?;
                        cb_obj.call(&JsValue::undefined(), &[e.to_opaque(ctx)], ctx)?;
                    }
                }
                Ok(JsValue::undefined())
            },
        )
    };

    let result = FunctionObjectBuilder::new(ctx.realm(), fn_val)
        .name("callbackified")
        .length(1)
        .build();
    Ok(JsValue::from(result))
}

// ── format ───────────────────────────────────────────────────────

fn js_stringify(v: &JsValue, ctx: &mut Context) -> String {
    if let Some(_obj) = v.as_object() {
        if let Ok(json) = ctx.global_object().get(js_string!("JSON"), ctx) {
            if let Some(json_obj) = json.as_object() {
                if let Ok(stringify) = json_obj.get(js_string!("stringify"), ctx) {
                    if let Some(fn_obj) = stringify.as_object().filter(|o| o.is_callable()) {
                        if let Ok(result) = fn_obj.call(v, &[v.clone()], ctx) {
                            return to_str(&result, ctx);
                        }
                    }
                }
            }
        }
    }
    to_str(v, ctx)
}

fn format_impl(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    if args.is_empty() {
        return Ok(JsValue::from(js_string!("")));
    }

    let fmt = args.first().cloned().unwrap_or(JsValue::undefined());
    let fmt_str = fmt.as_string().map(|s| s.to_std_string_escaped());

    let rest = &args[1..];
    let mut i = 0;

    let result = if let Some(f) = fmt_str {
        let mut out = String::new();
        let mut chars = f.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '%' {
                match chars.next() {
                    Some('%') => out.push('%'),
                    Some(spec) if i < rest.len() => {
                        let val = &rest[i];
                        i += 1;
                        match spec {
                            's' => out.push_str(&to_str(val, ctx)),
                            'd' | 'i' => {
                                let n = val.to_number(ctx).unwrap_or(0.0);
                                out.push_str(&(n as i64).to_string());
                            }
                            'f' => {
                                let n = val.to_number(ctx).unwrap_or(0.0);
                                out.push_str(&n.to_string());
                            }
                            'j' => {
                                out.push_str(&js_stringify(val, ctx));
                            }
                            'o' | 'O' => {
                                out.push_str(&inspect_value(val, 2, ctx));
                            }
                            _ => {
                                out.push('%');
                                out.push(spec);
                                i -= 1;
                            }
                        }
                    }
                    Some(spec) => {
                        out.push('%');
                        out.push(spec);
                    }
                    None => out.push('%'),
                }
            } else {
                out.push(c);
            }
        }
        out
    } else {
        inspect_value(&fmt, 2, ctx)
    };

    Ok(JsValue::from(js_string!(result.as_str())))
}

// ── inspect ──────────────────────────────────────────────────────

fn inspect_value(val: &JsValue, depth: usize, ctx: &mut Context) -> String {
    inspect_value_inner(val, depth, &mut HashSet::new(), "", ctx)
}

fn inspect_value_inner(
    val: &JsValue,
    depth: usize,
    seen: &mut HashSet<usize>,
    indent: &str,
    ctx: &mut Context,
) -> String {
    let next_indent = format!("{indent}  ");

    if val.is_null() {
        return "null".to_string();
    }
    if val.is_undefined() {
        return "undefined".to_string();
    }
    if let Some(b) = val.as_boolean() {
        return b.to_string();
    }
    if let Some(n) = val.as_number() {
        return n.to_string();
    }
    if let Some(s) = val.as_string() {
        return format!("'{}'", s.to_std_string_escaped());
    }
    if val.is_symbol() {
        return val
            .to_string(ctx)
            .map(|s| s.to_std_string_escaped())
            .unwrap_or_default();
    }
    if let Some(bi) = val.as_bigint() {
        return format!("{bi}n");
    }

    if let Some(obj) = val.as_object() {
        let ptr = obj.as_ref() as *const _ as usize;
        if seen.contains(&ptr) {
            return "[Circular]".to_string();
        }
        seen.insert(ptr);

        let is_function = obj.is_callable();
        let is_array = JsArray::from_object(obj.clone()).is_ok();

        let result = if is_function {
            let name = obj
                .get(js_string!("name"), ctx)
                .ok()
                .and_then(|v| v.as_string())
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_else(|| "(anonymous)".to_string());
            format!("[Function: {name}]")
        } else if is_array {
            let arr = JsArray::from_object(obj.clone()).unwrap();
            let len = arr.length(ctx).unwrap_or(0);
            if len == 0 {
                "[]".to_string()
            } else if depth == 0 {
                format!("[{len} items]")
            } else {
                let mut items = Vec::new();
                for i in 0..len {
                    let item = arr.get(i, ctx).unwrap_or_default();
                    items.push(format!(
                        "{next_indent}{}",
                        inspect_value_inner(&item, depth - 1, seen, &next_indent, ctx)
                    ));
                }
                format!("[\n{}\n{indent}]", items.join(",\n"))
            }
        } else {
            let keys = obj.own_property_keys(ctx).unwrap_or_default();
            if keys.is_empty() {
                "{}".to_string()
            } else if depth == 0 {
                let ctor_name = obj
                    .get(js_string!("constructor"), ctx)
                    .ok()
                    .and_then(|c| c.as_object())
                    .and_then(|o| o.get(js_string!("name"), ctx).ok())
                    .and_then(|v| v.as_string())
                    .map(|s| s.to_std_string_escaped())
                    .unwrap_or_else(|| "Object".to_string());
                format!("[{ctor_name}]")
            } else {
                let mut items = Vec::new();
                for key in &keys {
                    let key_str = key.to_string();
                    let item_val = obj.get(key.clone(), ctx).unwrap_or(JsValue::undefined());
                    items.push(format!(
                        "{next_indent}{}: {}",
                        key_str,
                        inspect_value_inner(&item_val, depth - 1, seen, &next_indent, ctx)
                    ));
                }
                let ctor_name = obj
                    .get(js_string!("constructor"), ctx)
                    .ok()
                    .and_then(|c| c.as_object())
                    .and_then(|o| o.get(js_string!("name"), ctx).ok())
                    .and_then(|v| v.as_string())
                    .map(|s| s.to_std_string_escaped())
                    .unwrap_or_default();
                if ctor_name.is_empty() || ctor_name == "Object" {
                    format!("{{\n{}\n{indent}}}", items.join(",\n"))
                } else {
                    format!("{ctor_name} {{\n{}\n{indent}}}", items.join(",\n"))
                }
            }
        };

        seen.remove(&ptr);
        return result;
    }

    to_str(val, ctx)
}

fn inspect_impl(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let obj = args.first().cloned().unwrap_or(JsValue::undefined());
    let options = args.get(1).cloned().unwrap_or(JsValue::undefined());

    let depth = options
        .as_object()
        .and_then(|o| o.get(js_string!("depth"), ctx).ok())
        .and_then(|v| v.as_number())
        .map(|n| n as usize)
        .unwrap_or(2);

    let result = inspect_value_inner(&obj, depth, &mut HashSet::new(), "", ctx);
    Ok(JsValue::from(js_string!(result.as_str())))
}

// ── deprecate ─────────────────────────────────────────────────────

fn deprecate_impl(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let fn_val = args.first().cloned().unwrap_or(JsValue::undefined());
    let msg = args
        .get(1)
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();

    let fn_obj = fn_val
        .as_object()
        .ok_or_else(|| JsNativeError::typ().with_message("fn must be a function"))?;

    if !fn_obj.is_callable() {
        return Err(JsNativeError::typ()
            .with_message("fn must be a function")
            .into());
    }

    let fn_clone = fn_obj.clone();
    let msg_clone = msg.clone();
    let warned = std::cell::Cell::new(false);

    let wrapper = unsafe {
        NativeFunction::from_closure(
            move |this: &JsValue, args: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
                if !warned.replace(true) {
                    let warning = format!("DeprecationWarning: {msg_clone}");
                    eprintln!("{warning}");
                }
                fn_clone.call(this, args, ctx)
            },
        )
    };

    let result = FunctionObjectBuilder::new(ctx.realm(), wrapper)
        .name("deprecated")
        .length(1)
        .build();
    Ok(JsValue::from(result))
}

// ── inherits ─────────────────────────────────────────────────────

fn inherits_impl(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let ctor = args.first().cloned().unwrap_or(JsValue::undefined());
    let super_ctor = args.get(1).cloned().unwrap_or(JsValue::undefined());

    let ctor_obj = ctor
        .as_object()
        .ok_or_else(|| JsNativeError::typ().with_message("ctor must be a function"))?;
    let super_obj = super_ctor
        .as_object()
        .ok_or_else(|| JsNativeError::typ().with_message("superCtor must be a function"))?;

    if let Ok(super_proto) = super_obj.get(js_string!("prototype"), ctx) {
        if let Some(super_proto_obj) = super_proto.as_object() {
            ctor_obj
                .set(
                    js_string!("prototype"),
                    JsValue::from(super_proto_obj.clone()),
                    false,
                    ctx,
                )
                .map_err(|_| JsNativeError::typ().with_message("set prototype failed"))?;
        }
    }

    ctor_obj
        .set(js_string!("super_"), super_ctor, false, ctx)
        .map_err(|_| JsNativeError::typ().with_message("set super_ failed"))?;

    Ok(JsValue::undefined())
}

// ── debuglog ──────────────────────────────────────────────────────

fn debuglog_impl(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let section = args
        .first()
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();

    let node_debug = std::env::var("NODE_DEBUG").unwrap_or_default();
    let sections: Vec<&str> = node_debug.split(',').map(|s| s.trim()).collect();
    let enabled = sections.contains(&section.as_str()) || sections.contains(&"*");

    if enabled {
        let section_clone = section.clone();
        let fn_val = unsafe {
            NativeFunction::from_closure(
                move |_: &JsValue, args: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
                    let formatted: Vec<String> = args.iter().map(|a| to_str(a, ctx)).collect();
                    let prefix = format!("{} {:?}", section_clone, std::process::id());
                    eprintln!("{}: {}", prefix, formatted.join(" "));
                    Ok(JsValue::undefined())
                },
            )
        };
        let result = FunctionObjectBuilder::new(ctx.realm(), fn_val)
            .name("debuglog")
            .length(1)
            .build();
        Ok(JsValue::from(result))
    } else {
        let noop =
            make_native(|_: &JsValue, _: &[JsValue], _: &mut Context| Ok(JsValue::undefined()));
        let result = FunctionObjectBuilder::new(ctx.realm(), noop)
            .name("debuglog")
            .length(0)
            .build();
        Ok(JsValue::from(result))
    }
}

// ── types ────────────────────────────────────────────────────────

fn build_types_obj(ctx: &mut Context) -> JsObject {
    let types = JsObject::with_object_proto(ctx.intrinsics());
    let mut set = |name: &str, f: NativeFunction, len: usize| {
        let v = build_fn(f, name, len, ctx);
        let _ = types.set(js_string!(name), v, false, ctx);
    };

    set(
        "isDate",
        make_native(|_: &JsValue, args: &[JsValue], ctx: &mut Context| {
            let v = args.first().cloned().unwrap_or(JsValue::undefined());
            let date_ctor = ctx.global_object().get(js_string!("Date"), ctx).ok();
            let is_date = date_ctor.is_some_and(|ctor| v.instance_of(&ctor, ctx).unwrap_or(false));
            Ok(JsValue::from(is_date))
        }),
        1,
    );

    set(
        "isRegExp",
        make_native(|_: &JsValue, args: &[JsValue], _ctx: &mut Context| {
            let v = args.first().cloned().unwrap_or(JsValue::undefined());
            Ok(JsValue::from(is_object_of_type::<RegExp>(&v)))
        }),
        1,
    );

    set(
        "isArrayBuffer",
        make_native(|_: &JsValue, args: &[JsValue], _ctx: &mut Context| {
            let v = args.first().cloned().unwrap_or(JsValue::undefined());
            Ok(JsValue::from(is_object_of_type::<ArrayBuffer>(&v)))
        }),
        1,
    );

    set(
        "isMap",
        make_native(|_: &JsValue, args: &[JsValue], ctx: &mut Context| {
            let v = args.first().cloned().unwrap_or(JsValue::undefined());
            let map_ctor = ctx.global_object().get(js_string!("Map"), ctx).ok();
            let is_map = map_ctor.is_some_and(|ctor| v.instance_of(&ctor, ctx).unwrap_or(false));
            Ok(JsValue::from(is_map))
        }),
        1,
    );

    set(
        "isSet",
        make_native(|_: &JsValue, args: &[JsValue], ctx: &mut Context| {
            let v = args.first().cloned().unwrap_or(JsValue::undefined());
            let set_ctor = ctx.global_object().get(js_string!("Set"), ctx).ok();
            let is_set = set_ctor.is_some_and(|ctor| v.instance_of(&ctor, ctx).unwrap_or(false));
            Ok(JsValue::from(is_set))
        }),
        1,
    );

    set(
        "isNativeError",
        make_native(|_: &JsValue, args: &[JsValue], _ctx: &mut Context| {
            let v = args.first().cloned().unwrap_or(JsValue::undefined());
            Ok(JsValue::from(is_object_of_type::<Error>(&v)))
        }),
        1,
    );

    set(
        "isTypedArray",
        make_native(|_: &JsValue, args: &[JsValue], ctx: &mut Context| {
            let v = args.first().cloned().unwrap_or(JsValue::undefined());
            let result = v.as_object().is_some_and(|obj| {
                if let Ok(ab_val) = ctx.global_object().get(js_string!("ArrayBuffer"), ctx) {
                    if let Some(ab_obj) = ab_val.as_object() {
                        if let Ok(is_view_fn) = ab_obj.get(js_string!("isView"), ctx) {
                            if let Some(fn_obj) = is_view_fn.as_object().filter(|o| o.is_callable())
                            {
                                return fn_obj
                                    .call(&JsValue::undefined(), &[JsValue::from(obj)], ctx)
                                    .ok()
                                    .map(|r| r.to_boolean())
                                    .unwrap_or(false);
                            }
                        }
                    }
                }
                false
            });
            Ok(JsValue::from(result))
        }),
        1,
    );

    types
}

// ── Module ────────────────────────────────────────────────────────

pub fn create_node_util_module(context: &mut Context) -> Result<Module, String> {
    let export_names: &[JsString] = &[
        js_string!("promisify"),
        js_string!("callbackify"),
        js_string!("format"),
        js_string!("inspect"),
        js_string!("deprecate"),
        js_string!("inherits"),
        js_string!("debuglog"),
        js_string!("types"),
        js_string!("default"),
    ];

    let module = Module::synthetic(
        export_names,
        unsafe {
            SyntheticModuleInitializer::from_closure(
                move |m: &boa_engine::module::SyntheticModule, ctx: &mut Context| {
                    let promisify = build_fn(make_native(promisify_impl), "promisify", 1, ctx);
                    let callbackify =
                        build_fn(make_native(callbackify_impl), "callbackify", 1, ctx);
                    let format = build_fn(make_native(format_impl), "format", 1, ctx);
                    let inspect = build_fn(make_native(inspect_impl), "inspect", 1, ctx);
                    let deprecate = build_fn(make_native(deprecate_impl), "deprecate", 2, ctx);
                    let inherits = build_fn(make_native(inherits_impl), "inherits", 2, ctx);
                    let debuglog = build_fn(make_native(debuglog_impl), "debuglog", 1, ctx);
                    let types = JsValue::from(build_types_obj(ctx));

                    m.set_export(&js_string!("promisify"), promisify.clone())?;
                    m.set_export(&js_string!("callbackify"), callbackify.clone())?;
                    m.set_export(&js_string!("format"), format.clone())?;
                    m.set_export(&js_string!("inspect"), inspect.clone())?;
                    m.set_export(&js_string!("deprecate"), deprecate.clone())?;
                    m.set_export(&js_string!("inherits"), inherits.clone())?;
                    m.set_export(&js_string!("debuglog"), debuglog.clone())?;
                    m.set_export(&js_string!("types"), types.clone())?;

                    let default_obj = JsObject::with_object_proto(ctx.intrinsics());
                    default_obj
                        .set(js_string!("promisify"), promisify, false, ctx)
                        .map_err(|_| JsNativeError::typ().with_message("set failed"))?;
                    default_obj
                        .set(js_string!("callbackify"), callbackify, false, ctx)
                        .map_err(|_| JsNativeError::typ().with_message("set failed"))?;
                    default_obj
                        .set(js_string!("format"), format, false, ctx)
                        .map_err(|_| JsNativeError::typ().with_message("set failed"))?;
                    default_obj
                        .set(js_string!("inspect"), inspect, false, ctx)
                        .map_err(|_| JsNativeError::typ().with_message("set failed"))?;
                    default_obj
                        .set(js_string!("deprecate"), deprecate, false, ctx)
                        .map_err(|_| JsNativeError::typ().with_message("set failed"))?;
                    default_obj
                        .set(js_string!("inherits"), inherits, false, ctx)
                        .map_err(|_| JsNativeError::typ().with_message("set failed"))?;
                    default_obj
                        .set(js_string!("debuglog"), debuglog, false, ctx)
                        .map_err(|_| JsNativeError::typ().with_message("set failed"))?;
                    default_obj
                        .set(js_string!("types"), types, false, ctx)
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
