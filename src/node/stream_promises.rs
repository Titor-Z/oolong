use boa_engine::object::builtins::JsPromise;
use boa_engine::object::FunctionObjectBuilder;
use boa_engine::{
    Context, JsError, JsNativeError, JsResult, JsValue, Module, NativeFunction, js_string,
};

fn js_err(msg: &str) -> JsError {
    JsNativeError::typ().with_message(msg.to_string()).into()
}

fn build_fn<F>(f: F, name: &str, len: usize, ctx: &mut Context) -> JsValue
where
    F: Fn(&JsValue, &[JsValue], &mut Context) -> JsResult<JsValue> + 'static,
{
    FunctionObjectBuilder::new(ctx.realm(), unsafe { NativeFunction::from_closure(f) })
        .name(name)
        .length(len)
        .build()
        .into()
}

fn make_cb(
    resolve: boa_engine::object::builtins::JsFunction,
    reject: boa_engine::object::builtins::JsFunction,
    ctx: &mut Context,
) -> JsValue {
    build_fn(
        move |_: &JsValue,
              cb_args: &[JsValue],
              ctx2: &mut Context|
              -> JsResult<JsValue> {
            if let Some(err) = cb_args.first()
                && !err.is_null() && !err.is_undefined()
            {
                let _ = reject.call(&JsValue::undefined(), std::slice::from_ref(err), ctx2);
                return Ok(JsValue::undefined());
            }
            let _ = resolve.call(&JsValue::undefined(), &[], ctx2);
            Ok(JsValue::undefined())
        },
        "",
        1,
        ctx,
    )
}

pub fn create_node_stream_promises_module(context: &mut Context) -> Result<Module, String> {
    let export_names = &[js_string!("pipeline"), js_string!("finished"), js_string!("default")];

    let module = Module::synthetic(
        export_names,
        unsafe {
            boa_engine::module::SyntheticModuleInitializer::from_closure(
                move |m: &boa_engine::module::SyntheticModule, ctx: &mut Context| {
                    let pipeline = build_fn(
                        |this: &JsValue, args: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
                            let (promise, resolvers) = JsPromise::new_pending(ctx);
                            let cb = make_cb(resolvers.resolve, resolvers.reject, ctx);

                            let mut new_args = args.to_vec();
                            new_args.push(cb);
                            crate::node::stream::pipeline_impl(this, &new_args, ctx)?;

                            Ok(promise.into())
                        },
                        "pipeline",
                        3,
                        ctx,
                    );

                    let finished = build_fn(
                        |this: &JsValue, args: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
                            if args.is_empty() {
                                return Err(js_err("stream is required"));
                            }
                            let (promise, resolvers) = JsPromise::new_pending(ctx);
                            let cb = make_cb(resolvers.resolve, resolvers.reject, ctx);

                            let mut new_args = args.to_vec();
                            new_args.push(cb);
                            crate::node::stream::finished_impl(this, &new_args, ctx)?;

                            Ok(promise.into())
                        },
                        "finished",
                        2,
                        ctx,
                    );

                    let def = boa_engine::JsObject::with_object_proto(ctx.intrinsics());
                    let _ = def.set(js_string!("pipeline"), pipeline.clone(), false, ctx);
                    let _ = def.set(js_string!("finished"), finished.clone(), false, ctx);

                    let _ = m.set_export(&js_string!("pipeline"), pipeline);
                    let _ = m.set_export(&js_string!("finished"), finished);
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
