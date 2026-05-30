use std::collections::HashSet;

use boa_engine::module::SyntheticModuleInitializer;
use boa_engine::object::FunctionObjectBuilder;
use boa_engine::object::builtins::JsArray;
use boa_engine::{
    Context, JsError, JsNativeError, JsObject, JsResult, JsString, JsValue, Module, NativeFunction,
    js_string,
};

fn make_native<F>(f: F) -> NativeFunction
where
    F: Fn(&JsValue, &[JsValue], &mut Context) -> JsResult<JsValue> + 'static,
{
    unsafe { NativeFunction::from_closure(f) }
}

fn js_err(msg: &str) -> JsError {
    JsNativeError::typ().with_message(msg.to_string()).into()
}

fn make_assertion_err(
    msg: &str,
    actual: JsValue,
    expected: JsValue,
    operator: &str,
    ctx: &mut Context,
) -> JsError {
    let err = JsObject::with_object_proto(ctx.intrinsics());
    let _ = err.set(js_string!("name"), js_string!("AssertionError"), false, ctx);
    let _ = err.set(js_string!("message"), js_string!(msg), false, ctx);
    let _ = err.set(js_string!("actual"), actual, false, ctx);
    let _ = err.set(js_string!("expected"), expected, false, ctx);
    let _ = err.set(js_string!("operator"), js_string!(operator), false, ctx);
    JsNativeError::typ().with_message(msg.to_string()).into()
}

fn to_display(v: &JsValue, ctx: &mut Context) -> String {
    v.to_string(ctx)
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_else(|_| format!("{v:?}"))
}

fn loose_equals(a: &JsValue, b: &JsValue, ctx: &mut Context) -> bool {
    if a.type_of() == b.type_of() {
        return a.strict_equals(b);
    }
    if (a.is_null() && b.is_undefined()) || (a.is_undefined() && b.is_null()) {
        return true;
    }
    if a.is_number() && b.is_string() {
        let a_n = a.as_number().unwrap_or(0.0);
        let b_n = b
            .to_string(ctx)
            .ok()
            .and_then(|s| s.to_std_string_escaped().parse::<f64>().ok())
            .unwrap_or(f64::NAN);
        return a_n == b_n || (a_n.is_nan() && b_n.is_nan());
    }
    if a.is_string() && b.is_number() {
        return loose_equals(b, a, ctx);
    }
    if a.is_boolean() {
        let n = if a.to_boolean() { 1.0 } else { 0.0 };
        return loose_equals(&JsValue::from(n), b, ctx);
    }
    if b.is_boolean() {
        let n = if b.to_boolean() { 1.0 } else { 0.0 };
        return loose_equals(a, &JsValue::from(n), ctx);
    }
    false
}

fn deep_equals(
    a: &JsValue,
    b: &JsValue,
    strict: bool,
    ctx: &mut Context,
    seen: &mut HashSet<usize>,
) -> bool {
    if strict {
        if a.strict_equals(b) {
            return true;
        }
    } else if loose_equals(a, b, ctx) {
        return true;
    }

    let Some(a_obj) = a.as_object() else {
        return false;
    };
    let Some(b_obj) = b.as_object() else {
        return false;
    };

    let a_ptr = a_obj.as_ref() as *const _ as usize;
    let b_ptr = b_obj.as_ref() as *const _ as usize;

    if a_ptr == b_ptr {
        return true;
    }
    if seen.contains(&a_ptr) || seen.contains(&b_ptr) {
        return false;
    }
    seen.insert(a_ptr);
    seen.insert(b_ptr);

    let result = if let (Ok(a_arr), Ok(b_arr)) = (
        JsArray::from_object(a_obj.clone()),
        JsArray::from_object(b_obj.clone()),
    ) {
        let a_len = a_arr.length(ctx).unwrap_or(0);
        let b_len = b_arr.length(ctx).unwrap_or(0);
        if a_len != b_len {
            return false;
        }
        for i in 0..a_len {
            let a_v = a_arr.get(i, ctx).unwrap_or_default();
            let b_v = b_arr.get(i, ctx).unwrap_or_default();
            if !deep_equals(&a_v, &b_v, strict, ctx, seen) {
                return false;
            }
        }
        true
    } else {
        let a_keys = a_obj.own_property_keys(ctx).unwrap_or_default();
        let b_keys = b_obj.own_property_keys(ctx).unwrap_or_default();
        if a_keys.len() != b_keys.len() {
            return false;
        }

        let mut a_key_strs: Vec<String> = a_keys.iter().map(|k| k.to_string()).collect();
        let mut b_key_strs: Vec<String> = b_keys.iter().map(|k| k.to_string()).collect();
        a_key_strs.sort();
        b_key_strs.sort();

        for (ak, bk) in a_key_strs.iter().zip(b_key_strs.iter()) {
            if ak != bk {
                return false;
            }
            let a_v = a_obj.get(js_string!(ak.as_str()), ctx).unwrap_or_default();
            let b_v = b_obj.get(js_string!(bk.as_str()), ctx).unwrap_or_default();
            if !deep_equals(&a_v, &b_v, strict, ctx, seen) {
                return false;
            }
        }
        true
    };

    seen.remove(&a_ptr);
    seen.remove(&b_ptr);
    result
}

fn ok_impl(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let value = args.first().cloned().unwrap_or(JsValue::undefined());
    let msg = args
        .get(1)
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();
    if !value.to_boolean() {
        let actual_msg = if msg.is_empty() {
            "The expression evaluated to a falsy value".to_string()
        } else {
            msg
        };
        return Err(make_assertion_err(
            &actual_msg,
            value,
            JsValue::from(true),
            "==",
            ctx,
        ));
    }
    Ok(JsValue::undefined())
}

fn equal_impl(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let actual = args.first().cloned().unwrap_or(JsValue::undefined());
    let expected = args.get(1).cloned().unwrap_or(JsValue::undefined());
    let msg = args
        .get(2)
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();
    if !loose_equals(&actual, &expected, ctx) {
        let err_msg = if msg.is_empty() {
            format!(
                "{} != {}",
                to_display(&actual, ctx),
                to_display(&expected, ctx)
            )
        } else {
            msg
        };
        return Err(make_assertion_err(&err_msg, actual, expected, "==", ctx));
    }
    Ok(JsValue::undefined())
}

fn not_equal_impl(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let actual = args.first().cloned().unwrap_or(JsValue::undefined());
    let expected = args.get(1).cloned().unwrap_or(JsValue::undefined());
    let msg = args
        .get(2)
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();
    if loose_equals(&actual, &expected, ctx) {
        let err_msg = if msg.is_empty() {
            format!(
                "{} == {}",
                to_display(&actual, ctx),
                to_display(&expected, ctx)
            )
        } else {
            msg
        };
        return Err(make_assertion_err(&err_msg, actual, expected, "!=", ctx));
    }
    Ok(JsValue::undefined())
}

fn strict_equal_impl(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let actual = args.first().cloned().unwrap_or(JsValue::undefined());
    let expected = args.get(1).cloned().unwrap_or(JsValue::undefined());
    let msg = args
        .get(2)
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();
    if !actual.strict_equals(&expected) {
        let err_msg = if msg.is_empty() {
            format!(
                "{} !== {}",
                to_display(&actual, ctx),
                to_display(&expected, ctx)
            )
        } else {
            msg
        };
        return Err(make_assertion_err(&err_msg, actual, expected, "===", ctx));
    }
    Ok(JsValue::undefined())
}

fn not_strict_equal_impl(
    _this: &JsValue,
    args: &[JsValue],
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let actual = args.first().cloned().unwrap_or(JsValue::undefined());
    let expected = args.get(1).cloned().unwrap_or(JsValue::undefined());
    let msg = args
        .get(2)
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();
    if actual.strict_equals(&expected) {
        let err_msg = if msg.is_empty() {
            format!(
                "{} === {}",
                to_display(&actual, ctx),
                to_display(&expected, ctx)
            )
        } else {
            msg
        };
        return Err(make_assertion_err(&err_msg, actual, expected, "!==", ctx));
    }
    Ok(JsValue::undefined())
}

fn deep_equal_impl(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let actual = args.first().cloned().unwrap_or(JsValue::undefined());
    let expected = args.get(1).cloned().unwrap_or(JsValue::undefined());
    let msg = args
        .get(2)
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();
    let mut seen = HashSet::new();
    if !deep_equals(&actual, &expected, false, ctx, &mut seen) {
        let err_msg = if msg.is_empty() {
            "notDeepEqual".to_string()
        } else {
            msg
        };
        return Err(make_assertion_err(
            &err_msg,
            actual,
            expected,
            "deepEqual",
            ctx,
        ));
    }
    Ok(JsValue::undefined())
}

fn not_deep_equal_impl(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let actual = args.first().cloned().unwrap_or(JsValue::undefined());
    let expected = args.get(1).cloned().unwrap_or(JsValue::undefined());
    let msg = args
        .get(2)
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();
    let mut seen = HashSet::new();
    if deep_equals(&actual, &expected, false, ctx, &mut seen) {
        let err_msg = if msg.is_empty() {
            "deepEqual".to_string()
        } else {
            msg
        };
        return Err(make_assertion_err(
            &err_msg,
            actual,
            expected,
            "notDeepEqual",
            ctx,
        ));
    }
    Ok(JsValue::undefined())
}

fn deep_strict_equal_impl(
    _this: &JsValue,
    args: &[JsValue],
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let actual = args.first().cloned().unwrap_or(JsValue::undefined());
    let expected = args.get(1).cloned().unwrap_or(JsValue::undefined());
    let msg = args
        .get(2)
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();
    let mut seen = HashSet::new();
    if !deep_equals(&actual, &expected, true, ctx, &mut seen) {
        let err_msg = if msg.is_empty() {
            "notDeepStrictEqual".to_string()
        } else {
            msg
        };
        return Err(make_assertion_err(
            &err_msg,
            actual,
            expected,
            "deepStrictEqual",
            ctx,
        ));
    }
    Ok(JsValue::undefined())
}

fn not_deep_strict_equal_impl(
    _this: &JsValue,
    args: &[JsValue],
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let actual = args.first().cloned().unwrap_or(JsValue::undefined());
    let expected = args.get(1).cloned().unwrap_or(JsValue::undefined());
    let msg = args
        .get(2)
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();
    let mut seen = HashSet::new();
    if deep_equals(&actual, &expected, true, ctx, &mut seen) {
        let err_msg = if msg.is_empty() {
            "deepStrictEqual".to_string()
        } else {
            msg
        };
        return Err(make_assertion_err(
            &err_msg,
            actual,
            expected,
            "notDeepStrictEqual",
            ctx,
        ));
    }
    Ok(JsValue::undefined())
}

fn fail_impl(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let msg = args
        .first()
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_else(|| "Failed".to_string());
    Err(make_assertion_err(
        &msg,
        JsValue::undefined(),
        JsValue::undefined(),
        "fail",
        ctx,
    ))
}

fn if_error_impl(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let val = args.first().cloned().unwrap_or(JsValue::undefined());
    if !val.is_null_or_undefined() {
        let msg = format!("ifError got unwanted exception: {val:?}");
        return Err(make_assertion_err(
            &msg,
            val,
            JsValue::undefined(),
            "ifError",
            ctx,
        ));
    }
    Ok(JsValue::undefined())
}

fn throws_impl(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let fn_val = args.first().cloned().unwrap_or(JsValue::undefined());
    let error = args.get(1).cloned().unwrap_or(JsValue::undefined());
    let msg = args
        .get(2)
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped());

    let fn_obj = fn_val
        .as_object()
        .ok_or_else(|| js_err("fn must be a function"))?;
    let result = fn_obj.call(&JsValue::undefined(), &[], ctx);

    match result {
        Ok(_) => Err(make_assertion_err(
            msg.as_deref().unwrap_or("Missing expected exception"),
            JsValue::undefined(),
            error,
            "throws",
            ctx,
        )),
        Err(e) => {
            let thrown = e.to_opaque(ctx);

            if !error.is_undefined() {
                if let Some(err_ctor) = error.as_object() {
                    let ctor_val = JsValue::from(err_ctor.clone());
                    let is_instance = thrown.instance_of(&ctor_val, ctx).unwrap_or(false);
                    if !is_instance {
                        return Err(make_assertion_err(
                            msg.as_deref().unwrap_or(
                                "The error was not an instance of the expected constructor",
                            ),
                            JsValue::from(thrown),
                            error,
                            "throws",
                            ctx,
                        ));
                    }
                }
            }
            Ok(JsValue::undefined())
        }
    }
}

fn does_not_throw_impl(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let fn_val = args.first().cloned().unwrap_or(JsValue::undefined());
    let _msg = args
        .get(1)
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();
    let fn_obj = fn_val
        .as_object()
        .ok_or_else(|| js_err("fn must be a function"))?;
    let result = fn_obj.call(&JsValue::undefined(), &[], ctx);
    if let Err(e) = result {
        let opaque = e.to_opaque(ctx);
        return Err(make_assertion_err(
            &format!("The function threw an unexpected exception"),
            JsValue::from(opaque),
            JsValue::undefined(),
            "doesNotThrow",
            ctx,
        ));
    }
    Ok(JsValue::undefined())
}

fn assertion_error_ctor_impl(
    _this: &JsValue,
    args: &[JsValue],
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let options = args.first().cloned().unwrap_or(JsValue::undefined());

    let instance = JsObject::with_object_proto(ctx.intrinsics());
    instance
        .set(js_string!("name"), js_string!("AssertionError"), false, ctx)
        .map_err(|_| JsNativeError::typ().with_message("set name"))?;

    if let Some(opts_obj) = options.as_object() {
        if let Ok(msg) = opts_obj.get(js_string!("message"), ctx) {
            instance
                .set(js_string!("message"), msg, false, ctx)
                .map_err(|_| JsNativeError::typ().with_message("set message"))?;
        }
        if let Ok(actual) = opts_obj.get(js_string!("actual"), ctx) {
            instance
                .set(js_string!("actual"), actual, false, ctx)
                .map_err(|_| JsNativeError::typ().with_message("set actual"))?;
        }
        if let Ok(expected) = opts_obj.get(js_string!("expected"), ctx) {
            instance
                .set(js_string!("expected"), expected, false, ctx)
                .map_err(|_| JsNativeError::typ().with_message("set expected"))?;
        }
        if let Ok(operator) = opts_obj.get(js_string!("operator"), ctx) {
            instance
                .set(js_string!("operator"), operator, false, ctx)
                .map_err(|_| JsNativeError::typ().with_message("set operator"))?;
        }
    } else {
        instance
            .set(js_string!("message"), options, false, ctx)
            .map_err(|_| JsNativeError::typ().with_message("set message"))?;
    }

    Ok(JsValue::from(instance))
}

fn build_fn(f: NativeFunction, name: &str, len: usize, ctx: &mut Context) -> JsValue {
    FunctionObjectBuilder::new(ctx.realm(), f)
        .name(name)
        .length(len)
        .build()
        .into()
}

fn build_assert_obj(ctx: &mut Context, strict_mode: bool) -> JsObject {
    let obj = JsObject::with_object_proto(ctx.intrinsics());
    let mut set = |name: &str, f: NativeFunction, len: usize| {
        let v = build_fn(f, name, len, ctx);
        let _ = obj.set(js_string!(name), v, false, ctx);
    };

    set("ok", make_native(ok_impl), 2);
    if strict_mode {
        set("equal", make_native(strict_equal_impl), 3);
        set("notEqual", make_native(not_strict_equal_impl), 3);
    } else {
        set("equal", make_native(equal_impl), 3);
        set("notEqual", make_native(not_equal_impl), 3);
    }
    set("strictEqual", make_native(strict_equal_impl), 3);
    set("notStrictEqual", make_native(not_strict_equal_impl), 3);
    if strict_mode {
        set("deepEqual", make_native(deep_strict_equal_impl), 3);
        set("notDeepEqual", make_native(not_deep_strict_equal_impl), 3);
    } else {
        set("deepEqual", make_native(deep_equal_impl), 3);
        set("notDeepEqual", make_native(not_deep_equal_impl), 3);
    }
    set("deepStrictEqual", make_native(deep_strict_equal_impl), 3);
    set(
        "notDeepStrictEqual",
        make_native(not_deep_strict_equal_impl),
        3,
    );
    set("throws", make_native(throws_impl), 3);
    set("doesNotThrow", make_native(does_not_throw_impl), 2);
    set("ifError", make_native(if_error_impl), 1);
    set("fail", make_native(fail_impl), 1);
    obj
}

pub fn create_node_assert_module(context: &mut Context) -> Result<Module, String> {
    let export_names: &[JsString] = &[
        js_string!("AssertionError"),
        js_string!("ok"),
        js_string!("equal"),
        js_string!("notEqual"),
        js_string!("strictEqual"),
        js_string!("notStrictEqual"),
        js_string!("deepEqual"),
        js_string!("notDeepEqual"),
        js_string!("deepStrictEqual"),
        js_string!("notDeepStrictEqual"),
        js_string!("throws"),
        js_string!("doesNotThrow"),
        js_string!("ifError"),
        js_string!("fail"),
        js_string!("strict"),
        js_string!("default"),
    ];

    let module = Module::synthetic(
        export_names,
        unsafe {
            SyntheticModuleInitializer::from_closure(
                move |m: &boa_engine::module::SyntheticModule, ctx: &mut Context| {
                    let main = build_assert_obj(ctx, false);
                    let strict = build_assert_obj(ctx, true);

                    for &name in &[
                        "ok",
                        "equal",
                        "notEqual",
                        "strictEqual",
                        "notStrictEqual",
                        "deepEqual",
                        "notDeepEqual",
                        "deepStrictEqual",
                        "notDeepStrictEqual",
                        "throws",
                        "doesNotThrow",
                        "ifError",
                        "fail",
                    ] {
                        let js_name = JsString::from(name);
                        let val = main
                            .get(js_name.clone(), ctx)
                            .map_err(|_| JsNativeError::typ().with_message(format!("no {name}")))?;
                        m.set_export(&js_name, val)?;
                    }

                    let assertion_error_ctor = FunctionObjectBuilder::new(
                        ctx.realm(),
                        make_native(assertion_error_ctor_impl),
                    )
                    .name("AssertionError")
                    .length(1)
                    .constructor(true)
                    .build();
                    let assertion_error_val: JsValue = assertion_error_ctor.into();
                    m.set_export(&js_string!("AssertionError"), assertion_error_val)?;
                    m.set_export(&js_string!("strict"), JsValue::from(strict))?;
                    m.set_export(&js_string!("default"), JsValue::from(main))?;

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
