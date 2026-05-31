use std::io::Write;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};

use boa_engine::object::builtins::JsArray;
use boa_engine::{js_string, Context, JsObject, JsResult, JsValue};

use super::common::{build_fn, get_obj, make_native, add_listener, build_response_string, collect_res_headers, emit};

pub fn create_outgoing_message(stream: Arc<Mutex<TcpStream>>, ctx: &mut Context) -> JsObject {
    let res = JsObject::with_object_proto(ctx.intrinsics());
    let _ = res.set(js_string!("statusCode"), JsValue::from(200), false, ctx);
    let _ = res.set(js_string!("statusMessage"), js_string!("OK"), false, ctx);
    let _ = res.set(
        js_string!("__headers"),
        JsValue::from(JsObject::with_object_proto(ctx.intrinsics())),
        false,
        ctx,
    );
    let _ = res.set(
        js_string!("__buffer"),
        JsValue::from(JsArray::new(ctx)),
        false,
        ctx,
    );
    let _ = res.set(js_string!("__ended"), JsValue::from(false), false, ctx);
    let _ = res.set(js_string!("__sent"), JsValue::from(false), false, ctx);
    let _ = res.set(
        js_string!("_events"),
        JsValue::from(JsObject::with_object_proto(ctx.intrinsics())),
        false,
        ctx,
    );

    // setHeader(name, value)
    let _ = res.set(
        js_string!("setHeader"),
        build_fn(
            make_native(
                |this: &JsValue, args: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
                    let inst = get_obj(this)?;
                    let name = args
                        .first()
                        .and_then(|v| v.to_string(ctx).ok())
                        .map(|s| s.to_std_string_escaped())
                        .unwrap_or_default();
                    let value = args.get(1).cloned().unwrap_or(JsValue::undefined());
                    if let Some(headers) = inst
                        .get(js_string!("__headers"), ctx)
                        .ok()
                        .and_then(|v| v.as_object())
                        .map(|o| o.clone())
                    {
                        let _ = headers.set(js_string!(name.to_lowercase()), value, false, ctx);
                    }
                    Ok(this.clone())
                },
            ),
            "setHeader",
            2,
            ctx,
        ),
        false,
        ctx,
    );

    // getHeader(name)
    let _ = res.set(
        js_string!("getHeader"),
        build_fn(
            make_native(
                |this: &JsValue, args: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
                    let inst = get_obj(this)?;
                    let name = args
                        .first()
                        .and_then(|v| v.to_string(ctx).ok())
                        .map(|s| s.to_std_string_escaped())
                        .unwrap_or_default();
                    if let Ok(headers) = inst.get(js_string!("__headers"), ctx) {
                        if let Some(h) = headers.as_object() {
                            if let Ok(val) = h.get(js_string!(name.to_lowercase()), ctx) {
                                if !val.is_undefined() {
                                    return Ok(val);
                                }
                            }
                        }
                    }
                    Ok(JsValue::undefined())
                },
            ),
            "getHeader",
            1,
            ctx,
        ),
        false,
        ctx,
    );

    // removeHeader(name)
    let _ = res.set(
        js_string!("removeHeader"),
        build_fn(
            make_native(
                |this: &JsValue, args: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
                    let inst = get_obj(this)?;
                    let name = args
                        .first()
                        .and_then(|v| v.to_string(ctx).ok())
                        .map(|s| s.to_std_string_escaped())
                        .unwrap_or_default();
                    if let Some(headers) = inst
                        .get(js_string!("__headers"), ctx)
                        .ok()
                        .and_then(|v| v.as_object())
                        .map(|o| o.clone())
                    {
                        let _ =
                            headers.delete_property_or_throw(js_string!(name.to_lowercase()), ctx);
                    }
                    Ok(this.clone())
                },
            ),
            "removeHeader",
            1,
            ctx,
        ),
        false,
        ctx,
    );

    // writeHead(statusCode, statusMessage, headers)
    let _ = res.set(
        js_string!("writeHead"),
        build_fn(
            make_native(
                |this: &JsValue, args: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
                    let inst = get_obj(this)?;
                    if let Some(sc) = args.first().and_then(|v| v.as_number()) {
                        let _ = inst.set(js_string!("statusCode"), JsValue::from(sc), false, ctx);
                    }
                    if let Some(msg) = args.get(1).and_then(|v| v.to_string(ctx).ok()) {
                        let _ = inst.set(js_string!("statusMessage"), msg, false, ctx);
                    }
                    for arg_idx in [1usize, 2] {
                        if let Some(obj) = args.get(arg_idx).and_then(|v| v.as_object()) {
                            if let Some(h) = inst
                                .get(js_string!("__headers"), ctx)
                                .ok()
                                .and_then(|v| v.as_object())
                                .map(|o| o.clone())
                            {
                                for key in obj.own_property_keys(ctx).unwrap_or_default() {
                                    if let Ok(val) = obj.get(key.clone(), ctx) {
                                        let key_str = key.to_string();
                                        let _ = h.set(
                                            js_string!(key_str.to_lowercase()),
                                            val,
                                            false,
                                            ctx,
                                        );
                                    }
                                }
                            }
                        }
                    }
                    Ok(this.clone())
                },
            ),
            "writeHead",
            3,
            ctx,
        ),
        false,
        ctx,
    );

    // on(event, cb)
    let _ = res.set(
        js_string!("on"),
        build_fn(
            make_native(
                |this: &JsValue, args: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
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
        ),
        false,
        ctx,
    );

    // write(chunk, encoding, cb)
    let _ = res.set(
        js_string!("write"),
        build_fn(
            make_native(
                |this: &JsValue, args: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
                    let inst = get_obj(this)?;
                    if let Some(data) = args.first() {
                        let chunk_str = if let Some(s) = data.as_string() {
                            s.to_std_string_escaped()
                        } else {
                            data.to_string(ctx)
                                .ok()
                                .map(|s| s.to_std_string_escaped())
                                .unwrap_or_default()
                        };
                        if let Some(buf) = inst
                            .get(js_string!("__buffer"), ctx)
                            .ok()
                            .and_then(|v| v.as_object())
                            .and_then(|o| JsArray::from_object(o.clone()).ok())
                        {
                            let _ = buf.push(JsValue::from(js_string!(chunk_str)), ctx);
                        }
                    }
                    if let Some(cb) = args
                        .get(1)
                        .or_else(|| args.get(2))
                        .and_then(|v| v.as_object())
                        .filter(|o| o.is_callable())
                    {
                        let _ = cb.call(&JsValue::undefined(), &[], ctx);
                    }
                    Ok(JsValue::from(true))
                },
            ),
            "write",
            2,
            ctx,
        ),
        false,
        ctx,
    );

    // end(data, encoding, cb)
    let res_for_end = res.clone();
    let stream_for_end = stream.clone();
    let _ = res.set(
        js_string!("end"),
        build_fn(
            make_native(
                move |_: &JsValue, end_args: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
                    let mut body_parts: Vec<Vec<u8>> = Vec::new();
                    if let Ok(buf_val) = res_for_end.get(js_string!("__buffer"), ctx) {
                        if let Some(buf_obj) = buf_val.as_object() {
                            if let Ok(buf_arr) = JsArray::from_object(buf_obj.clone()) {
                                for i in 0..buf_arr.length(ctx).unwrap_or(0) {
                                    if let Ok(item) = buf_arr.get(i, ctx) {
                                        if let Some(s) = item.as_string() {
                                            body_parts.push(s.to_std_string_escaped().into_bytes());
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if let Some(data) = end_args.first() {
                        if let Some(s) = data.as_string() {
                            body_parts.push(s.to_std_string_escaped().into_bytes());
                        } else if let Ok(s) = data.to_string(ctx) {
                            body_parts.push(s.to_std_string_escaped().into_bytes());
                        }
                    }
                    let body: Vec<u8> = body_parts.into_iter().flatten().collect();

                    let mut headers: Vec<(String, String)> = Vec::new();
                    if let Ok(hdr_val) = res_for_end.get(js_string!("__headers"), ctx) {
                        if let Some(hdr_obj) = hdr_val.as_object() {
                            for key in hdr_obj.own_property_keys(ctx).unwrap_or_default() {
                                if let Ok(val) = hdr_obj.get(key.clone(), ctx) {
                                    if let Some(v) = val.as_string() {
                                        let ks = key.to_string();
                                        headers.push((ks, v.to_std_string_escaped()));
                                    }
                                }
                            }
                        }
                    }

                    let sc = res_for_end
                        .get(js_string!("statusCode"), ctx)
                        .ok()
                        .and_then(|v| v.as_number())
                        .unwrap_or(200.0) as u16;
                    let sm = res_for_end
                        .get(js_string!("statusMessage"), ctx)
                        .ok()
                        .and_then(|v| v.to_string(ctx).ok())
                        .map(|s| s.to_std_string_escaped())
                        .unwrap_or_default();

                    let resp = build_response_string(sc, &sm, &headers, &body);
                    if let Ok(mut stream) = stream_for_end.lock() {
                        let _ = stream.write_all(resp.as_bytes());
                        let _ = stream.shutdown(std::net::Shutdown::Write);
                    }
                    let _ =
                        res_for_end.set(js_string!("__ended"), JsValue::from(true), false, ctx);
                    let _ =
                        res_for_end.set(js_string!("__sent"), JsValue::from(true), false, ctx);

                    emit(&res_for_end, "close", &[], ctx);

                    if let Some(cb) = end_args
                        .get(1)
                        .or_else(|| end_args.get(2))
                        .and_then(|v| v.as_object().filter(|o| o.is_callable()))
                    {
                        let _ = cb.call(&JsValue::undefined(), &[], ctx);
                    }
                    Ok(JsValue::from(true))
                },
            ),
            "end",
            2,
            ctx,
        ),
        false,
        ctx,
    );

    res
}
