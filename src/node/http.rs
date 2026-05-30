use std::io::{BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

use boa_engine::module::SyntheticModuleInitializer;
use boa_engine::object::FunctionObjectBuilder;
use boa_engine::object::builtins::JsArray;
use boa_engine::{
    Context, JsError, JsNativeError, JsObject, JsResult, JsString, JsValue, Module, NativeFunction,
    js_string,
};

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
    v.as_object()
        .ok_or_else(|| JsNativeError::typ().with_message("not an object").into())
}

fn add_listener(
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
            if let Ok(a) = boa_engine::object::builtins::JsArray::from_object(obj.clone()) {
                a
            } else {
                let a = boa_engine::object::builtins::JsArray::new(ctx);
                let _ = events.set(key, JsValue::from(a.clone()), false, ctx);
                a
            }
        } else {
            let a = boa_engine::object::builtins::JsArray::new(ctx);
            let _ = events.set(key, JsValue::from(a.clone()), false, ctx);
            a
        }
    } else {
        let a = boa_engine::object::builtins::JsArray::new(ctx);
        let _ = events.set(key, JsValue::from(a.clone()), false, ctx);
        a
    };
    let _ = arr.push(listener.clone(), ctx);
    Ok(())
}

fn emit(inst: &JsObject, name: &str, args: &[JsValue], ctx: &mut Context) {
    if let Ok(events) = inst.get(js_string!("_events"), ctx) {
        if let Some(ev_obj) = events.as_object() {
            if let Ok(val) = ev_obj.get(js_string!(name), ctx) {
                if let Some(arr_obj) = val.as_object() {
                    if let Ok(arr) =
                        boa_engine::object::builtins::JsArray::from_object(arr_obj.clone())
                    {
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

fn status_text(code: u16) -> &'static str {
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

fn build_response_string(
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

fn parse_http_request(
    stream: &mut TcpStream,
) -> Result<(String, String, Vec<(String, String)>, Vec<u8>), String> {
    let mut reader = BufReader::new(stream.try_clone().map_err(|e| e.to_string())?);
    let mut buf = Vec::new();
    let mut headers_done = false;
    let mut header_bytes = 0usize;

    loop {
        let mut temp = [0u8; 4096];
        match reader.read(&mut temp) {
            Ok(0) => break,
            Ok(n) => {
                buf.extend_from_slice(&temp[..n]);
                if !headers_done {
                    if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                        header_bytes = pos + 4;
                        headers_done = true;
                        break;
                    }
                }
            }
            Err(e) => return Err(format!("read error: {e}")),
        }
    }

    if !headers_done {
        return Err("incomplete HTTP headers".to_string());
    }

    let header_section = &buf[..header_bytes];
    let header_text = String::from_utf8_lossy(header_section);
    let mut lines = header_text.lines();

    let request_line = lines.next().ok_or("empty request line")?;
    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 3 {
        return Err(format!("invalid request line: {request_line}"));
    }
    let method = parts[0].to_string();
    let path = parts[1].to_string();

    let mut headers: Vec<(String, String)> = Vec::new();
    for line in lines {
        if line.is_empty() {
            continue;
        }
        if let Some(pos) = line.find(':') {
            let k = line[..pos].trim().to_string();
            let v = line[pos + 1..].trim().to_string();
            headers.push((k, v));
        }
    }

    let content_length: usize = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, v)| v.parse().ok())
        .unwrap_or(0);

    let body = if content_length > 0 {
        let remaining = buf.len() - header_bytes;
        if remaining >= content_length {
            buf[header_bytes..header_bytes + content_length].to_vec()
        } else {
            let mut body = buf[header_bytes..].to_vec();
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

fn create_incoming_message(
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
        let _ = raw_arr.push(JsString::from(k.as_str()), ctx);
        let _ = raw_arr.push(JsString::from(v.as_str()), ctx);
    }
    let _ = req.set(
        js_string!("headers"),
        JsValue::from(headers_obj),
        false,
        ctx,
    );
    let _ = req.set(js_string!("rawHeaders"), JsValue::from(raw_arr), false, ctx);

    // socket
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

    // EventEmitter via _events
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

fn create_outgoing_message(stream: Arc<Mutex<TcpStream>>, ctx: &mut Context) -> JsObject {
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
                    // If arg[1] is an object (headers) or arg[2] is an object
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

    // write(chunk, encoding, cb) - buffer body chunks
    let _ = res.set(
        js_string!("write"),
        build_fn(
            make_native(
                |this: &JsValue, args: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
                    let inst = get_obj(this)?;
                    if let Some(data) = args.first() {
                        let chunk_str = if let Some(s) = data.as_string() {
                            s.to_std_string_escaped()
                        } else if data.as_object().is_some() {
                            data.to_string(ctx)
                                .ok()
                                .map(|s| s.to_std_string_escaped())
                                .unwrap_or_default()
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
                    // encoding arg (arg[1]) ignored for now
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

    // end(data, encoding, cb) - flush response to TcpStream
    let res_for_stream = res.clone();
    let stream_for_end = stream.clone();
    let _ = res.set(
        js_string!("end"),
        build_fn(
            make_native(
                move |_: &JsValue, end_args: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
                    let mut body_parts: Vec<Vec<u8>> = Vec::new();
                    if let Ok(buf_val) = res_for_stream.get(js_string!("__buffer"), ctx) {
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
                        } else if data.as_object().is_some() {
                            if let Ok(s) = data.to_string(ctx) {
                                body_parts.push(s.to_std_string_escaped().into_bytes());
                            }
                        }
                    }
                    let body: Vec<u8> = body_parts.into_iter().flatten().collect();

                    // Collect headers from __headers
                    let mut headers: Vec<(String, String)> = Vec::new();
                    if let Ok(hdr_val) = res_for_stream.get(js_string!("__headers"), ctx) {
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

                    let sc = res_for_stream
                        .get(js_string!("statusCode"), ctx)
                        .ok()
                        .and_then(|v| v.as_number())
                        .unwrap_or(200.0) as u16;
                    let sm = res_for_stream
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
                        res_for_stream.set(js_string!("__ended"), JsValue::from(true), false, ctx);
                    let _ =
                        res_for_stream.set(js_string!("__sent"), JsValue::from(true), false, ctx);

                    emit(&res_for_stream, "close", &[], ctx);

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

fn collect_res_headers(res: &JsObject, ctx: &mut Context) -> Vec<(String, String)> {
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

fn flush_response(res: &JsObject, body: &[u8], stream: &Arc<Mutex<TcpStream>>, ctx: &mut Context) {
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

fn create_response_from_reqwest(
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
    // Emit events after callback sets up listeners
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

    // Fill headers
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

    // Store body
    if let Ok(bytes) = resp.bytes() {
        let body_arr = JsArray::new(ctx);
        let body_str = String::from_utf8_lossy(&bytes).to_string();
        let _ = body_arr.push(JsValue::from(js_string!(body_str)), ctx);
        let _ = res.set(js_string!("__body"), JsValue::from(body_arr), false, ctx);
    }

    // on(event, cb)
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

pub fn create_node_http_module(context: &mut Context) -> Result<Module, String> {
    let export_names: &[JsString] = &[
        js_string!("createServer"),
        js_string!("Server"),
        js_string!("request"),
        js_string!("get"),
        js_string!("STATUS_CODES"),
        js_string!("default"),
    ];

    let module = Module::synthetic(
        export_names,
        unsafe {
            SyntheticModuleInitializer::from_closure(
                move |m: &boa_engine::module::SyntheticModule, ctx: &mut Context| {
                    // STATUS_CODES
                    let status_codes = JsObject::with_object_proto(ctx.intrinsics());
                    for &(code, text) in &[
                        (100, "Continue"),
                        (101, "Switching Protocols"),
                        (200, "OK"),
                        (201, "Created"),
                        (202, "Accepted"),
                        (204, "No Content"),
                        (206, "Partial Content"),
                        (301, "Moved Permanently"),
                        (302, "Found"),
                        (303, "See Other"),
                        (304, "Not Modified"),
                        (307, "Temporary Redirect"),
                        (400, "Bad Request"),
                        (401, "Unauthorized"),
                        (403, "Forbidden"),
                        (404, "Not Found"),
                        (405, "Method Not Allowed"),
                        (408, "Request Timeout"),
                        (409, "Conflict"),
                        (410, "Gone"),
                        (411, "Length Required"),
                        (413, "Payload Too Large"),
                        (415, "Unsupported Media Type"),
                        (429, "Too Many Requests"),
                        (500, "Internal Server Error"),
                        (501, "Not Implemented"),
                        (502, "Bad Gateway"),
                        (503, "Service Unavailable"),
                        (504, "Gateway Timeout"),
                    ] {
                        let _ = status_codes.set(
                            JsString::from(code.to_string()),
                            js_string!(text),
                            false,
                            ctx,
                        );
                    }

                    // ── createServer(requestListener) ────────────────────────
                    let create_server = build_fn(
                        make_native(
                            |_: &JsValue,
                             args: &[JsValue],
                             ctx: &mut Context|
                             -> JsResult<JsValue> {
                                let request_listener =
                                    args.first().cloned().unwrap_or(JsValue::undefined());

                                let server = JsObject::with_object_proto(ctx.intrinsics());
                                let _ = server.set(
                                    js_string!("__listening"),
                                    JsValue::from(false),
                                    false,
                                    ctx,
                                );
                                let _ = server.set(
                                    js_string!("_events"),
                                    JsValue::from(JsObject::with_object_proto(ctx.intrinsics())),
                                    false,
                                    ctx,
                                );

                                if !request_listener.is_undefined() {
                                    let _ = server.set(
                                        js_string!("__handler"),
                                        request_listener,
                                        false,
                                        ctx,
                                    );
                                }

                                // on(event, cb)
                                let server_on = build_fn(
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
                                let _ = server.set(js_string!("on"), server_on, false, ctx);

                                // close(cb)
                                let server_close = build_fn(
                                    make_native(
                                        |this: &JsValue, args: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
                                            let inst = get_obj(this)?;
                                            let _ = inst.set(
                                                js_string!("__listening"),
                                                JsValue::from(false),
                                                false,
                                                ctx,
                                            );
                                            emit(&inst, "close", &[], ctx);
                                            if let Some(cb) = args
                                                .first()
                                                .and_then(|v| v.as_object())
                                                .filter(|o| o.is_callable())
                                            {
                                                let _ =
                                                    cb.call(&JsValue::undefined(), &[], ctx);
                                            }
                                            Ok(JsValue::undefined())
                                        },
                                    ),
                                    "close",
                                    1,
                                    ctx,
                                );
                                let _ = server.set(js_string!("close"), server_close, false, ctx);

                                // address()
                                let server_addr = build_fn(
                                    make_native(
                                        |this: &JsValue, _args: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
                                            let inst = get_obj(this)?;
                                            let port_val = inst
                                                .get(js_string!("__port"), ctx)
                                                .ok()
                                                .and_then(|v| v.as_number())
                                                .unwrap_or(0.0);
                                            let host =
                                                inst.get(js_string!("__host"), ctx)
                                                    .ok()
                                                    .and_then(|v| v.to_string(ctx).ok())
                                                    .map(|s| s.to_std_string_escaped())
                                                    .unwrap_or_else(|| "127.0.0.1".to_string());
                                            let addr = JsObject::with_object_proto(ctx.intrinsics());
                                            let _ = addr.set(
                                                js_string!("address"),
                                                js_string!(host.as_str()),
                                                false,
                                                ctx,
                                            );
                                            let _ = addr.set(
                                                js_string!("port"),
                                                JsValue::from(port_val),
                                                false,
                                                ctx,
                                            );
                                            let _ = addr.set(
                                                js_string!("family"),
                                                js_string!("IPv4"),
                                                false,
                                                ctx,
                                            );
                                            Ok(JsValue::from(addr))
                                        },
                                    ),
                                    "address",
                                    0,
                                    ctx,
                                );
                                let _ = server.set(js_string!("address"), server_addr, false, ctx);

                                // listen(port, host, cb)
                                let server_listen = make_native(
                                    |this: &JsValue,
                                     args: &[JsValue],
                                     ctx: &mut Context|
                                     -> JsResult<JsValue> {
                                        let port =
                                            args.first().and_then(|v| v.as_number()).unwrap_or(0.0)
                                                as u16;
                                        let host_arg = args
                                            .get(1)
                                            .and_then(|v| v.to_string(ctx).ok())
                                            .map(|s| s.to_std_string_escaped());
                                        let cb_val = if host_arg.is_some() {
                                            args.get(2)
                                        } else {
                                            args.get(1)
                                        };
                                        let host =
                                            host_arg.unwrap_or_else(|| "127.0.0.1".to_string());

                                        let listener = TcpListener::bind(format!("{host}:{port}"))
                                            .map_err(|e| -> JsError {
                                                JsNativeError::typ()
                                                    .with_message(format!(
                                                        "EADDRINUSE port={port}: {e}"
                                                    ))
                                                    .into()
                                            })?;
                                        let addr =
                                            listener.local_addr().map_err(|e| -> JsError {
                                                JsNativeError::typ()
                                                    .with_message(format!("addr error: {e}"))
                                                    .into()
                                            })?;

                                        let inst = get_obj(this)?;
                                        let _ = inst.set(
                                            js_string!("__listening"),
                                            JsValue::from(true),
                                            false,
                                            ctx,
                                        );
                                        let _ = inst.set(
                                            js_string!("__port"),
                                            JsValue::from(addr.port() as f64),
                                            false,
                                            ctx,
                                        );
                                        let _ = inst.set(
                                            js_string!("__host"),
                                            js_string!(addr.ip().to_string()),
                                            false,
                                            ctx,
                                        );

                                        emit(&inst, "listening", &[], ctx);

                                        if let Some(cb) = cb_val {
                                            if let Some(cb_fn) =
                                                cb.as_object().filter(|o| o.is_callable())
                                            {
                                                let _ = cb_fn.call(&JsValue::undefined(), &[], ctx);
                                            }
                                        }

                                        // Accept loop (synchronous blocking)
                                        for stream in listener.incoming() {
                                            let _listening = inst
                                                .get(js_string!("__listening"), ctx)
                                                .ok()
                                                .and_then(|v| v.as_boolean())
                                                .unwrap_or(false);
                                            if !_listening {
                                                break;
                                            }

                                            match stream {
                                                Ok(mut tcp_stream) => {
                                                    let peer = tcp_stream.peer_addr().ok();
                                                    let peer_ip =
                                                        peer.as_ref().map(|p| p.ip().to_string());
                                                    let peer_port =
                                                        peer.map(|p| p.port()).unwrap_or(0);

                                                    match parse_http_request(&mut tcp_stream) {
                                                        Ok((method, path, headers, body)) => {
                                                            let stream_arc =
                                                                Arc::new(Mutex::new(tcp_stream));

                                                            let req = create_incoming_message(
                                                                &method,
                                                                &path,
                                                                "1.1",
                                                                &headers,
                                                                body.clone(),
                                                                peer_ip.as_deref(),
                                                                peer_port,
                                                                ctx,
                                                            );
                                                            let res = create_outgoing_message(
                                                                stream_arc.clone(),
                                                                ctx,
                                                            );

                                                            // Emit 'request' event on server
                                                            emit(
                                                                &inst,
                                                                "request",
                                                                &[
                                                                    JsValue::from(req.clone()),
                                                                    JsValue::from(res.clone()),
                                                                ],
                                                                ctx,
                                                            );

                                                            // Call handler
                                                            if let Some(handler) = inst
                                                                .get(js_string!("__handler"), ctx)
                                                                .ok()
                                                                .and_then(|v| {
                                                                    v.as_object()
                                                                        .filter(|o| o.is_callable())
                                                                })
                                                            {
                                                                let _ = handler.call(
                                                                    &JsValue::undefined(),
                                                                    &[
                                                                        JsValue::from(req.clone()),
                                                                        JsValue::from(res.clone()),
                                                                    ],
                                                                    ctx,
                                                                );
                                                            }

                                                            // After handler, emit body events on req
                                                            let ended = res
                                                                .get(js_string!("__ended"), ctx)
                                                                .ok()
                                                                .and_then(|v| v.as_boolean())
                                                                .unwrap_or(false);

                                                            if !ended {
                                                                // Emit data + end on req
                                                                let body_str =
                                                                    String::from_utf8_lossy(&body);
                                                                if !body.is_empty() {
                                                                    emit(
                                                                        &req,
                                                                        "data",
                                                                        &[JsValue::from(
                                                                            js_string!(
                                                                                body_str
                                                                                    .to_string()
                                                                            ),
                                                                        )],
                                                                        ctx,
                                                                    );
                                                                }
                                                                emit(&req, "end", &[], ctx);
                                                            }

                                                            // If still not ended after body events, auto-end
                                                            let ended2 = res
                                                                .get(js_string!("__ended"), ctx)
                                                                .ok()
                                                                .and_then(|v| v.as_boolean())
                                                                .unwrap_or(false);
                                                            if !ended2 {
                                                                flush_response(
                                                                    &res,
                                                                    &[],
                                                                    &stream_arc,
                                                                    ctx,
                                                                );
                                                                let _ = res.set(
                                                                    js_string!("__ended"),
                                                                    JsValue::from(true),
                                                                    false,
                                                                    ctx,
                                                                );
                                                                emit(&res, "close", &[], ctx);
                                                            }

                                                            // Shutdown write side to signal client connection is done
                                                            if let Ok(mut s) = stream_arc.lock() {
                                                                let _ = s.shutdown(
                                                                    std::net::Shutdown::Write,
                                                                );
                                                            }
                                                        }
                                                        Err(e) => {
                                                            let err_resp = build_response_string(
                                                                400,
                                                                "Bad Request",
                                                                &[],
                                                                format!("Bad Request: {e}")
                                                                    .as_bytes(),
                                                            );
                                                            let _ = tcp_stream
                                                                .write_all(err_resp.as_bytes());
                                                            let _ = tcp_stream.shutdown(
                                                                std::net::Shutdown::Write,
                                                            );
                                                        }
                                                    }
                                                }
                                                Err(_) => break,
                                            }
                                        }

                                        Ok(JsValue::undefined())
                                    },
                                );
                                let listen_val =
                                    FunctionObjectBuilder::new(ctx.realm(), server_listen)
                                        .name("listen")
                                        .length(2)
                                        .build();
                                let _ = server.set(
                                    js_string!("listen"),
                                    JsValue::from(listen_val),
                                    false,
                                    ctx,
                                );

                                Ok(JsValue::from(server))
                            },
                        ),
                        "createServer",
                        1,
                        ctx,
                    );

                    // ── request(options, callback) ───────────────────────────
                    let http_request = build_fn(
                        make_native(
                            |_: &JsValue,
                             args: &[JsValue],
                             ctx: &mut Context|
                             -> JsResult<JsValue> {
                                let callback = args.get(1).cloned().unwrap_or(JsValue::undefined());

                                let creq = JsObject::with_object_proto(ctx.intrinsics());
                                let _ =
                                    creq.set(js_string!("__method"), js_string!("GET"), false, ctx);
                                let _ = creq.set(js_string!("__url"), js_string!(""), false, ctx);
                                let _ = creq.set(
                                    js_string!("__headers"),
                                    JsValue::from(JsObject::with_object_proto(ctx.intrinsics())),
                                    false,
                                    ctx,
                                );
                                let _ = creq.set(
                                    js_string!("__buffer"),
                                    JsValue::from(JsArray::new(ctx)),
                                    false,
                                    ctx,
                                );
                                let _ = creq.set(js_string!("__callback"), callback, false, ctx);
                                let _ = creq.set(
                                    js_string!("__ended"),
                                    JsValue::from(false),
                                    false,
                                    ctx,
                                );

                                // Parse options
                                if let Some(opts) = args.first() {
                                    if let Some(s) = opts.as_string() {
                                        let _ = creq.set(
                                            js_string!("__url"),
                                            js_string!(s.to_std_string_escaped()),
                                            false,
                                            ctx,
                                        );
                                    } else if let Some(obj) = opts.as_object() {
                                        // { hostname, port, path, method, headers }
                                        if let Ok(method) = obj.get(js_string!("method"), ctx) {
                                            if !method.is_undefined() {
                                                let _ = creq.set(
                                                    js_string!("__method"),
                                                    method,
                                                    false,
                                                    ctx,
                                                );
                                            }
                                        }
                                        if let Ok(hostname) = obj.get(js_string!("hostname"), ctx) {
                                            let port_val = obj
                                                .get(js_string!("port"), ctx)
                                                .ok()
                                                .and_then(|v| v.as_number())
                                                .unwrap_or(80.0)
                                                as u16;
                                            let path_val = obj
                                                .get(js_string!("path"), ctx)
                                                .ok()
                                                .and_then(|v| v.to_string(ctx).ok())
                                                .map(|s| s.to_std_string_escaped())
                                                .unwrap_or_else(|| "/".to_string());
                                            if let Some(h) = hostname.as_string() {
                                                let url = format!(
                                                    "http://{}:{}{}",
                                                    h.to_std_string_escaped(),
                                                    port_val,
                                                    path_val
                                                );
                                                let _ = creq.set(
                                                    js_string!("__url"),
                                                    js_string!(url),
                                                    false,
                                                    ctx,
                                                );
                                            }
                                        }
                                        if let Ok(hdr_obj) = obj.get(js_string!("headers"), ctx) {
                                            if let Some(_h) = hdr_obj.as_object() {
                                                let _ = creq.set(
                                                    js_string!("__headers"),
                                                    hdr_obj,
                                                    false,
                                                    ctx,
                                                );
                                            }
                                        }
                                    }
                                }

                                // write(chunk, encoding, cb)
                                let _ = creq.set(
                                    js_string!("write"),
                                    build_fn(
                                        make_native(
                                            |this: &JsValue,
                                             args: &[JsValue],
                                             ctx: &mut Context| -> JsResult<JsValue> {
                                                let inst = get_obj(this)?;
                                                if let Some(data) = args.first() {
                                                    let s = data
                                                        .to_string(ctx)
                                                        .ok()
                                                        .map(|s| {
                                                            s.to_std_string_escaped()
                                                        })
                                                        .unwrap_or_default();
                                                    if let Some(buf) = inst
                                                        .get(js_string!("__buffer"), ctx)
                                                        .ok()
                                                        .and_then(|v| v.as_object())
                                                        .and_then(|o| {
                                                            JsArray::from_object(
                                                                o.clone(),
                                                            )
                                                            .ok()
                                                        })
                                                    {
                                                        let _ = buf.push(
                                                            JsValue::from(js_string!(s)),
                                                            ctx,
                                                        );
                                                    }
                                                }
                                                if let Some(cb) = args
                                                    .get(1)
                                                    .or_else(|| args.get(2))
                                                    .and_then(|v| {
                                                        v.as_object()
                                                    })
                                                    .filter(|o| o.is_callable())
                                                {
                                                    let _ = cb.call(
                                                        &JsValue::undefined(),
                                                        &[],
                                                        ctx,
                                                    );
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
                                let _ = creq.set(
                                    js_string!("end"),
                                    build_fn(
                                        make_native(
                                            |this: &JsValue,
                                             end_args: &[JsValue],
                                             ctx: &mut Context| -> JsResult<JsValue> {
                                                let inst = get_obj(this)?;

                                                let mut body_parts: Vec<Vec<u8>> = Vec::new();
                                                if let Ok(buf_val) =
                                                    inst.get(js_string!("__buffer"), ctx)
                                                {
                                                    if let Some(buf_obj) =
                                                        buf_val.as_object()
                                                    {
                                                        if let Ok(buf_arr) =
                                                            JsArray::from_object(
                                                                buf_obj.clone(),
                                                            )
                                                        {
                                                            for i in 0..buf_arr
                                                                .length(ctx)
                                                                .unwrap_or(0)
                                                            {
                                                                if let Ok(item) =
                                                                    buf_arr.get(i, ctx)
                                                                {
                                                                    if let Some(s) =
                                                                        item.as_string()
                                                                    {
                                                                        body_parts.push(
                                                                            s.to_std_string_escaped()
                                                                                .into_bytes(),
                                                                        );
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                                if let Some(data) = end_args.first() {
                                                    if let Some(s) = data.as_string() {
                                                        body_parts.push(
                                                            s.to_std_string_escaped()
                                                                .into_bytes(),
                                                        );
                                                    } else if let Ok(s) =
                                                        data.to_string(ctx)
                                                    {
                                                        body_parts.push(
                                                            s.to_std_string_escaped()
                                                                .into_bytes(),
                                                        );
                                                    }
                                                }
                                                let body: Vec<u8> =
                                                    body_parts.into_iter().flatten().collect();

                                                let method = inst
                                                    .get(js_string!("__method"), ctx)
                                                    .ok()
                                                    .and_then(|v| v.to_string(ctx).ok())
                                                    .map(|s| {
                                                        s.to_std_string_escaped()
                                                    })
                                                    .unwrap_or_else(|| "GET".to_string());
                                                let url = inst
                                                    .get(js_string!("__url"), ctx)
                                                    .ok()
                                                    .and_then(|v| v.to_string(ctx).ok())
                                                    .map(|s| {
                                                        s.to_std_string_escaped()
                                                    })
                                                    .unwrap_or_default();

                                                let callback_val = inst
                                                    .get(js_string!("__callback"), ctx)
                                                    .ok();

                                                // Build request headers
                                                let mut req_headers: Vec<(String, String)> =
                                                    Vec::new();
                                                if let Ok(hdr_val) =
                                                    inst.get(js_string!("__headers"), ctx)
                                                {
                                                    if let Some(hdr_obj) =
                                                        hdr_val.as_object()
                                                    {
                                                        for key in hdr_obj
                                                            .own_property_keys(ctx)
                                                            .unwrap_or_default()
                                                        {
                                                            if let Ok(val) =
                                                                hdr_obj.get(key.clone(), ctx)
                                                            {
                                                                if let Some(v) =
                                                                    val.as_string()
                                                                {
                                                                    req_headers.push((
                                                                        key.to_string(),
                                                                        v.to_std_string_escaped(),
                                                                    ));
                                                                }
                                                            }
                                                        }
                                                    }
                                                }

                                                // Execute via reqwest
                                                let result = (|| -> Result<String, String> {
                                                    let client = reqwest::blocking::Client::builder()
                                                        .danger_accept_invalid_certs(true)
                                                        .build()
                                                        .map_err(|e| e.to_string())?;

                                                    let mut rb = client
                                                        .request(
                                                            reqwest::Method::from_bytes(
                                                                method.as_bytes(),
                                                            )
                                                            .map_err(|e| e.to_string())?,
                                                            &url,
                                                        );

                                                    for (k, v) in &req_headers {
                                                        rb = rb.header(k.as_str(), v.as_str());
                                                    }
                                                    if !body.is_empty() {
                                                        rb = rb.body(body.clone());
                                                    }

                                                    let resp = rb
                                                        .send()
                                                        .map_err(|e| format!("request failed: {e}"))?;

                                                    let status_code = resp.status().as_u16();
                                                    let status_msg = resp
                                                        .status()
                                                        .canonical_reason()
                                                        .unwrap_or("Unknown")
                                                        .to_string();

                                                    let mut resp_headers: Vec<(String, String)> =
                                                        Vec::new();
                                                    for (k, v) in resp.headers() {
                                                        if let Ok(val) = v.to_str() {
                                                            resp_headers.push((
                                                                k.as_str().to_string(),
                                                                val.to_string(),
                                                            ));
                                                        }
                                                    }

                                                    let resp_body = resp
                                                        .bytes()
                                                        .map_err(|e| {
                                                            format!("read body: {e}")
                                                        })?
                                                        .to_vec();

                                                    // Encode as JSON for transport back to JS
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
                                                        // Create response object
                                                        let res_inner = JsObject::with_object_proto(
                                                            ctx.intrinsics(),
                                                        );
                                                        let _ = res_inner.set(
                                                            js_string!("_events"),
                                                            JsValue::from(
                                                                JsObject::with_object_proto(
                                                                    ctx.intrinsics(),
                                                                ),
                                                            ),
                                                            false,
                                                            ctx,
                                                        );

                                                        // Parse JSON response
                                                        if let Ok(parsed) = serde_json::from_str::<
                                                            serde_json::Value,
                                                        >(&json_str)
                                                        {
                                                            if let Some(sc) = parsed["statusCode"]
                                                                .as_u64()
                                                            {
                                                                let _ = res_inner.set(
                                                                    js_string!("statusCode"),
                                                                    JsValue::from(sc as f64),
                                                                    false,
                                                                    ctx,
                                                                );
                                                            }
                                                            if let Some(sm) = parsed[
                                                                "statusMessage"
                                                            ]
                                                                .as_str()
                                                            {
                                                                let _ = res_inner.set(
                                                                    js_string!("statusMessage"),
                                                                    js_string!(sm),
                                                                    false,
                                                                    ctx,
                                                                );
                                                            }
                                                            // Set headers
                                                            let hdr_obj = JsObject::with_object_proto(
                                                                ctx.intrinsics(),
                                                            );
                                                            if let Some(hdrs) =
                                                                parsed["headers"].as_array()
                                                            {
                                                                for h in hdrs {
                                                                    if let Some(arr) = h.as_array()
                                                                    {
                                                                        if arr.len() >= 2 {
                                                                            let k = arr[0]
                                                                                .as_str()
                                                                                .unwrap_or("");
                                                                            let v = arr[1]
                                                                                .as_str()
                                                                                .unwrap_or("");
                                                                            let _ = hdr_obj.set(
                                                                                js_string!(k.to_lowercase()),
                                                                                js_string!(v),
                                                                                false,
                                                                                ctx,
                                                                            );
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                            let _ = res_inner.set(
                                                                js_string!("headers"),
                                                                JsValue::from(hdr_obj),
                                                                false,
                                                                ctx,
                                                            );

                                                            // on(event, cb)
                                                            let res_on = build_fn(
                                                                make_native(
                                                                    |this: &JsValue,
                                                                     args: &[JsValue],
                                                                     ctx: &mut Context| -> JsResult<JsValue> {
                                                                        if let Some(inst) =
                                                                            this.as_object()
                                                                        {
                                                                            let name = args
                                                                                .first()
                                                                                .and_then(|v| {
                                                                                    v.to_string(ctx)
                                                                                        .ok()
                                                                                })
                                                                                .map(|s| {
                                                                                    s.to_std_string_escaped()
                                                                                })
                                                                                .unwrap_or_default();
                                                                            if let Some(listener) =
                                                                                args.get(1)
                                                                            {
                                                                                let _ = add_listener(
                                                                                    &inst,
                                                                                    &name,
                                                                                    listener,
                                                                                    ctx,
                                                                                );
                                                                            }
                                                                        }
                                                                        Ok(this.clone())
                                                                    },
                                                                ),
                                                                "on",
                                                                2,
                                                                ctx,
                                                            );
                                                            let _ = res_inner.set(
                                                                js_string!("on"),
                                                                res_on,
                                                                false,
                                                                ctx,
                                                            );

                                                            // Call user callback with response
                                                            if let Some(body_str) =
                                                                parsed["body"].as_str()
                                                            {
                                                                if !body_str.is_empty() {
                                                                    emit(
                                                                        &res_inner,
                                                                        "data",
                                                                        &[JsValue::from(
                                                                            js_string!(body_str),
                                                                        )],
                                                                        ctx,
                                                                    );
                                                                }
                                                            }
                                                            emit(&res_inner, "end", &[], ctx);

                                                            if let Some(cb) = &callback_val {
                                                                if let Some(cb_fn) =
                                                                    cb.as_object().filter(|o| {
                                                                        o.is_callable()
                                                                    })
                                                                {
                                                                    let _ = cb_fn.call(
                                                                        &JsValue::undefined(),
                                                                        &[JsValue::from(
                                                                            res_inner,
                                                                        )],
                                                                        ctx,
                                                                    );
                                                                }
                                                            }
                                                        }
                                                    }
                                                    Err(e) => {
                                                        if let Some(cb) = &callback_val {
                                                            if let Some(cb_fn) = cb
                                                                .as_object()
                                                                .filter(|o| o.is_callable())
                                                            {
                                                                let _ = cb_fn.call(
                                                                    &JsValue::undefined(),
                                                                    &[JsValue::from(
                                                                        js_string!(e),
                                                                    )],
                                                                    ctx,
                                                                );
                                                            }
                                                        }
                                                    }
                                                }

                                                let _ = inst.set(
                                                    js_string!("__ended"),
                                                    JsValue::from(true),
                                                    false,
                                                    ctx,
                                                );
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

                                Ok(JsValue::from(creq))
                            },
                        ),
                        "request",
                        2,
                        ctx,
                    );

                    // ── get(url, callback) — standalone HTTP GET ──────────────
                    let http_get = build_fn(
                        make_native(
                            |_: &JsValue,
                             args: &[JsValue],
                             ctx: &mut Context|
                             -> JsResult<JsValue> {
                                let opts = args.first().cloned().unwrap_or(JsValue::undefined());
                                let callback = args.get(1).cloned().unwrap_or(JsValue::undefined());

                                // Parse URL
                                let url = if let Some(s) = opts.as_string() {
                                    s.to_std_string_escaped()
                                } else if let Some(obj) = opts.as_object() {
                                    let hostname = obj
                                        .get(js_string!("hostname"), ctx)
                                        .ok()
                                        .and_then(|v| v.to_string(ctx).ok())
                                        .map(|s| s.to_std_string_escaped())
                                        .unwrap_or_else(|| "localhost".to_string());
                                    let port = obj
                                        .get(js_string!("port"), ctx)
                                        .ok()
                                        .and_then(|v| v.as_number())
                                        .unwrap_or(80.0)
                                        as u16;
                                    let path = obj
                                        .get(js_string!("path"), ctx)
                                        .ok()
                                        .and_then(|v| v.to_string(ctx).ok())
                                        .map(|s| s.to_std_string_escaped())
                                        .unwrap_or_else(|| "/".to_string());
                                    let proto = if port == 443 { "https" } else { "http" };
                                    format!("{proto}://{hostname}:{port}{path}")
                                } else {
                                    return Err(JsNativeError::typ()
                                        .with_message("http.get: invalid URL")
                                        .into());
                                };

                                // Make GET request via reqwest
                                let client = reqwest::blocking::Client::builder()
                                    .danger_accept_invalid_certs(true)
                                    .build()
                                    .map_err(|e| {
                                        JsNativeError::typ().with_message(format!("http.get: {e}"))
                                    })?;

                                match client.get(&url).send() {
                                    Ok(resp) => {
                                        let res = create_response_from_reqwest(resp, ctx)?;
                                        if let Some(cb) =
                                            callback.as_object().filter(|o| o.is_callable())
                                        {
                                            let _ = cb.call(
                                                &JsValue::undefined(),
                                                &[JsValue::from(res.clone())],
                                                ctx,
                                            );
                                        }
                                        // Emit body events after callback sets up listeners
                                        if let Ok(body_val) = res.get(js_string!("__body"), ctx) {
                                            if let Some(arr_obj) = body_val.as_object() {
                                                if let Ok(arr) =
                                                    JsArray::from_object(arr_obj.clone())
                                                {
                                                    for i in 0..arr.length(ctx).unwrap_or(0) {
                                                        if let Ok(item) = arr.get(i, ctx) {
                                                            emit(&res, "data", &[item], ctx);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        emit(&res, "end", &[], ctx);
                                        Ok(JsValue::from(res))
                                    }
                                    Err(e) => Err(JsNativeError::typ()
                                        .with_message(format!("http.get failed: {e}"))
                                        .into()),
                                }
                            },
                        ),
                        "get",
                        2,
                        ctx,
                    );

                    // Set exports
                    let _ = m.set_export(&js_string!("createServer"), create_server.clone());
                    let _ = m.set_export(&js_string!("request"), http_request.clone());
                    let _ = m.set_export(&js_string!("get"), http_get.clone());
                    let sc_for_default = status_codes.clone();
                    let _ = m.set_export(&js_string!("STATUS_CODES"), JsValue::from(status_codes));

                    let default_obj = JsObject::with_object_proto(ctx.intrinsics());
                    let _ = default_obj.set(
                        js_string!("createServer"),
                        create_server.clone(),
                        false,
                        ctx,
                    );
                    let _ = default_obj.set(js_string!("request"), http_request, false, ctx);
                    let _ = default_obj.set(js_string!("get"), http_get, false, ctx);
                    let _ = default_obj.set(
                        js_string!("STATUS_CODES"),
                        JsValue::from(sc_for_default),
                        false,
                        ctx,
                    );
                    let _ = m.set_export(&js_string!("default"), JsValue::from(default_obj));
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
