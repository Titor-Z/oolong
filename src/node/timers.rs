use boa_engine::module::SyntheticModuleInitializer;
use boa_engine::object::FunctionObjectBuilder;
use boa_engine::object::builtins::JsPromise;
use boa_engine::{
    Context, JsNativeError, JsObject, JsResult, JsString, JsValue, Module, NativeFunction,
    js_string,
};

fn get_global_fn(ctx: &mut Context, name: &str) -> JsResult<JsValue> {
    ctx.global_object().get(js_string!(name), ctx).map_err(|_| {
        JsNativeError::typ()
            .with_message(format!("no global {name}"))
            .into()
    })
}

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

fn make_promise_settimeout(ctx: &mut Context) -> JsValue {
    make_fn(
        move |_: &JsValue, args: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
            let delay = args.first().cloned().unwrap_or_default();
            let val = args.get(1).cloned().unwrap_or_default();
            let (promise, resolvers) = JsPromise::new_pending(ctx);
            let resolve = resolvers.resolve;
            let global_timeout = get_global_fn(ctx, "setTimeout")?;
            let timeout_obj = global_timeout
                .as_object()
                .ok_or_else(|| JsNativeError::typ().with_message("setTimeout not callable"))?;
            let handler = make_fn(
                move |_: &JsValue, _: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
                    resolve.call(&JsValue::undefined(), &[val.clone()], ctx)
                },
                "",
                0,
                ctx,
            );
            timeout_obj.call(&JsValue::undefined(), &[handler, delay], ctx)?;
            Ok(JsValue::from(promise))
        },
        "setTimeout",
        2,
        ctx,
    )
}

fn make_promise_setimmediate(ctx: &mut Context) -> JsValue {
    make_fn(
        move |_: &JsValue, args: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
            let val = args.first().cloned().unwrap_or_default();
            let (promise, resolvers) = JsPromise::new_pending(ctx);
            let resolve = resolvers.resolve;
            let global_immediate = get_global_fn(ctx, "setImmediate")?;
            let immediate_obj = global_immediate
                .as_object()
                .ok_or_else(|| JsNativeError::typ().with_message("setImmediate not callable"))?;
            let handler = make_fn(
                move |_: &JsValue, _: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
                    resolve.call(&JsValue::undefined(), &[val.clone()], ctx)
                },
                "",
                0,
                ctx,
            );
            immediate_obj.call(&JsValue::undefined(), &[handler], ctx)?;
            Ok(JsValue::from(promise))
        },
        "setImmediate",
        1,
        ctx,
    )
}

pub fn create_node_timers_module(context: &mut Context) -> Result<Module, String> {
    let export_names: &[JsString] = &[
        js_string!("setTimeout"),
        js_string!("clearTimeout"),
        js_string!("setInterval"),
        js_string!("clearInterval"),
        js_string!("setImmediate"),
        js_string!("clearImmediate"),
        js_string!("default"),
    ];

    let module = Module::synthetic(
        export_names,
        SyntheticModuleInitializer::from_copy_closure(
            move |m: &boa_engine::module::SyntheticModule, ctx: &mut Context| {
                let timeout = get_global_fn(ctx, "setTimeout")?;
                let clear_timeout = get_global_fn(ctx, "clearTimeout")?;
                let interval = get_global_fn(ctx, "setInterval")?;
                let clear_interval = get_global_fn(ctx, "clearInterval")?;
                let immediate = get_global_fn(ctx, "setImmediate")?;
                let clear_immediate = get_global_fn(ctx, "clearImmediate")?;

                m.set_export(&js_string!("setTimeout"), timeout.clone())?;
                m.set_export(&js_string!("clearTimeout"), clear_timeout.clone())?;
                m.set_export(&js_string!("setInterval"), interval.clone())?;
                m.set_export(&js_string!("clearInterval"), clear_interval.clone())?;
                m.set_export(&js_string!("setImmediate"), immediate.clone())?;
                m.set_export(&js_string!("clearImmediate"), clear_immediate.clone())?;

                let default_obj = JsObject::with_object_proto(ctx.intrinsics());
                default_obj.set(js_string!("setTimeout"), timeout, false, ctx)?;
                default_obj.set(js_string!("clearTimeout"), clear_timeout, false, ctx)?;
                default_obj.set(js_string!("setInterval"), interval, false, ctx)?;
                default_obj.set(js_string!("clearInterval"), clear_interval, false, ctx)?;
                default_obj.set(js_string!("setImmediate"), immediate, false, ctx)?;
                default_obj.set(js_string!("clearImmediate"), clear_immediate, false, ctx)?;

                let promises_obj = JsObject::with_object_proto(ctx.intrinsics());
                promises_obj.set(
                    js_string!("setTimeout"),
                    make_promise_settimeout(ctx),
                    false,
                    ctx,
                )?;
                promises_obj.set(
                    js_string!("setImmediate"),
                    make_promise_setimmediate(ctx),
                    false,
                    ctx,
                )?;
                default_obj.set(
                    js_string!("promises"),
                    JsValue::from(promises_obj),
                    false,
                    ctx,
                )?;

                m.set_export(&js_string!("default"), JsValue::from(default_obj))?;

                Ok(())
            },
        ),
        None,
        None,
        context,
    );

    Ok(module)
}
