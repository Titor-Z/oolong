use boa_engine::object::builtins::JsArray;
use boa_engine::{js_string, Context, JsValue};

use super::common::{build_fn, emit, get_obj, make_native, create_response_from_reqwest};

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

                let result = (|| -> Result<reqwest::blocking::Response, String> {
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
                    Ok(resp)
                })();

                match result {
                    Ok(resp) => {
                        match create_response_from_reqwest(resp, ctx) {
                            Ok(res_obj) => {
                                if let Ok(body_val) = res_obj.get(js_string!("__body"), ctx) {
                                    if let Some(arr_obj) = body_val.as_object() {
                                        if let Ok(arr) = JsArray::from_object(arr_obj.clone()) {
                                            for i in 0..arr.length(ctx).unwrap_or(0) {
                                                if let Ok(item) = arr.get(i, ctx) {
                                                    emit(&res_obj, "data", &[item], ctx);
                                                }
                                            }
                                        }
                                    }
                                }
                                emit(&res_obj, "end", &[], ctx);

                                if let Some(cb) = &callback_val {
                                    if let Some(cb_fn) = cb.as_object().filter(|o| o.is_callable()) {
                                        let _ = cb_fn.call(&JsValue::undefined(), &[JsValue::from(res_obj)], ctx);
                                    }
                                }
                            }
                            Err(e) => {
                                if let Some(cb) = &callback_val {
                                    if let Some(cb_fn) = cb.as_object().filter(|o| o.is_callable()) {
                                        let _ = cb_fn.call(&JsValue::undefined(), &[JsValue::from(js_string!(format!("res creation: {e}")))], ctx);
                                    }
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
