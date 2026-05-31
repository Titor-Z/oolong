use boa_engine::object::builtins::JsArray;
use boa_engine::{js_string, Context, JsObject, JsResult, JsValue};

use super::common::{build_fn, make_native, add_listener};

pub fn create_incoming_message(
    method: &str,
    url: &str,
    http_version: &str,
    headers: &[(String, String)],
    _body: Vec<u8>,
    remote_addr: Option<&str>,
    remote_port: u16,
    ctx: &mut Context,
) -> JsObject {
    let req = JsObject::with_object_proto(ctx.intrinsics());
    let _ = req.set(js_string!("method"), js_string!(method), false, ctx);
    let _ = req.set(js_string!("url"), js_string!(url), false, ctx);
    let _ = req.set(
        js_string!("httpVersion"),
        js_string!(http_version),
        false,
        ctx,
    );

    if let Some(addr) = remote_addr {
        let _ = req.set(js_string!("remoteAddress"), js_string!(addr), false, ctx);
        let _ = req.set(
            js_string!("remotePort"),
            JsValue::from(remote_port as f64),
            false,
            ctx,
        );
    }

    let headers_obj = JsObject::with_object_proto(ctx.intrinsics());
    let raw_arr = JsArray::new(ctx);
    for (k, v) in headers {
        let _ = headers_obj.set(
            js_string!(k.to_lowercase()),
            js_string!(v.as_str()),
            false,
            ctx,
        );
        let _ = raw_arr.push(js_string!(k.as_str()), ctx);
        let _ = raw_arr.push(js_string!(v.as_str()), ctx);
    }
    let _ = req.set(
        js_string!("headers"),
        JsValue::from(headers_obj),
        false,
        ctx,
    );
    let _ = req.set(js_string!("rawHeaders"), JsValue::from(raw_arr), false, ctx);

    let socket = JsObject::with_object_proto(ctx.intrinsics());
    if let Some(addr) = remote_addr {
        let _ = socket.set(js_string!("remoteAddress"), js_string!(addr), false, ctx);
    }
    let _ = socket.set(
        js_string!("remotePort"),
        JsValue::from(remote_port as f64),
        false,
        ctx,
    );
    let _ = req.set(js_string!("socket"), JsValue::from(socket), false, ctx);

    let _ = req.set(
        js_string!("_events"),
        JsValue::from(JsObject::with_object_proto(ctx.intrinsics())),
        false,
        ctx,
    );

    let on_fn = build_fn(
        make_native(
            move |this: &JsValue, args: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
                if let Some(inst) = this.as_object() {
                    let name = args
                        .first()
                        .and_then(|v| v.to_string(ctx).ok())
                        .map(|s| s.to_std_string_escaped())
                        .unwrap_or_default();
                    if let Some(listener) = args.get(1) {
                        let _ = add_listener(&inst, &name, listener, ctx);
                    }
                }
                Ok(this.clone())
            },
        ),
        "on",
        2,
        ctx,
    );
    let _ = req.set(js_string!("on"), on_fn, false, ctx);

    req
}
