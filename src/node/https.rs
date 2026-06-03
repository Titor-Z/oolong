use boa_engine::module::SyntheticModuleInitializer;
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

fn create_server_impl(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    crate::node::http::server::create_server(
        args.first().cloned().unwrap_or(JsValue::undefined()),
        ctx,
    )
}

fn request_impl(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let opts = args.first().cloned().unwrap_or(JsValue::undefined());
    let callback = args.get(1).cloned().unwrap_or(JsValue::undefined());

    let url = if let Some(s) = opts.as_string() {
        s.to_std_string_escaped()
    } else if let Some(obj) = opts.as_object() {
        let hostname = obj.get(js_string!("hostname"), ctx).ok()
            .and_then(|v| v.to_string(ctx).ok())
            .map(|s| s.to_std_string_escaped())
            .unwrap_or_else(|| "localhost".to_string());
        let port = obj.get(js_string!("port"), ctx).ok()
            .and_then(|v| v.as_number()).unwrap_or(443.0) as u16;
        let path = obj.get(js_string!("path"), ctx).ok()
            .and_then(|v| v.to_string(ctx).ok())
            .map(|s| s.to_std_string_escaped())
            .unwrap_or_else(|| "/".to_string());
        format!("https://{hostname}:{port}{path}")
    } else {
        return Err(JsNativeError::typ().with_message("https.request: invalid URL").into());
    };

    let client = reqwest::blocking::Client::builder()
        .danger_accept_invalid_certs(true).build()
        .map_err(|e| JsNativeError::typ().with_message(format!("https.request: {e}")))?;

    match client.get(&url).send() {
        Ok(resp) => {
            let res = crate::node::http::common::create_response_from_reqwest(resp, ctx)?;
            if let Some(cb) = callback.as_object().filter(|o| o.is_callable()) {
                let _ = cb.call(&JsValue::undefined(), &[JsValue::from(res.clone())], ctx);
            }
            if let Ok(body_val) = res.get(js_string!("__body"), ctx) {
                if let Some(arr_obj) = body_val.as_object() {
                    if let Ok(arr) = boa_engine::object::builtins::JsArray::from_object(arr_obj.clone()) {
                        for i in 0..arr.length(ctx).unwrap_or(0) {
                            if let Ok(item) = arr.get(i, ctx) {
                                crate::node::http::common::emit(&res, "data", &[item], ctx);
                            }
                        }
                    }
                }
            }
            crate::node::http::common::emit(&res, "end", &[], ctx);
            Ok(JsValue::from(res))
        }
        Err(e) => Err(JsNativeError::typ()
            .with_message(format!("https.get failed: {e}")).into()),
    }
}

fn get_impl(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    request_impl(_this, args, ctx)
}

pub fn create_node_https_module(context: &mut Context) -> Result<Module, String> {
    let export_names: &[JsString] = &[
        js_string!("createServer"),
        js_string!("request"),
        js_string!("get"),
        js_string!("globalAgent"),
        js_string!("default"),
    ];

    let module = Module::synthetic(
        export_names,
        unsafe {
            SyntheticModuleInitializer::from_closure(
                move |m: &boa_engine::module::SyntheticModule, ctx: &mut Context| {
                    let create_server_fn = make_fn(create_server_impl, "createServer", 1, ctx);
                    let request_fn = make_fn(request_impl, "request", 2, ctx);
                    let get_fn = make_fn(get_impl, "get", 2, ctx);

                    let global_agent = JsObject::with_object_proto(ctx.intrinsics());
                    let _ = global_agent.set(js_string!("maxSockets"), JsValue::from(10f64), false, ctx);

                    m.set_export(&js_string!("createServer"), create_server_fn.clone())?;
                    m.set_export(&js_string!("request"), request_fn.clone())?;
                    m.set_export(&js_string!("get"), get_fn.clone())?;
                    m.set_export(&js_string!("globalAgent"), JsValue::from(global_agent.clone()))?;

                    let default_obj = JsObject::with_object_proto(ctx.intrinsics());
                    default_obj.set(js_string!("createServer"), create_server_fn, false, ctx)?;
                    default_obj.set(js_string!("request"), request_fn, false, ctx)?;
                    default_obj.set(js_string!("get"), get_fn, false, ctx)?;
                    default_obj.set(js_string!("globalAgent"), JsValue::from(global_agent), false, ctx)?;
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
