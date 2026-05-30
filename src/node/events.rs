use std::cell::Cell;

use boa_engine::module::SyntheticModuleInitializer;
use boa_engine::object::builtins::{JsArray, JsFunction, JsPromise};
use boa_engine::{
    Context, JsError, JsNativeError, JsObject, JsResult, JsString, JsValue, Module, NativeFunction,
    js_string, object::FunctionObjectBuilder,
};

thread_local! {
    static DEFAULT_MAX_LISTENERS: Cell<i32> = const { Cell::new(10) };
}

fn js_err(msg: &str) -> JsError {
    JsNativeError::typ().with_message(msg.to_string()).into()
}

fn get_obj(this: &JsValue) -> JsResult<JsObject> {
    this.as_object().ok_or_else(|| js_err("not an object"))
}

fn make_fn(f: NativeFunction, name: &str, len: usize, ctx: &mut Context) -> JsValue {
    FunctionObjectBuilder::new(ctx.realm(), f)
        .name(JsString::from(name))
        .length(len)
        .build()
        .into()
}

// ── Constructor ────────────────────────────────────────────────────

// CAUTION: When called via `new`, `_this` is the NEW TARGET (constructor function),
// NOT a new instance. Return `undefined` to let Boa create a proper instance.
fn constructor(_this: &JsValue, _args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    // Sync the Rust static from the constructor's defaultMaxListeners property
    if let Some(obj) = _this.as_object()
        && let Ok(val) = obj.get(js_string!("defaultMaxListeners"), ctx)
        && let Some(n) = val.as_number()
    {
        DEFAULT_MAX_LISTENERS.with(|c| c.set(n as i32));
    }
    Ok(JsValue::undefined())
}

fn ensure_instance_init(instance: &JsObject, ctx: &mut Context) -> JsResult<()> {
    if instance
        .has_own_property(js_string!("_events"), ctx)
        .unwrap_or(false)
    {
        return Ok(());
    }
    let max = DEFAULT_MAX_LISTENERS.with(|c| c.get() as f64);
    instance.set(js_string!("_maxListeners"), JsValue::from(max), false, ctx)?;
    let events = JsObject::with_object_proto(ctx.intrinsics());
    instance.set(js_string!("_events"), JsValue::from(events), false, ctx)?;
    Ok(())
}

// ── Helpers ────────────────────────────────────────────────────────

fn get_events(instance: &JsObject, ctx: &mut Context) -> JsResult<JsObject> {
    if !instance
        .has_own_property(js_string!("_events"), ctx)
        .unwrap_or(false)
    {
        ensure_instance_init(instance, ctx)?;
    }
    let val = instance.get(js_string!("_events"), ctx)?;
    val.as_object().ok_or_else(|| js_err("no _events"))
}

fn get_event_arr(events: &JsObject, name: &str, ctx: &mut Context) -> JsResult<JsArray> {
    let val = events.get(js_string!(name), ctx)?;
    let obj = val.as_object().ok_or_else(|| js_err("no array"))?;
    JsArray::from_object(obj.clone())
}

fn has_event(events: &JsObject, name: &str, ctx: &mut Context) -> bool {
    if let Ok(val) = events.get(js_string!(name), ctx)
        && let Some(obj) = val.as_object()
        && let Ok(arr) = JsArray::from_object(obj)
    {
        return arr.length(ctx).unwrap_or(0) > 0;
    }
    false
}

fn get_max_listeners(instance: &JsObject, ctx: &mut Context) -> f64 {
    if !instance
        .has_own_property(js_string!("_maxListeners"), ctx)
        .unwrap_or(false)
    {
        let _ = ensure_instance_init(instance, ctx);
    }
    instance
        .get(js_string!("_maxListeners"), ctx)
        .and_then(|v| v.to_number(ctx))
        .unwrap_or(10.0)
}

// ── addListener ────────────────────────────────────────────────────

fn add_listener_impl(
    instance: &JsObject,
    name: &str,
    listener: &JsValue,
    prepend: bool,
    ctx: &mut Context,
) -> JsResult<()> {
    let events = get_events(instance, ctx)?;

    if name != "newListener" && has_event(&events, "newListener", ctx) {
        let _ = emit_internal(
            instance,
            "newListener",
            &[JsValue::from(JsString::from(name)), listener.clone()],
            ctx,
        );
    }

    let arr = get_or_create_arr(&events, name, ctx)?;

    if prepend {
        arr.unshift(&[listener.clone()], ctx)?;
    } else {
        arr.push(listener.clone(), ctx)?;
    }

    let len = arr.length(ctx)?;
    let max = get_max_listeners(instance, ctx);
    if (len as f64) > max && len < 1000 {
        let msg = format!(
            "Possible EventEmitter memory leak detected. {} {} listeners added. Use emitter.setMaxListeners() to increase limit",
            len, name
        );
        let _ = emit_internal(
            instance,
            "warning",
            &[JsValue::from(JsString::from(msg))],
            ctx,
        );
    }

    Ok(())
}

fn get_or_create_arr(events: &JsObject, name: &str, ctx: &mut Context) -> JsResult<JsArray> {
    let val = events.get(js_string!(name), ctx)?;
    if let Some(obj) = val.as_object()
        && let Ok(arr) = JsArray::from_object(obj.clone())
    {
        return Ok(arr);
    }
    let arr = JsArray::new(ctx);
    events.set(js_string!(name), JsValue::from(arr.clone()), false, ctx)?;
    Ok(arr)
}

fn on_method(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let instance = get_obj(this)?;
    let name = args
        .first()
        .and_then(|v| v.to_string(ctx).ok())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();
    let listener = args.get(1).cloned().unwrap_or(JsValue::undefined());
    add_listener_impl(&instance, &name, &listener, false, ctx)?;
    Ok(this.clone())
}

fn once_method(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let instance = get_obj(this)?;
    let name = args
        .first()
        .and_then(|v| v.to_string(ctx).ok())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();
    let listener_val = args.get(1).cloned().unwrap_or(JsValue::undefined());
    let listener_obj = listener_val
        .as_object()
        .ok_or_else(|| js_err("listener must be a function"))?;
    if !listener_obj.is_callable() {
        return Err(js_err("listener must be a function"));
    }
    let listener_obj2 = listener_obj.clone();
    let called = std::cell::Cell::new(false);
    let wrapper = unsafe {
        NativeFunction::from_closure(
            move |inner_this: &JsValue,
                  inner_args: &[JsValue],
                  inner_ctx: &mut Context|
                  -> JsResult<JsValue> {
                if called.replace(true) {
                    return Ok(JsValue::undefined());
                }
                listener_obj2.call(inner_this, inner_args, inner_ctx)
            },
        )
    };
    let wrapper_val = make_fn(wrapper, "wrapper", 0, ctx);
    add_listener_impl(&instance, &name, &wrapper_val, false, ctx)?;
    Ok(this.clone())
}

fn prepend_listener_method(
    this: &JsValue,
    args: &[JsValue],
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let instance = get_obj(this)?;
    let name = args
        .first()
        .and_then(|v| v.to_string(ctx).ok())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();
    let listener = args.get(1).cloned().unwrap_or(JsValue::undefined());
    add_listener_impl(&instance, &name, &listener, true, ctx)?;
    Ok(this.clone())
}

fn prepend_once_listener_method(
    this: &JsValue,
    args: &[JsValue],
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let instance = get_obj(this)?;
    let name = args
        .first()
        .and_then(|v| v.to_string(ctx).ok())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();
    let listener_val = args.get(1).cloned().unwrap_or(JsValue::undefined());
    let listener_obj = listener_val
        .as_object()
        .ok_or_else(|| js_err("listener must be a function"))?;
    if !listener_obj.is_callable() {
        return Err(js_err("listener must be a function"));
    }
    let listener_obj2 = listener_obj.clone();
    let called = std::cell::Cell::new(false);
    let wrapper = unsafe {
        NativeFunction::from_closure(
            move |inner_this: &JsValue,
                  inner_args: &[JsValue],
                  inner_ctx: &mut Context|
                  -> JsResult<JsValue> {
                if called.replace(true) {
                    return Ok(JsValue::undefined());
                }
                listener_obj2.call(inner_this, inner_args, inner_ctx)
            },
        )
    };
    let wrapper_val = make_fn(wrapper, "wrapper", 0, ctx);
    add_listener_impl(&instance, &name, &wrapper_val, true, ctx)?;
    Ok(this.clone())
}

// ── emit ───────────────────────────────────────────────────────────

fn emit_internal(
    instance: &JsObject,
    name: &str,
    emit_args: &[JsValue],
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let events = match get_events(instance, ctx) {
        Ok(e) => e,
        Err(_) => return Ok(JsValue::from(false)),
    };
    let arr = match get_event_arr(&events, name, ctx) {
        Ok(a) => a,
        Err(_) => return Ok(JsValue::from(false)),
    };
    let len = arr.length(ctx)?;
    let mut items = Vec::new();
    for i in 0..len {
        if let Ok(item) = arr.get(i, ctx) {
            items.push(item);
        }
    }
    for item in &items {
        if let Some(obj) = item.as_object()
            && let Some(func) = JsFunction::from_object(obj.clone())
        {
            let this_val = JsValue::from(instance.clone());
            let _ = func.call(&this_val, emit_args, ctx);
        }
    }
    Ok(JsValue::from(true))
}

fn emit_method(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let instance = get_obj(this)?;
    let name = args
        .first()
        .and_then(|v| v.to_string(ctx).ok())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();
    let emit_args: Vec<JsValue> = args.iter().skip(1).cloned().collect();
    emit_internal(&instance, &name, &emit_args, ctx)
}

// ── removeListener ─────────────────────────────────────────────────

fn remove_listener_impl(
    instance: &JsObject,
    name: &str,
    listener: &JsValue,
    ctx: &mut Context,
) -> JsResult<()> {
    if !instance
        .has_own_property(js_string!("_events"), ctx)
        .unwrap_or(false)
    {
        return Ok(());
    }
    let events = match get_events(instance, ctx) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };
    let arr = match get_event_arr(&events, name, ctx) {
        Ok(a) => a,
        Err(_) => return Ok(()),
    };
    let len = arr.length(ctx)?;
    let mut items = Vec::new();
    for i in 0..len {
        if let Ok(item) = arr.get(i, ctx) {
            if listener.is_undefined() {
                continue;
            }
            let is_match = match (item.as_object(), listener.as_object()) {
                (Some(a), Some(b)) => std::ptr::eq(a.as_ref(), b.as_ref()),
                _ => false,
            };
            if is_match {
                continue;
            }
            items.push(item);
        }
    }
    if items.is_empty() {
        let _ = events.delete_property_or_throw(js_string!(name), ctx);
    } else {
        let new_arr = JsArray::from_iter(items, ctx);
        let _ = events.set(js_string!(name), JsValue::from(new_arr), false, ctx);
    }
    Ok(())
}

fn remove_listener_method(
    this: &JsValue,
    args: &[JsValue],
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let instance = get_obj(this)?;
    let name = args
        .first()
        .and_then(|v| v.to_string(ctx).ok())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();
    let listener = args.get(1).cloned().unwrap_or(JsValue::undefined());
    remove_listener_impl(&instance, &name, &listener, ctx)?;
    if let Ok(events) = get_events(&instance, ctx)
        && has_event(&events, &name, ctx)
    {
        let ev_name = JsValue::from(JsString::from(name.as_str()));
        let _ = emit_internal(&instance, "removeListener", &[ev_name, listener], ctx);
    }
    Ok(this.clone())
}

fn remove_all_listeners_method(
    this: &JsValue,
    args: &[JsValue],
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let instance = get_obj(this)?;
    if let Some(ev) = args.first() {
        let name = ev
            .to_string(ctx)
            .map(|s| s.to_std_string_escaped())
            .unwrap_or_default();
        if let Ok(events) = get_events(&instance, ctx) {
            let _ = events.delete_property_or_throw(js_string!(name.as_str()), ctx);
        }
    } else {
        let new_events = JsObject::with_object_proto(ctx.intrinsics());
        instance.set(js_string!("_events"), JsValue::from(new_events), false, ctx)?;
    }
    Ok(this.clone())
}

// ── Query methods ──────────────────────────────────────────────────

fn listener_count_method(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let instance = get_obj(this)?;
    let name = args
        .first()
        .and_then(|v| v.to_string(ctx).ok())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();
    let events = get_events(&instance, ctx)?;
    let count = if has_event(&events, &name, ctx) {
        let arr = get_event_arr(&events, &name, ctx)?;
        arr.length(ctx)? as f64
    } else {
        0.0
    };
    Ok(JsValue::from(count))
}

fn listeners_method(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let instance = get_obj(this)?;
    let name = args
        .first()
        .and_then(|v| v.to_string(ctx).ok())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();
    let events = get_events(&instance, ctx)?;
    let val = events
        .get(js_string!(name.as_str()), ctx)
        .unwrap_or(JsValue::undefined());
    if val.is_undefined() {
        return Ok(JsValue::from(JsArray::new(ctx)));
    }
    Ok(val)
}

fn event_names_method(this: &JsValue, _args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let instance = get_obj(this)?;
    let events = get_events(&instance, ctx)?;
    let keys = events.own_property_keys(ctx)?;
    let result = JsArray::new(ctx);
    for key in keys {
        if let boa_engine::property::PropertyKey::String(s) = key {
            let _ = result.push(JsValue::from(s), ctx);
        }
    }
    Ok(JsValue::from(result))
}

fn get_max_listeners_method(
    this: &JsValue,
    _args: &[JsValue],
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let instance = get_obj(this)?;
    let n = get_max_listeners(&instance, ctx);
    Ok(JsValue::from(n))
}

fn set_max_listeners_method(
    this: &JsValue,
    args: &[JsValue],
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let instance = get_obj(this)?;
    let n = args
        .first()
        .and_then(|v| v.as_number())
        .ok_or_else(|| js_err("n must be a non-negative number"))?;
    if n < 0.0 {
        return Err(js_err("n must be a non-negative number"));
    }
    instance.set(js_string!("_maxListeners"), JsValue::from(n), false, ctx)?;
    Ok(this.clone())
}

// ── Static ─────────────────────────────────────────────────────────

fn static_listener_count_method(
    _this: &JsValue,
    args: &[JsValue],
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let emitter = args
        .first()
        .ok_or_else(|| js_err("listenerCount requires emitter"))?;
    let key = args.get(1).cloned().unwrap_or(JsValue::undefined());
    listener_count_method(emitter, &[key], ctx)
}

fn static_once_method(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let emitter_val = args
        .first()
        .ok_or_else(|| js_err("EventEmitter.once requires emitter"))?;
    let emitter = get_obj(emitter_val)?;
    let event_name = args.get(1).cloned().unwrap_or(JsValue::undefined());
    let name = event_name
        .to_string(ctx)
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();

    let (promise, resolvers) = JsPromise::new_pending(ctx);
    let resolve = resolvers.resolve;

    let handler = unsafe {
        NativeFunction::from_closure(
            move |_: &JsValue,
                  handler_args: &[JsValue],
                  handler_ctx: &mut Context|
                  -> JsResult<JsValue> {
                let result = if handler_args.len() == 1 {
                    handler_args[0].clone()
                } else {
                    JsValue::from(JsArray::from_iter(
                        handler_args.iter().cloned(),
                        handler_ctx,
                    ))
                };
                let _ = resolve.call(&JsValue::undefined(), &[result], handler_ctx);
                Ok(JsValue::undefined())
            },
        )
    };

    let handler_fn = make_fn(handler, "once_handler", 0, ctx);
    add_listener_impl(&emitter, &name, &handler_fn, false, ctx)?;
    Ok(JsValue::from(promise))
}

// ── Build class ────────────────────────────────────────────────────
// Strategy: Return a callable object (the constructor function).
// The constructor creates and returns instances that have all methods
// directly set on them (no prototype chain needed).

fn build_class(ctx: &mut Context) -> JsValue {
    // Create a constructor function
    let ctor_fn = FunctionObjectBuilder::new(ctx.realm(), NativeFunction::from_fn_ptr(constructor))
        .name("EventEmitter")
        .length(0)
        .constructor(true)
        .build();

    let ctor_val: JsValue = ctor_fn.into();
    let ctor_obj = ctor_val.as_object().expect("ctor must be object");

    // Set up prototype on the constructor's existing prototype
    let proto = JsObject::with_object_proto(ctx.intrinsics());
    let _ = ctor_obj.set(
        js_string!("prototype"),
        JsValue::from(proto.clone()),
        false,
        ctx,
    );
    let _ = proto.set(js_string!("constructor"), ctor_val.clone(), false, ctx);

    // Add instance methods to prototype
    let methods: &[(&str, NativeFunction, usize)] = &[
        ("on", NativeFunction::from_fn_ptr(on_method), 2),
        ("addListener", NativeFunction::from_fn_ptr(on_method), 2),
        ("once", NativeFunction::from_fn_ptr(once_method), 2),
        (
            "prependListener",
            NativeFunction::from_fn_ptr(prepend_listener_method),
            2,
        ),
        (
            "prependOnceListener",
            NativeFunction::from_fn_ptr(prepend_once_listener_method),
            2,
        ),
        ("emit", NativeFunction::from_fn_ptr(emit_method), 1),
        (
            "removeListener",
            NativeFunction::from_fn_ptr(remove_listener_method),
            2,
        ),
        (
            "off",
            NativeFunction::from_fn_ptr(remove_listener_method),
            2,
        ),
        (
            "removeAllListeners",
            NativeFunction::from_fn_ptr(remove_all_listeners_method),
            0,
        ),
        (
            "listenerCount",
            NativeFunction::from_fn_ptr(listener_count_method),
            1,
        ),
        (
            "listeners",
            NativeFunction::from_fn_ptr(listeners_method),
            1,
        ),
        (
            "eventNames",
            NativeFunction::from_fn_ptr(event_names_method),
            0,
        ),
        (
            "getMaxListeners",
            NativeFunction::from_fn_ptr(get_max_listeners_method),
            0,
        ),
        (
            "setMaxListeners",
            NativeFunction::from_fn_ptr(set_max_listeners_method),
            1,
        ),
    ];

    for (name, func, len) in methods.iter().map(|(a, b, c)| (*a, b.clone(), *c)) {
        let fv: JsValue = FunctionObjectBuilder::new(ctx.realm(), func)
            .name(JsString::from(name))
            .length(len)
            .build()
            .into();
        let _ = proto.set(js_string!(name), fv, false, ctx);
    }

    // Static defaultMaxListeners
    let default_val = DEFAULT_MAX_LISTENERS.with(|c| JsValue::from(c.get() as f64));
    let _ = ctor_obj.set(js_string!("defaultMaxListeners"), default_val, false, ctx);

    // Static methods
    let fv: JsValue = FunctionObjectBuilder::new(
        ctx.realm(),
        NativeFunction::from_fn_ptr(static_listener_count_method),
    )
    .name("listenerCount")
    .length(2)
    .build()
    .into();
    let _ = ctor_obj.set(js_string!("listenerCount"), fv, false, ctx);

    let fv: JsValue =
        FunctionObjectBuilder::new(ctx.realm(), NativeFunction::from_fn_ptr(static_once_method))
            .name("once")
            .length(2)
            .build()
            .into();
    let _ = ctor_obj.set(js_string!("once"), fv, false, ctx);

    ctor_val
}

// ── Module ─────────────────────────────────────────────────────────

pub fn create_node_events_module(context: &mut Context) -> Result<Module, String> {
    let export_names: &[JsString] = &[js_string!("EventEmitter"), js_string!("default")];

    let module = Module::synthetic(
        export_names,
        unsafe {
            SyntheticModuleInitializer::from_closure(
                move |m: &boa_engine::module::SyntheticModule, ctx: &mut Context| {
                    let emitter = build_class(ctx);
                    m.set_export(&js_string!("EventEmitter"), emitter.clone())?;
                    m.set_export(&js_string!("default"), emitter)?;
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
