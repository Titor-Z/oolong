use std::cell::Cell;
use std::rc::Rc;

use boa_engine::module::SyntheticModuleInitializer;
use boa_engine::object::FunctionObjectBuilder;
use boa_engine::object::builtins::{JsArray, JsFunction};
use boa_engine::{
    Context, JsError, JsNativeError, JsObject, JsResult, JsString, JsValue, Module, NativeFunction,
    js_string,
};

fn js_err(msg: &str) -> JsError {
    JsNativeError::typ().with_message(msg.to_string()).into()
}

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

fn get_obj(v: &JsValue) -> JsResult<JsObject> {
    v.as_object().ok_or_else(|| js_err("not an object"))
}

// ── EventEmitter helpers ─────────────────────────────────────────

fn ee_init(instance: &JsObject, ctx: &mut Context) -> JsResult<()> {
    if instance
        .has_own_property(js_string!("_events"), ctx)
        .unwrap_or(false)
    {
        return Ok(());
    }
    let _ = instance.set(js_string!("_maxListeners"), JsValue::from(10), false, ctx);
    let events = JsObject::with_object_proto(ctx.intrinsics());
    let _ = instance.set(js_string!("_events"), JsValue::from(events), false, ctx);
    Ok(())
}

fn get_events(instance: &JsObject, ctx: &mut Context) -> JsResult<JsObject> {
    if !instance
        .has_own_property(js_string!("_events"), ctx)
        .unwrap_or(false)
    {
        ee_init(instance, ctx)?;
    }
    match instance.get(js_string!("_events"), ctx) {
        Ok(v) => v.as_object().ok_or_else(|| js_err("no _events")),
        Err(e) => Err(e),
    }
}

fn get_or_create_arr(events: &JsObject, name: &str, ctx: &mut Context) -> JsResult<JsArray> {
    let val = events
        .get(js_string!(name), ctx)
        .unwrap_or(JsValue::undefined());
    if let Some(obj) = val.as_object() {
        if let Ok(arr) = JsArray::from_object(obj.clone()) {
            return Ok(arr);
        }
    }
    let arr = JsArray::new(ctx);
    let _ = events.set(js_string!(name), JsValue::from(arr.clone()), false, ctx);
    Ok(arr)
}

fn add_listener_impl(
    instance: &JsObject,
    name: &str,
    listener: &JsValue,
    ctx: &mut Context,
) -> JsResult<()> {
    let events = get_events(instance, ctx)?;
    let arr = get_or_create_arr(&events, name, ctx)?;
    let _ = arr.push(listener.clone(), ctx);
    Ok(())
}

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
    let val = events
        .get(js_string!(name), ctx)
        .unwrap_or(JsValue::undefined());
    let arr = match val
        .as_object()
        .and_then(|o| JsArray::from_object(o.clone()).ok())
    {
        Some(a) => a,
        None => return Ok(JsValue::from(false)),
    };
    let items: Vec<JsValue> = (0..arr.length(ctx).unwrap_or(0))
        .filter_map(|i| arr.get(i, ctx).ok())
        .collect();
    for item in &items {
        if let Some(obj) = item.as_object() {
            if let Some(func) = JsFunction::from_object(obj.clone()) {
                let _ = func.call(&JsValue::from(instance.clone()), emit_args, ctx);
            }
        }
    }
    Ok(JsValue::from(true))
}

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
    let val = events
        .get(js_string!(name), ctx)
        .unwrap_or(JsValue::undefined());
    let arr = match val
        .as_object()
        .and_then(|o| JsArray::from_object(o.clone()).ok())
    {
        Some(a) => a,
        None => return Ok(()),
    };
    let keep: Vec<JsValue> = (0..arr.length(ctx).unwrap_or(0))
        .filter_map(|i| arr.get(i, ctx).ok())
        .filter(|item| match (item.as_object(), listener.as_object()) {
            (Some(a), Some(b)) => !std::ptr::eq(a.as_ref(), b.as_ref()),
            _ => true,
        })
        .collect();
    if keep.is_empty() {
        let _ = events.delete_property_or_throw(js_string!(name), ctx);
    } else {
        let _ = events.set(
            js_string!(name),
            JsValue::from(JsArray::from_iter(keep, ctx)),
            false,
            ctx,
        );
    }
    Ok(())
}

// ── Stream methods ──────────────────────────────────────────────

fn ee_on(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let inst = get_obj(this)?;
    let name = args
        .first()
        .and_then(|v| v.to_string(ctx).ok())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();
    let listener = args.get(1).cloned().unwrap_or(JsValue::undefined());
    add_listener_impl(&inst, &name, &listener, ctx)?;
    Ok(this.clone())
}

fn ee_emit(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let inst = get_obj(this)?;
    let name = args
        .first()
        .and_then(|v| v.to_string(ctx).ok())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();
    emit_internal(&inst, &name, &args[1..], ctx)
}

fn ee_remove_listener(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let inst = get_obj(this)?;
    let name = args
        .first()
        .and_then(|v| v.to_string(ctx).ok())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();
    let listener = args.get(1).cloned().unwrap_or(JsValue::undefined());
    remove_listener_impl(&inst, &name, &listener, ctx)?;
    Ok(this.clone())
}

fn set_methods(inst: &JsObject, methods: &[(&str, NativeFunction, usize)], ctx: &mut Context) {
    for &(n, ref f, l) in methods {
        let fv = FunctionObjectBuilder::new(ctx.realm(), f.clone())
            .name(n)
            .length(l)
            .build();
        let _ = inst.set(js_string!(n), JsValue::from(fv), false, ctx);
    }
}

fn make_ctor(name: &str, ctor_fn: NativeFunction, len: usize, ctx: &mut Context) -> JsValue {
    let ctor = FunctionObjectBuilder::new(ctx.realm(), ctor_fn)
        .name(name)
        .length(len)
        .constructor(true)
        .build();
    let ctor_val: JsValue = ctor.into();
    let ctor_obj = ctor_val.as_object().expect("ctor must be object");
    let proto = JsObject::with_object_proto(ctx.intrinsics());
    let _ = proto.set(js_string!("constructor"), ctor_val.clone(), false, ctx);
    let _ = ctor_obj.set(js_string!("prototype"), JsValue::from(proto), false, ctx);
    ctor_val
}

fn r_state(inst: &JsObject, ctx: &mut Context) -> JsResult<JsObject> {
    if let Ok(v) = inst.get(js_string!("_r"), ctx) {
        if let Some(o) = v.as_object() {
            return Ok(o);
        }
    }
    let s = JsObject::with_object_proto(ctx.intrinsics());
    let _ = s.set(
        js_string!("b"),
        JsValue::from(JsArray::new(ctx)),
        false,
        ctx,
    );
    let _ = s.set(js_string!("f"), JsValue::undefined(), false, ctx);
    let _ = s.set(js_string!("e"), JsValue::from(false), false, ctx);
    let _ = s.set(js_string!("ee"), JsValue::from(false), false, ctx);
    let _ = inst.set(js_string!("_r"), JsValue::from(s.clone()), false, ctx);
    Ok(s)
}

fn w_state(inst: &JsObject, ctx: &mut Context) -> JsResult<JsObject> {
    if let Ok(v) = inst.get(js_string!("_w"), ctx) {
        if let Some(o) = v.as_object() {
            return Ok(o);
        }
    }
    let s = JsObject::with_object_proto(ctx.intrinsics());
    let _ = s.set(
        js_string!("b"),
        JsValue::from(JsArray::new(ctx)),
        false,
        ctx,
    );
    let _ = s.set(js_string!("w"), JsValue::from(false), false, ctx);
    let _ = s.set(js_string!("e"), JsValue::from(false), false, ctx);
    let _ = inst.set(js_string!("_w"), JsValue::from(s.clone()), false, ctx);
    Ok(s)
}

fn resume(inst: &JsObject, this: &JsValue, ctx: &mut Context) -> JsResult<()> {
    let s = r_state(inst, ctx)?;
    let f = s.get(js_string!("f"), ctx).unwrap_or(JsValue::undefined());
    let ed = s
        .get(js_string!("e"), ctx)
        .ok()
        .and_then(|v| v.as_boolean())
        .unwrap_or(false);
    if f.as_boolean() != Some(true) && !ed {
        let _ = s.set(js_string!("f"), JsValue::from(true), false, ctx);
        if let Some(fn_val) = inst
            .get(js_string!("_rfn"), ctx)
            .ok()
            .and_then(|v| v.as_object().filter(|o| o.is_callable()))
        {
            let _ = fn_val.call(this, &[], ctx);
        }
    }
    Ok(())
}

fn readable_push(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let inst = get_obj(this)?;
    let chunk = args.first().cloned().unwrap_or(JsValue::undefined());
    let s = r_state(&inst, ctx)?;
    if chunk.is_null() {
        let _ = s.set(js_string!("e"), JsValue::from(true), false, ctx);
        if get_or_create_arr(&s, "b", ctx)?.length(ctx).unwrap_or(0) == 0 {
            let ic = inst.clone();
            let _ = ctx.enqueue_job(
                boa_engine::job::GenericJob::new(
                    move |jctx| {
                        if let Ok(st) = r_state(&ic, jctx) {
                            if !st
                                .get(js_string!("ee"), jctx)
                                .ok()
                                .and_then(|v| v.as_boolean())
                                .unwrap_or(false)
                            {
                                let _ = st.set(js_string!("ee"), JsValue::from(true), false, jctx);
                                let _ = emit_internal(&ic, "end", &[], jctx);
                            }
                        }
                        Ok(JsValue::undefined())
                    },
                    ctx.realm().clone(),
                )
                .into(),
            );
        }
        return Ok(JsValue::from(false));
    }
    if s.get(js_string!("f"), ctx)
        .unwrap_or(JsValue::undefined())
        .as_boolean()
        == Some(true)
    {
        let _ = emit_internal(&inst, "data", &[chunk], ctx);
    } else {
        let _ = get_or_create_arr(&s, "b", ctx)?.push(chunk, ctx);
    }
    Ok(JsValue::from(true))
}

fn readable_read(this: &JsValue, _args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let inst = get_obj(this)?;
    let s = r_state(&inst, ctx)?;
    let ed = s
        .get(js_string!("e"), ctx)
        .ok()
        .and_then(|v| v.as_boolean())
        .unwrap_or(false);
    let buf = get_or_create_arr(&s, "b", ctx)?;
    if ed && buf.length(ctx).unwrap_or(0) == 0 {
        if !s
            .get(js_string!("ee"), ctx)
            .ok()
            .and_then(|v| v.as_boolean())
            .unwrap_or(false)
        {
            let _ = s.set(js_string!("ee"), JsValue::from(true), false, ctx);
            let _ = emit_internal(&inst, "end", &[], ctx);
        }
        return Ok(JsValue::null());
    }
    if buf.length(ctx).unwrap_or(0) > 0 {
        let v = buf.get(0, ctx).unwrap_or(JsValue::undefined());
        let _ = buf.shift(ctx);
        return Ok(v);
    }
    Ok(JsValue::null())
}

fn readable_resume(this: &JsValue, _args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let _ = resume(&get_obj(this)?, this, ctx);
    Ok(JsValue::undefined())
}

fn readable_do_read(_this: &JsValue, _args: &[JsValue], _ctx: &mut Context) -> JsResult<JsValue> {
    Ok(JsValue::undefined())
}

fn readable_on(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let inst = get_obj(this)?;
    let name = args
        .first()
        .and_then(|v| v.to_string(ctx).ok())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();
    let listener = args.get(1).cloned().unwrap_or(JsValue::undefined());
    let _ = add_listener_impl(&inst, &name, &listener, ctx);
    if name == "data" {
        let _ = resume(&inst, this, ctx);
    }
    Ok(this.clone())
}

fn stream_pipe(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let src_inst = get_obj(this)?;
    let dv = args.first().cloned().unwrap_or(JsValue::undefined());

    let dv_for_closure = dv.clone();
    let on_data = unsafe {
        NativeFunction::from_closure(
            move |_: &JsValue, da: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
                let data = da.first().cloned().unwrap_or(JsValue::undefined());
                if let Some(wf) = dv_for_closure
                    .as_object()
                    .and_then(|o| o.get(js_string!("write"), ctx).ok())
                    .and_then(|f| f.as_object().filter(|o| o.is_callable()))
                {
                    let _ = wf.call(&dv_for_closure, &[data], ctx);
                }
                Ok(JsValue::undefined())
            },
        )
    };
    let _ = add_listener_impl(
        &src_inst,
        "data",
        &JsValue::from(
            FunctionObjectBuilder::new(ctx.realm(), on_data)
                .name("")
                .length(1)
                .build(),
        ),
        ctx,
    );
    let _ = resume(&src_inst, this, ctx);

    let dest = get_obj(&dv)?;
    let ic = src_inst.clone();
    let on_drain = unsafe {
        NativeFunction::from_closure(
            move |_: &JsValue, _: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
                if let Some(rfn) = ic
                    .get(js_string!("resume"), ctx)
                    .ok()
                    .and_then(|f| f.as_object().filter(|o| o.is_callable()))
                {
                    let _ = rfn.call(&JsValue::undefined(), &[], ctx);
                }
                Ok(JsValue::undefined())
            },
        )
    };
    let _ = add_listener_impl(
        &dest,
        "drain",
        &JsValue::from(
            FunctionObjectBuilder::new(ctx.realm(), on_drain)
                .name("")
                .length(0)
                .build(),
        ),
        ctx,
    );

    let dc = dest.clone();
    let on_end = unsafe {
        NativeFunction::from_closure(
            move |_: &JsValue, _: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
                if let Some(ef) = dc
                    .get(js_string!("end"), ctx)
                    .ok()
                    .and_then(|f| f.as_object().filter(|o| o.is_callable()))
                {
                    let _ = ef.call(&JsValue::undefined(), &[], ctx);
                }
                Ok(JsValue::undefined())
            },
        )
    };
    let _ = add_listener_impl(
        &src_inst,
        "end",
        &JsValue::from(
            FunctionObjectBuilder::new(ctx.realm(), on_end)
                .name("")
                .length(0)
                .build(),
        ),
        ctx,
    );

    Ok(dv)
}

fn stream_destroy(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let _ = get_obj(this)?.set(js_string!("_d"), JsValue::from(true), false, ctx);
    if let Some(err) = args.first() {
        if !err.is_null_or_undefined() {
            let _ = emit_internal(&get_obj(this)?, "error", &[err.clone()], ctx);
        }
    }
    Ok(JsValue::undefined())
}

fn writable_write(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let inst = get_obj(this)?;
    let chunk = args.first().cloned().unwrap_or(JsValue::undefined());
    let encoding = args.get(1).cloned().unwrap_or(JsValue::undefined());
    let cb = args
        .get(2)
        .or_else(|| {
            if encoding.as_object().is_some_and(|o| o.is_callable()) {
                Some(&encoding)
            } else {
                None
            }
        })
        .cloned()
        .unwrap_or(JsValue::undefined());

    let s = w_state(&inst, ctx)?;
    if s.get(js_string!("e"), ctx)
        .ok()
        .and_then(|v| v.as_boolean())
        .unwrap_or(false)
    {
        return Ok(JsValue::from(false));
    }

    let writing = s
        .get(js_string!("w"), ctx)
        .ok()
        .and_then(|v| v.as_boolean())
        .unwrap_or(false);
    let buf = get_or_create_arr(&s, "b", ctx)?;
    if writing {
        let item = JsObject::with_object_proto(ctx.intrinsics());
        let _ = item.set(js_string!("c"), chunk, false, ctx);
        let _ = item.set(js_string!("cb"), cb, false, ctx);
        let _ = buf.push(JsValue::from(item), ctx);
    } else {
        let _ = s.set(js_string!("w"), JsValue::from(true), false, ctx);
        let jv = this.clone();
        let ic = inst.clone();
        let sc = s.clone();
        let cbc = cb.clone();
        let on_done = unsafe {
            NativeFunction::from_closure(
                move |_: &JsValue, da: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
                    let _ = sc.set(js_string!("w"), JsValue::from(false), false, ctx);
                    let err = da.first().cloned().unwrap_or(JsValue::undefined());
                    if !err.is_null_or_undefined() {
                        let _ = emit_internal(&ic, "error", &[err.clone()], ctx);
                        if let Some(f) = cbc.as_object().filter(|o| o.is_callable()) {
                            let _ = f.call(&JsValue::undefined(), &[err], ctx);
                        }
                        return Ok(JsValue::undefined());
                    }
                    if let Some(f) = cbc.as_object().filter(|o| o.is_callable()) {
                        let _ = f.call(&JsValue::undefined(), &[], ctx);
                    }
                    let b = get_or_create_arr(&sc, "b", ctx)?;
                    while b.length(ctx).unwrap_or(0) > 0 {
                        let item = b.get(0, ctx).unwrap_or(JsValue::undefined());
                        let _ = b.shift(ctx);
                        if let Some(o) = item.as_object() {
                            let c = o.get(js_string!("c"), ctx).unwrap_or(JsValue::undefined());
                            let cbv = o.get(js_string!("cb"), ctx).unwrap_or(JsValue::undefined());
                            let _ = writable_write(&jv, &[c, cbv], ctx);
                        }
                    }
                    Ok(JsValue::undefined())
                },
            )
        };
        let on_done_jv = JsValue::from(
            FunctionObjectBuilder::new(ctx.realm(), on_done)
                .name("")
                .length(1)
                .build(),
        );
        if let Some(wf) = inst
            .get(js_string!("_wf"), ctx)
            .ok()
            .and_then(|f| f.as_object().filter(|o| o.is_callable()))
        {
            let _ = wf.call(this, &[chunk, encoding, on_done_jv], ctx)?;
        }
    }
    Ok(JsValue::from(true))
}

fn writable_end(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let inst = get_obj(this)?;
    let chunk = args.first().cloned().unwrap_or(JsValue::undefined());
    let is_fn = chunk.as_object().is_some_and(|o| o.is_callable());
    let cb = if is_fn {
        chunk.clone()
    } else {
        args.get(1).cloned().unwrap_or(JsValue::undefined())
    };
    let chunk = if is_fn { JsValue::undefined() } else { chunk };

    let s = w_state(&inst, ctx)?;
    let _ = s.set(js_string!("e"), JsValue::from(true), false, ctx);

    if chunk.is_undefined() || chunk.is_null() {
        if let Some(ff) = inst
            .get(js_string!("_ff"), ctx)
            .ok()
            .and_then(|f| f.as_object().filter(|o| o.is_callable()))
        {
            let ic = inst.clone();
            let cbc = cb.clone();
            let on_final = unsafe {
                NativeFunction::from_closure(
                    move |_: &JsValue, _: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
                        let _ = emit_internal(&ic, "finish", &[], ctx);
                        if let Some(f) = cbc.as_object().filter(|o| o.is_callable()) {
                            let _ = f.call(&JsValue::undefined(), &[], ctx);
                        }
                        Ok(JsValue::undefined())
                    },
                )
            };
            let ofv = JsValue::from(
                FunctionObjectBuilder::new(ctx.realm(), on_final)
                    .name("")
                    .length(0)
                    .build(),
            );
            let _ = ff.call(&JsValue::undefined(), &[ofv], ctx);
        }
    } else {
        if let Some(wf) = inst
            .get(js_string!("write"), ctx)
            .ok()
            .and_then(|f| f.as_object().filter(|o| o.is_callable()))
        {
            let _ = wf.call(this, &[chunk, JsValue::undefined(), cb], ctx);
        }
    }
    Ok(this.clone())
}

fn pipeline_impl(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    if args.len() < 2 {
        return Ok(JsValue::undefined());
    }
    if let Some(pf) = args[0]
        .as_object()
        .and_then(|o| o.get(js_string!("pipe"), ctx).ok())
        .and_then(|f| f.as_object().filter(|o| o.is_callable()))
    {
        let _ = pf.call(&args[0], &[args[1].clone()], ctx);
    }
    if let Some(cb_val) = args.get(2) {
        if let Some(cb_obj) = cb_val.as_object().filter(|o| o.is_callable()) {
            if let Some(do_) = args[1].as_object() {
                let cb2 = cb_obj.clone();
                let of = unsafe {
                    NativeFunction::from_closure(
                        move |_: &JsValue, _: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
                            let _ = cb2.call(&JsValue::undefined(), &[JsValue::null()], ctx);
                            Ok(JsValue::undefined())
                        },
                    )
                };
                let _ = add_listener_impl(
                    &do_,
                    "finish",
                    &JsValue::from(
                        FunctionObjectBuilder::new(ctx.realm(), of)
                            .name("")
                            .length(0)
                            .build(),
                    ),
                    ctx,
                );
            }
        }
    }
    Ok(JsValue::undefined())
}

fn finished_impl(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let stream = get_obj(&args.first().cloned().unwrap_or(JsValue::undefined()))?;
    let cb_val = args.get(1).cloned().unwrap_or(JsValue::undefined());
    let done = Rc::new(Cell::new(false));

    let d1 = done.clone();
    let s1 = stream.clone();
    let c1 = cb_val.clone();
    let on_end = unsafe {
        NativeFunction::from_closure(
            move |_: &JsValue, _: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
                if !d1.replace(true) {
                    let _ = remove_listener_impl(&s1, "end", &JsValue::undefined(), ctx);
                    let _ = remove_listener_impl(&s1, "finish", &JsValue::undefined(), ctx);
                    if let Some(f) = c1.as_object().filter(|o| o.is_callable()) {
                        let _ = f.call(&JsValue::undefined(), &[JsValue::null()], ctx);
                    }
                }
                Ok(JsValue::undefined())
            },
        )
    };
    let d2 = done.clone();
    let s2 = stream.clone();
    let c2 = cb_val.clone();
    let on_finish = unsafe {
        NativeFunction::from_closure(
            move |_: &JsValue, _: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
                if !d2.replace(true) {
                    let _ = remove_listener_impl(&s2, "end", &JsValue::undefined(), ctx);
                    let _ = remove_listener_impl(&s2, "finish", &JsValue::undefined(), ctx);
                    if let Some(f) = c2.as_object().filter(|o| o.is_callable()) {
                        let _ = f.call(&JsValue::undefined(), &[JsValue::null()], ctx);
                    }
                }
                Ok(JsValue::undefined())
            },
        )
    };
    let d3 = done.clone();
    let s3 = stream.clone();
    let c3 = cb_val.clone();
    let on_error = unsafe {
        NativeFunction::from_closure(
            move |_: &JsValue, ea: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
                if !d3.replace(true) {
                    let _ = remove_listener_impl(&s3, "end", &JsValue::undefined(), ctx);
                    let _ = remove_listener_impl(&s3, "finish", &JsValue::undefined(), ctx);
                    if let Some(f) = c3.as_object().filter(|o| o.is_callable()) {
                        let _ = f.call(
                            &JsValue::undefined(),
                            &[ea.first().cloned().unwrap_or(JsValue::undefined())],
                            ctx,
                        );
                    }
                }
                Ok(JsValue::undefined())
            },
        )
    };

    let _ = add_listener_impl(
        &stream,
        "end",
        &JsValue::from(
            FunctionObjectBuilder::new(ctx.realm(), on_end)
                .name("")
                .length(0)
                .build(),
        ),
        ctx,
    );
    let _ = add_listener_impl(
        &stream,
        "finish",
        &JsValue::from(
            FunctionObjectBuilder::new(ctx.realm(), on_finish)
                .name("")
                .length(0)
                .build(),
        ),
        ctx,
    );
    let _ = add_listener_impl(
        &stream,
        "error",
        &JsValue::from(
            FunctionObjectBuilder::new(ctx.realm(), on_error)
                .name("")
                .length(1)
                .build(),
        ),
        ctx,
    );

    let sc = stream.clone();
    let cleanup = unsafe {
        NativeFunction::from_closure(
            move |_: &JsValue, _: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
                let _ = remove_listener_impl(&sc, "end", &JsValue::undefined(), ctx);
                let _ = remove_listener_impl(&sc, "finish", &JsValue::undefined(), ctx);
                let _ = remove_listener_impl(&sc, "error", &JsValue::undefined(), ctx);
                Ok(JsValue::undefined())
            },
        )
    };
    Ok(JsValue::from(
        FunctionObjectBuilder::new(ctx.realm(), cleanup)
            .name("")
            .length(0)
            .build(),
    ))
}

pub fn create_node_stream_module(context: &mut Context) -> Result<Module, String> {
    let export_names: &[JsString] = &[
        js_string!("Stream"),
        js_string!("Readable"),
        js_string!("Writable"),
        js_string!("Duplex"),
        js_string!("Transform"),
        js_string!("PassThrough"),
        js_string!("pipeline"),
        js_string!("finished"),
        js_string!("default"),
    ];
    let module = Module::synthetic(
        export_names,
        unsafe {
            SyntheticModuleInitializer::from_closure(
                move |m: &boa_engine::module::SyntheticModule, ctx: &mut Context| {
                    let stream = make_ctor(
                        "Stream",
                        make_native(
                            |_: &JsValue, _: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
                                let inst = JsObject::with_object_proto(ctx.intrinsics());
                                let _ = ee_init(&inst, ctx);
                                let _ =
                                    inst.set(js_string!("_d"), JsValue::from(false), false, ctx);
                                set_methods(
                                    &inst,
                                    &[
                                        ("pipe", make_native(stream_pipe), 2),
                                        ("destroy", make_native(stream_destroy), 1),
                                        ("on", make_native(ee_on), 2),
                                        ("emit", make_native(ee_emit), 1),
                                        ("removeListener", make_native(ee_remove_listener), 2),
                                    ],
                                    ctx,
                                );
                                Ok(JsValue::from(inst))
                            },
                        ),
                        0,
                        ctx,
                    );

                    let readable = make_ctor(
                        "Readable",
                        make_native(
                            |_: &JsValue,
                             args: &[JsValue],
                             ctx: &mut Context|
                             -> JsResult<JsValue> {
                                let opts = args.first().cloned().unwrap_or(JsValue::undefined());
                                let inst = JsObject::with_object_proto(ctx.intrinsics());
                                let _ = ee_init(&inst, ctx);
                                let _ = r_state(&inst, ctx);
                                let ur = opts
                                    .as_object()
                                    .and_then(|o| o.get(js_string!("read"), ctx).ok())
                                    .filter(|v| !v.is_undefined());
                                let _ = inst.set(
                                    js_string!("_rfn"),
                                    ur.unwrap_or_else(|| {
                                        build_fn(make_native(readable_do_read), "_rfn", 1, ctx)
                                    }),
                                    false,
                                    ctx,
                                );
                                let _ =
                                    inst.set(js_string!("_d"), JsValue::from(false), false, ctx);
                                set_methods(
                                    &inst,
                                    &[
                                        ("push", make_native(readable_push), 2),
                                        ("read", make_native(readable_read), 1),
                                        ("resume", make_native(readable_resume), 0),
                                        ("pipe", make_native(stream_pipe), 2),
                                        ("destroy", make_native(stream_destroy), 1),
                                        ("on", make_native(readable_on), 2),
                                        ("addListener", make_native(readable_on), 2),
                                        ("emit", make_native(ee_emit), 1),
                                        ("removeListener", make_native(ee_remove_listener), 2),
                                    ],
                                    ctx,
                                );
                                Ok(JsValue::from(inst))
                            },
                        ),
                        1,
                        ctx,
                    );

                    let writable = make_ctor(
                        "Writable",
                        make_native(
                            |_: &JsValue,
                             args: &[JsValue],
                             ctx: &mut Context|
                             -> JsResult<JsValue> {
                                let opts = args.first().cloned().unwrap_or(JsValue::undefined());
                                let inst = JsObject::with_object_proto(ctx.intrinsics());
                                let _ = ee_init(&inst, ctx);
                                let _ = w_state(&inst, ctx);
                                if let Some(o) = opts.as_object() {
                                    if let Ok(fn_val) = o.get(js_string!("write"), ctx) {
                                        if !fn_val.is_undefined() {
                                            let _ = inst.set(js_string!("_wf"), fn_val, false, ctx);
                                        }
                                    }
                                }
                                inst.set(js_string!("_wf"), build_fn(make_native(|_: &JsValue, a: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
                            if let Some(cb) = a.get(2).or_else(|| a.get(1)).and_then(|v| v.as_object()).filter(|o| o.is_callable()) {
                                let _ = cb.call(&JsValue::undefined(), &[], ctx);
                            }
                            Ok(JsValue::undefined())
                        }), "_wf", 3, ctx), false, ctx)?;
                                let _ =
                                    inst.set(js_string!("_d"), JsValue::from(false), false, ctx);
                                set_methods(
                                    &inst,
                                    &[
                                        ("write", make_native(writable_write), 3),
                                        ("end", make_native(writable_end), 3),
                                        ("destroy", make_native(stream_destroy), 1),
                                        ("on", make_native(ee_on), 2),
                                        ("emit", make_native(ee_emit), 1),
                                        ("removeListener", make_native(ee_remove_listener), 2),
                                    ],
                                    ctx,
                                );
                                Ok(JsValue::from(inst))
                            },
                        ),
                        1,
                        ctx,
                    );

                    let duplex = make_ctor(
                        "Duplex",
                        make_native(
                            |_: &JsValue,
                             args: &[JsValue],
                             ctx: &mut Context|
                             -> JsResult<JsValue> {
                                let opts = args.first().cloned().unwrap_or(JsValue::undefined());
                                let inst = JsObject::with_object_proto(ctx.intrinsics());
                                let _ = ee_init(&inst, ctx);
                                let _ = r_state(&inst, ctx);
                                let _ = w_state(&inst, ctx);
                                let _ =
                                    inst.set(js_string!("_d"), JsValue::from(false), false, ctx);
                                if let Some(o) = opts.as_object() {
                                    if let Ok(fn_val) = o.get(js_string!("read"), ctx) {
                                        if !fn_val.is_undefined() {
                                            let _ =
                                                inst.set(js_string!("_rfn"), fn_val, false, ctx);
                                        }
                                    }
                                    if let Ok(fn_val) = o.get(js_string!("write"), ctx) {
                                        if !fn_val.is_undefined() {
                                            let _ = inst.set(js_string!("_wf"), fn_val, false, ctx);
                                        }
                                    }
                                }
                                set_methods(
                                    &inst,
                                    &[
                                        ("push", make_native(readable_push), 2),
                                        ("read", make_native(readable_read), 1),
                                        ("resume", make_native(readable_resume), 0),
                                        ("pipe", make_native(stream_pipe), 2),
                                        ("destroy", make_native(stream_destroy), 1),
                                        ("write", make_native(writable_write), 3),
                                        ("end", make_native(writable_end), 3),
                                        ("on", make_native(readable_on), 2),
                                        ("addListener", make_native(readable_on), 2),
                                        ("emit", make_native(ee_emit), 1),
                                        ("removeListener", make_native(ee_remove_listener), 2),
                                    ],
                                    ctx,
                                );
                                Ok(JsValue::from(inst))
                            },
                        ),
                        1,
                        ctx,
                    );

                    let transform = make_ctor(
                        "Transform",
                        make_native(
                            |_: &JsValue,
                             args: &[JsValue],
                             ctx: &mut Context|
                             -> JsResult<JsValue> {
                                let opts = args.first().cloned().unwrap_or(JsValue::undefined());
                                let inst = JsObject::with_object_proto(ctx.intrinsics());
                                let _ = ee_init(&inst, ctx);
                                let _ = r_state(&inst, ctx);
                                let _ = w_state(&inst, ctx);
                                let _ =
                                    inst.set(js_string!("_d"), JsValue::from(false), false, ctx);
                                if let Some(o) = opts.as_object() {
                                    if let Ok(fn_val) = o.get(js_string!("read"), ctx) {
                                        if !fn_val.is_undefined() {
                                            let _ =
                                                inst.set(js_string!("_rfn"), fn_val, false, ctx);
                                        }
                                    }
                                }
                                let _wf = make_native(
                                    |_t: &JsValue,
                                     _a: &[JsValue],
                                     _c: &mut Context|
                                     -> JsResult<JsValue> {
                                        let chunk =
                                            _a.first().cloned().unwrap_or(JsValue::undefined());
                                        if !chunk.is_undefined() && !chunk.is_null() {
                                            let _ = readable_push(_t, &[chunk], _c);
                                        }
                                        if let Some(cb) = _a
                                            .get(2)
                                            .and_then(|v| v.as_object())
                                            .filter(|o| o.is_callable())
                                        {
                                            let _ = cb.call(&JsValue::undefined(), &[], _c);
                                        }
                                        Ok(JsValue::undefined())
                                    },
                                );
                                let _ = inst.set(
                                    js_string!("_wf"),
                                    JsValue::from(
                                        FunctionObjectBuilder::new(ctx.realm(), _wf)
                                            .name("_wf")
                                            .length(3)
                                            .build(),
                                    ),
                                    false,
                                    ctx,
                                );
                                set_methods(
                                    &inst,
                                    &[
                                        ("push", make_native(readable_push), 2),
                                        ("read", make_native(readable_read), 1),
                                        ("resume", make_native(readable_resume), 0),
                                        ("pipe", make_native(stream_pipe), 2),
                                        ("destroy", make_native(stream_destroy), 1),
                                        ("write", make_native(writable_write), 3),
                                        ("end", make_native(writable_end), 3),
                                        ("on", make_native(readable_on), 2),
                                        ("addListener", make_native(readable_on), 2),
                                        ("emit", make_native(ee_emit), 1),
                                        ("removeListener", make_native(ee_remove_listener), 2),
                                    ],
                                    ctx,
                                );
                                Ok(JsValue::from(inst))
                            },
                        ),
                        1,
                        ctx,
                    );

                    let passthrough = make_ctor(
                        "PassThrough",
                        make_native(
                            |_: &JsValue,
                             args: &[JsValue],
                             ctx: &mut Context|
                             -> JsResult<JsValue> {
                                let opts = args.first().cloned().unwrap_or(JsValue::undefined());
                                let inst = JsObject::with_object_proto(ctx.intrinsics());
                                let _ = ee_init(&inst, ctx);
                                let _ = r_state(&inst, ctx);
                                let _ = w_state(&inst, ctx);
                                let _ =
                                    inst.set(js_string!("_d"), JsValue::from(false), false, ctx);
                                if let Some(o) = opts.as_object() {
                                    if let Ok(fn_val) = o.get(js_string!("read"), ctx) {
                                        if !fn_val.is_undefined() {
                                            let _ =
                                                inst.set(js_string!("_rfn"), fn_val, false, ctx);
                                        }
                                    }
                                }
                                let _wf = make_native(
                                    |_t: &JsValue,
                                     _a: &[JsValue],
                                     _c: &mut Context|
                                     -> JsResult<JsValue> {
                                        let chunk =
                                            _a.first().cloned().unwrap_or(JsValue::undefined());
                                        if !chunk.is_undefined() && !chunk.is_null() {
                                            let _ = readable_push(_t, &[chunk], _c);
                                        }
                                        if let Some(cb) = _a
                                            .get(2)
                                            .and_then(|v| v.as_object())
                                            .filter(|o| o.is_callable())
                                        {
                                            let _ = cb.call(&JsValue::undefined(), &[], _c);
                                        }
                                        Ok(JsValue::undefined())
                                    },
                                );
                                let _ = inst.set(
                                    js_string!("_wf"),
                                    JsValue::from(
                                        FunctionObjectBuilder::new(ctx.realm(), _wf)
                                            .name("_wf")
                                            .length(3)
                                            .build(),
                                    ),
                                    false,
                                    ctx,
                                );
                                set_methods(
                                    &inst,
                                    &[
                                        ("push", make_native(readable_push), 2),
                                        ("read", make_native(readable_read), 1),
                                        ("resume", make_native(readable_resume), 0),
                                        ("pipe", make_native(stream_pipe), 2),
                                        ("destroy", make_native(stream_destroy), 1),
                                        ("write", make_native(writable_write), 3),
                                        ("end", make_native(writable_end), 3),
                                        ("on", make_native(readable_on), 2),
                                        ("addListener", make_native(readable_on), 2),
                                        ("emit", make_native(ee_emit), 1),
                                        ("removeListener", make_native(ee_remove_listener), 2),
                                    ],
                                    ctx,
                                );
                                Ok(JsValue::from(inst))
                            },
                        ),
                        1,
                        ctx,
                    );

                    let pipeline = build_fn(make_native(pipeline_impl), "pipeline", 3, ctx);
                    let finished_fn = build_fn(make_native(finished_impl), "finished", 2, ctx);

                    for &(n, ref v) in &[
                        ("Stream", &stream),
                        ("Readable", &readable),
                        ("Writable", &writable),
                        ("Duplex", &duplex),
                        ("Transform", &transform),
                        ("PassThrough", &passthrough),
                        ("pipeline", &pipeline),
                        ("finished", &finished_fn),
                    ] {
                        let _ = m.set_export(&js_string!(n), (*v).clone());
                    }
                    let def = JsObject::with_object_proto(ctx.intrinsics());
                    for &(n, ref v) in &[
                        ("Stream", &stream),
                        ("Readable", &readable),
                        ("Writable", &writable),
                        ("Duplex", &duplex),
                        ("Transform", &transform),
                        ("PassThrough", &passthrough),
                        ("pipeline", &pipeline),
                        ("finished", &finished_fn),
                    ] {
                        let _ = def.set(js_string!(n), (*v).clone(), false, ctx);
                    }
                    let _ = m.set_export(&js_string!("default"), JsValue::from(def));
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
