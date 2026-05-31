use std::io::{BufReader, Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};

use boa_engine::object::builtins::JsArray;
use boa_engine::object::FunctionObjectBuilder;
use boa_engine::{
    js_string, Context, JsNativeError, JsObject, JsResult, JsString, JsValue, NativeFunction,
};

pub fn make_native<F>(f: F) -> NativeFunction
where
    F: Fn(&JsValue, &[JsValue], &mut Context) -> JsResult<JsValue> + 'static,
{
    unsafe { NativeFunction::from_closure(f) }
}

pub fn build_fn(f: NativeFunction, name: &str, len: usize, ctx: &mut Context) -> JsValue {
    FunctionObjectBuilder::new(ctx.realm(), f)
        .name(name)
        .length(len)
        .build()
        .into()
}

pub fn get_obj(v: &JsValue) -> JsResult<JsObject> {
    v.as_object()
        .ok_or_else(|| JsNativeError::typ().with_message("not an object").into())
}

pub fn add_listener(
    inst: &JsObject,
    name: &str,
    listener: &JsValue,
    ctx: &mut Context,
) -> JsResult<()> {
    if !inst
        .has_own_property(js_string!("_events"), ctx)
        .unwrap_or(false)
    {
        let ev = JsObject::with_object_proto(ctx.intrinsics());
        let _ = inst.set(js_string!("_events"), JsValue::from(ev), false, ctx);
    }
    let events = inst
        .get(js_string!("_events"), ctx)?
        .as_object()
        .ok_or_else(|| JsNativeError::typ().with_message("no _events"))?
        .clone();
    let key = js_string!(name);
    let arr = if let Ok(val) = events.get(js_string!(name), ctx) {
        if let Some(obj) = val.as_object() {
            if let Ok(a) = JsArray::from_object(obj.clone()) {
                a
            } else {
                let a = JsArray::new(ctx);
                let _ = events.set(key, JsValue::from(a.clone()), false, ctx);
                a
            }
        } else {
            let a = JsArray::new(ctx);
            let _ = events.set(key, JsValue::from(a.clone()), false, ctx);
            a
        }
    } else {
        let a = JsArray::new(ctx);
        let _ = events.set(key, JsValue::from(a.clone()), false, ctx);
        a
    };
    let _ = arr.push(listener.clone(), ctx);
    Ok(())
}

pub fn emit(inst: &JsObject, name: &str, args: &[JsValue], ctx: &mut Context) {
    if let Ok(events) = inst.get(js_string!("_events"), ctx) {
        if let Some(ev_obj) = events.as_object() {
            if let Ok(val) = ev_obj.get(js_string!(name), ctx) {
                if let Some(arr_obj) = val.as_object() {
                    if let Ok(arr) = JsArray::from_object(arr_obj.clone()) {
                        let items: Vec<JsValue> = (0..arr.length(ctx).unwrap_or(0))
                            .filter_map(|i| arr.get(i, ctx).ok())
                            .collect();
                        for item in &items {
                            if let Some(fn_obj) = item.as_object().filter(|o| o.is_callable()) {
                                let _ = fn_obj.call(&JsValue::from(inst.clone()), args, ctx);
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn status_text(code: u16) -> &'static str {
    match code {
        100 => "Continue",
        101 => "Switching Protocols",
        200 => "OK",
        201 => "Created",
        202 => "Accepted",
        204 => "No Content",
        206 => "Partial Content",
        301 => "Moved Permanently",
        302 => "Found",
        303 => "See Other",
        304 => "Not Modified",
        307 => "Temporary Redirect",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        408 => "Request Timeout",
        409 => "Conflict",
        410 => "Gone",
        411 => "Length Required",
        413 => "Payload Too Large",
        415 => "Unsupported Media Type",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        _ => {
            if code < 400 {
                "Unknown"
            } else {
                "Error"
            }
        }
    }
}

pub fn build_response_string(
    status_code: u16,
    status_msg: &str,
    headers: &[(String, String)],
    body: &[u8],
) -> String {
    let msg = if status_msg.is_empty() {
        status_text(status_code)
    } else {
        status_msg
    };
    let mut resp = format!("HTTP/1.1 {status_code} {msg}\r\n");
    resp.push_str(&format!("Content-Length: {}\r\n", body.len()));
    resp.push_str("Connection: close\r\n");
    for (k, v) in headers {
        resp.push_str(&format!("{k}: {v}\r\n"));
    }
    resp.push_str("\r\n");
    if !body.is_empty() {
        resp.push_str(&String::from_utf8_lossy(body));
    }
    resp
}

pub fn parse_http_request(
    stream: &mut TcpStream,
) -> Result<(String, String, Vec<(String, String)>, Vec<u8>), String> {
    let mut reader = BufReader::new(stream.try_clone().map_err(|e| e.to_string())?);
    let mut buf = Vec::new();
    let mut header_size = 0usize;

    loop {
        let mut temp = [0u8; 4096];
        match reader.read(&mut temp) {
            Ok(0) => break,
            Ok(n) => {
                buf.extend_from_slice(&temp[..n]);
                let mut h = [httparse::EMPTY_HEADER; 64];
                let mut req = httparse::Request::new(&mut h);
                match req.parse(&buf) {
                    Ok(httparse::Status::Complete(sz)) => {
                        header_size = sz;
                        break;
                    }
                    Ok(httparse::Status::Partial) => continue,
                    Err(e) => return Err(format!("httparse error: {e}")),
                }
            }
            Err(e) => return Err(format!("read error: {e}")),
        }
    }

    if header_size == 0 {
        return Err("incomplete HTTP headers".to_string());
    }

    let mut h = [httparse::EMPTY_HEADER; 64];
    let mut req = httparse::Request::new(&mut h);
    match req.parse(&buf) {
        Ok(httparse::Status::Complete(_)) => (),
        _ => return Err("incomplete HTTP headers after reading".to_string()),
    }

    let method = req.method.unwrap_or("GET").to_string();
    let path = req.path.unwrap_or("/").to_string();
    let mut headers: Vec<(String, String)> = Vec::new();

    let content_length: usize = req
        .headers
        .iter()
        .find(|h| h.name.eq_ignore_ascii_case("content-length"))
        .and_then(|h| std::str::from_utf8(h.value).ok())
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);

    for h in req.headers {
        if let Ok(v) = std::str::from_utf8(h.value) {
            headers.push((h.name.to_string(), v.to_string()));
        }
    }

    let body = if content_length > 0 {
        let remaining = buf.len() - header_size;
        if remaining >= content_length {
            buf[header_size..header_size + content_length].to_vec()
        } else {
            let mut body = buf[header_size..].to_vec();
            let mut temp = vec![0u8; content_length - remaining];
            if let Ok(_n) = reader.read_exact(&mut temp) {
                body.extend_from_slice(&temp);
            }
            body
        }
    } else {
        Vec::new()
    };

    Ok((method, path, headers, body))
}

pub fn collect_res_headers(res: &JsObject, ctx: &mut Context) -> Vec<(String, String)> {
    let mut headers = Vec::new();
    if let Ok(hdr_val) = res.get(js_string!("__headers"), ctx) {
        if let Some(hdr_obj) = hdr_val.as_object() {
            for key in hdr_obj.own_property_keys(ctx).unwrap_or_default() {
                if let Ok(val) = hdr_obj.get(key.clone(), ctx) {
                    if let Some(v) = val.as_string() {
                        headers.push((key.to_string(), v.to_std_string_escaped()));
                    }
                }
            }
        }
    }
    headers
}

pub fn flush_response(res: &JsObject, body: &[u8], stream: &Arc<Mutex<TcpStream>>, ctx: &mut Context) {
    let sc = res
        .get(js_string!("statusCode"), ctx)
        .ok()
        .and_then(|v| v.as_number())
        .unwrap_or(200.0) as u16;
    let sm = res
        .get(js_string!("statusMessage"), ctx)
        .ok()
        .and_then(|v| v.to_string(ctx).ok())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();
    let headers = collect_res_headers(res, ctx);
    let resp = build_response_string(sc, &sm, &headers, body);
    if let Ok(mut s) = stream.lock() {
        let _ = s.write_all(resp.as_bytes());
    }
}

pub fn create_response_from_reqwest(
    resp: reqwest::blocking::Response,
    ctx: &mut Context,
) -> JsResult<JsObject> {
    let res = JsObject::with_object_proto(ctx.intrinsics());
    let _ = res.set(
        js_string!("statusCode"),
        JsValue::from(resp.status().as_u16() as f64),
        false,
        ctx,
    );
    let _ = res.set(
        js_string!("statusMessage"),
        js_string!(resp.status().canonical_reason().unwrap_or("Unknown")),
        false,
        ctx,
    );
    let _ = res.set(
        js_string!("headers"),
        JsValue::from(JsObject::with_object_proto(ctx.intrinsics())),
        false,
        ctx,
    );
    let _ = res.set(
        js_string!("__body"),
        JsValue::from(JsArray::new(ctx)),
        false,
        ctx,
    );
    let _ = res.set(
        js_string!("_events"),
        JsValue::from(JsObject::with_object_proto(ctx.intrinsics())),
        false,
        ctx,
    );

    if let Some(hdr_obj) = res
        .get(js_string!("headers"), ctx)
        .ok()
        .and_then(|v| v.as_object())
    {
        for (k, v) in resp.headers() {
            if let Ok(val) = v.to_str() {
                let _ = hdr_obj.set(
                    js_string!(k.as_str().to_lowercase()),
                    js_string!(val),
                    false,
                    ctx,
                );
            }
        }
    }

    if let Ok(bytes) = resp.bytes() {
        let body_arr = JsArray::new(ctx);
        let body_str = String::from_utf8_lossy(&bytes).to_string();
        let _ = body_arr.push(JsValue::from(js_string!(body_str)), ctx);
        let _ = res.set(js_string!("__body"), JsValue::from(body_arr), false, ctx);
    }

    let res_on = build_fn(
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
    );
    let _ = res.set(js_string!("on"), res_on, false, ctx);

    Ok(res)
}
