use boa_engine::object::builtins::JsArray;
use boa_engine::{js_string, Context, JsObject, JsResult, JsValue};

use super::common::{build_fn, emit, get_obj, make_native, add_listener};

/// Creates the .write(chunk, encoding, cb) method for client request
pub fn create_request_write(ctx: &mut Context) -> JsValue {
    build_fn(
        make_native(
            |this: &JsValue, args: &[JsValue], ctx: &mut Context| -> Result<JsValue, _> {
                let inst = get_obj(this)?;
                if let Some(data) = args.first() {
                    let s = data.to_string(ctx)
                        .ok()
                        .map(|s| s.to_std_string_escaped())
                        .unwrap_or_default();
                    if let Some(buf) = inst
                        .get(js_string!("__buffer"), ctx)
                        .ok()
                        .and_then(|v| v.as_object())
                        .and_then(|o| JsArray::from_object(o.clone()).ok())
                    {
                        let _ = buf.push(JsValue::from(js_string!(s)), ctx);
                    }
                }
                if let Some(cb) = args.get(1).or_else(|| args.get(2))
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
    )
}

/// Creates the .end(data, encoding, cb) method for client request
pub fn create_request_end(ctx: &mut Context) -> JsValue {
    build_fn(
        make_native(
            move |this: &JsValue, end_args: &[JsValue], ctx: &mut Context| -> Result<JsValue, _> {
                let inst = get_obj(this)?;

                let mut body_parts: Vec<Vec<u8>> = Vec::new();
                if let Ok(buf_val) = inst.get(js_string!("__buffer"), ctx) {
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

                let method = inst.get(js_string!("__method"), ctx).ok()
                    .and_then(|v| v.to_string(ctx).ok())
                    .map(|s| s.to_std_string_escaped())
                    .unwrap_or_else(|| "GET".to_string());
                let url = inst.get(js_string!("__url"), ctx).ok()
                    .and_then(|v| v.to_string(ctx).ok())
                    .map(|s| s.to_std_string_escaped())
                    .unwrap_or_default();
                let callback_val = inst.get(js_string!("__callback"), ctx).ok();

                let mut req_headers: Vec<(String, String)> = Vec::new();
                if let Ok(hdr_val) = inst.get(js_string!("__headers"), ctx) {
                    if let Some(hdr_obj) = hdr_val.as_object() {
                        for key in hdr_obj.own_property_keys(ctx).unwrap_or_default() {
                            if let Ok(val) = hdr_obj.get(key.clone(), ctx) {
                                if let Some(v) = val.as_string() {
                                    req_headers.push((key.to_string(), v.to_std_string_escaped()));
                                }
                            }
                        }
                    }
                }

                let result = (|| -> Result<String, String> {
                    let client = reqwest::blocking::Client::builder()
                        .danger_accept_invalid_certs(true)
                        .build()
                        .map_err(|e| e.to_string())?;

                    let mut rb = client.request(
                        reqwest::Method::from_bytes(method.as_bytes()).map_err(|e| e.to_string())?,
                        &url,
                    );

                    for (k, v) in &req_headers {
                        rb = rb.header(k.as_str(), v.as_str());
                    }
                    if !body.is_empty() {
                        rb = rb.body(body.clone());
                    }

                    let resp = rb.send().map_err(|e| format!("request failed: {e}"))?;

                    let status_code = resp.status().as_u16();
                    let status_msg = resp.status().canonical_reason().unwrap_or("Unknown").to_string();

                    let mut resp_headers: Vec<(String, String)> = Vec::new();
                    for (k, v) in resp.headers() {
                        if let Ok(val) = v.to_str() {
                            resp_headers.push((k.as_str().to_string(), val.to_string()));
                        }
                    }

                    let resp_body = resp.bytes().map_err(|e| format!("read body: {e}"))?.to_vec();

                    let resp_json = serde_json::json!({
                        "statusCode": status_code,
                        "statusMessage": status_msg,
                        "headers": resp_headers.iter().map(|(k,v)| {
                            serde_json::json!([k, v])
                        }).collect::<Vec<_>>(),
                        "body": String::from_utf8_lossy(&resp_body).to_string(),
                    });
                    Ok(resp_json.to_string())
                })();

                match result {
                    Ok(json_str) => {
                        let res_inner = JsObject::with_object_proto(ctx.intrinsics());
                        let _ = res_inner.set(js_string!("_events"), JsValue::from(JsObject::with_object_proto(ctx.intrinsics())), false, ctx);

                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json_str) {
                            if let Some(sc) = parsed["statusCode"].as_u64() {
                                let _ = res_inner.set(js_string!("statusCode"), JsValue::from(sc as f64), false, ctx);
                            }
                            if let Some(sm) = parsed["statusMessage"].as_str() {
                                let _ = res_inner.set(js_string!("statusMessage"), js_string!(sm), false, ctx);
                            }
                            let hdr_obj = JsObject::with_object_proto(ctx.intrinsics());
                            if let Some(hdrs) = parsed["headers"].as_array() {
                                for h in hdrs {
                                    if let Some(arr) = h.as_array() {
                                        if arr.len() >= 2 {
                                            let k = arr[0].as_str().unwrap_or("");
                                            let v = arr[1].as_str().unwrap_or("");
                                            let _ = hdr_obj.set(js_string!(k.to_lowercase()), js_string!(v), false, ctx);
                                        }
                                    }
                                }
                            }
                            let _ = res_inner.set(js_string!("headers"), JsValue::from(hdr_obj), false, ctx);

                            let res_on = build_fn(
                                make_native(
                                    |this: &JsValue, args: &[JsValue], ctx: &mut Context| -> Result<JsValue, _> {
                                        if let Some(inst) = this.as_object() {
                                            let name = args.first()
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
                            let _ = res_inner.set(js_string!("on"), res_on, false, ctx);

                            if let Some(body_str) = parsed["body"].as_str() {
                                if !body_str.is_empty() {
                                    emit(&res_inner, "data", &[JsValue::from(js_string!(body_str))], ctx);
                                }
                            }
                            emit(&res_inner, "end", &[], ctx);

                            if let Some(cb) = &callback_val {
                                if let Some(cb_fn) = cb.as_object().filter(|o| o.is_callable()) {
                                    let _ = cb_fn.call(&JsValue::undefined(), &[JsValue::from(res_inner)], ctx);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        if let Some(cb) = &callback_val {
                            if let Some(cb_fn) = cb.as_object().filter(|o| o.is_callable()) {
                                let _ = cb_fn.call(&JsValue::undefined(), &[JsValue::from(js_string!(e))], ctx);
                            }
                        }
                    }
                }

                let _ = inst.set(js_string!("__ended"), JsValue::from(true), false, ctx);
                Ok(JsValue::from(true))
            },
        ),
        "end",
        2,
        ctx,
    )
}
