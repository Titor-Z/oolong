use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};

use boa_engine::module::SyntheticModuleInitializer;
use boa_engine::object::builtins::{JsArray, JsPromise};
use boa_engine::object::FunctionObjectBuilder;
use boa_engine::{
    Context, JsError, JsNativeError, JsObject, JsResult, JsString, JsValue, Module, NativeFunction, js_string,
};

fn make_native<F>(f: F) -> NativeFunction
where
    F: Fn(&JsValue, &[JsValue], &mut Context) -> JsResult<JsValue> + 'static,
{
    unsafe { NativeFunction::from_closure(f) }
}

fn build_fn(f: NativeFunction, name: &str, len: usize, ctx: &mut Context) -> JsValue {
    FunctionObjectBuilder::new(ctx.realm(), f).name(name).length(len).build().into()
}

fn get_obj(v: &JsValue) -> JsResult<JsObject> {
    v.as_object().ok_or_else(|| JsNativeError::typ().with_message("not an object").into())
}

fn status_text(code: u16) -> &'static str {
    match code {
        200 => "OK", 201 => "Created", 204 => "No Content",
        301 => "Moved Permanently", 302 => "Found", 304 => "Not Modified",
        400 => "Bad Request", 401 => "Unauthorized", 403 => "Forbidden",
        404 => "Not Found", 405 => "Method Not Allowed", 408 => "Request Timeout",
        500 => "Internal Server Error", 502 => "Bad Gateway", 503 => "Service Unavailable",
        _ => if code < 400 { "Unknown" } else { "Error" }
    }
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
                    // STATUS_CODES 对象
                    let status_codes = JsObject::with_object_proto(ctx.intrinsics());
                    for &(code, text) in &[
                        (100, "Continue"), (101, "Switching Protocols"),
                        (200, "OK"), (201, "Created"), (202, "Accepted"),
                        (204, "No Content"), (206, "Partial Content"),
                        (301, "Moved Permanently"), (302, "Found"), (303, "See Other"),
                        (304, "Not Modified"), (307, "Temporary Redirect"),
                        (400, "Bad Request"), (401, "Unauthorized"), (403, "Forbidden"),
                        (404, "Not Found"), (405, "Method Not Allowed"), (408, "Request Timeout"),
                        (409, "Conflict"), (410, "Gone"), (411, "Length Required"),
                        (413, "Payload Too Large"), (415, "Unsupported Media Type"),
                        (429, "Too Many Requests"),
                        (500, "Internal Server Error"), (501, "Not Implemented"),
                        (502, "Bad Gateway"), (503, "Service Unavailable"),
                        (504, "Gateway Timeout"),
                    ] {
                        let _ = status_codes.set(JsString::from(code.to_string()), js_string!(text), false, ctx);
                    }

                    let create_server = build_fn(make_native(|_: &JsValue, args: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
                        let request_listener = args.first().cloned().unwrap_or(JsValue::undefined());

                        let server = JsObject::with_object_proto(ctx.intrinsics());
                        let _ = server.set(js_string!("__listening"), JsValue::from(false), false, ctx);

                        // close(cb)
                        let _ = server.set(js_string!("close"), build_fn(make_native(|this: &JsValue, args: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
                            let inst = get_obj(this)?;
                            let _ = inst.set(js_string!("__listening"), JsValue::from(false), false, ctx);
                            if let Some(cb) = args.first().and_then(|v| v.as_object()).filter(|o| o.is_callable()) {
                                let _ = cb.call(&JsValue::undefined(), &[], ctx);
                            }
                            Ok(JsValue::undefined())
                        }), "close", 1, ctx), false, ctx);

                        // listen(port, host, cb)
                        let listen_fn = make_native(|this: &JsValue, args: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
                            let port = args.first().and_then(|v| v.as_number()).unwrap_or(0.0) as u16;
                            let host_arg = args.get(1).and_then(|v| v.to_string(ctx).ok()).map(|s| s.to_std_string_escaped());
                            let cb_val = if host_arg.is_some() { args.get(2) } else { args.get(1) };
                            let host = host_arg.unwrap_or_else(|| "127.0.0.1".to_string());

                            let listener = TcpListener::bind(format!("{host}:{port}")).map_err(|e| -> JsError {
                                JsNativeError::typ().with_message(format!("EADDRINUSE port={port}: {e}")).into()
                            })?;
                            let addr = listener.local_addr().map_err(|e| -> JsError {
                                JsNativeError::typ().with_message(format!("addr error: {e}")).into()
                            })?;

                            let inst = get_obj(this)?;
                            let _ = inst.set(js_string!("__listening"), JsValue::from(true), false, ctx);

                            if let Some(cb) = cb_val {
                                if let Some(cb_fn) = cb.as_object().filter(|o| o.is_callable()) {
                                    let _ = cb_fn.call(&JsValue::undefined(), &[], ctx);
                                }
                            }

                            let realm = ctx.realm().clone();

                            // 同步接受连接 + 处理请求
                            for stream in listener.incoming() {
                                let _listening = inst.get(js_string!("__listening"), ctx)
                                    .ok().and_then(|v| v.as_boolean()).unwrap_or(false);
                                if !_listening { break; }

                                match stream {
                                    Ok(mut tcp_stream) => {
                                        let peer = tcp_stream.peer_addr().ok();
                                        let mut reader = BufReader::new(&mut tcp_stream);

                                        // 解析 HTTP 请求行
                                        let mut request_line = String::new();
                                        if reader.read_line(&mut request_line).is_err() || request_line.trim().is_empty() {
                                            continue;
                                        }
                                        let parts: Vec<&str> = request_line.split_whitespace().collect();
                                        if parts.len() < 3 { continue; }
                                        let method = parts[0].to_string();
                                        let url = parts[1].to_string();

                                        // 解析请求头
                                        let mut headers_vec: Vec<(String, String)> = Vec::new();
                                        loop {
                                            let mut line = String::new();
                                            if reader.read_line(&mut line).is_err() { break; }
                                            let trimmed = line.trim();
                                            if trimmed.is_empty() { break; }
                                            if let Some(pos) = trimmed.find(':') {
                                                let k = trimmed[..pos].trim().to_string();
                                                let v = trimmed[pos + 1..].trim().to_string();
                                                headers_vec.push((k, v));
                                            }
                                        }

                                        // 读取 body
                                        let body: Vec<u8> = {
                                            let cl = headers_vec.iter()
                                                .find(|(k, _)| k.eq_ignore_ascii_case("content-length"))
                                                .and_then(|(_, v)| v.parse::<usize>().ok()).unwrap_or(0);
                                            let mut b = vec![0u8; cl];
                                            if cl > 0 { let _ = reader.read_exact(&mut b); }
                                            b
                                        };

                                        // 创建 req 对象
                                        let req = JsObject::with_object_proto(ctx.intrinsics());
                                        let _ = req.set(js_string!("method"), js_string!(method.as_str()), false, ctx);
                                        let _ = req.set(js_string!("url"), js_string!(url.as_str()), false, ctx);
                                        let _ = req.set(js_string!("httpVersion"), js_string!("1.1"), false, ctx);
                                        let _ = req.set(js_string!("httpVersionMajor"), JsValue::from(1), false, ctx);
                                        let _ = req.set(js_string!("httpVersionMinor"), JsValue::from(1), false, ctx);
                                        let _ = req.set(js_string!("socket"), JsValue::undefined(), false, ctx);
                                        if let Some(p) = peer {
                                            let _ = req.set(js_string!("remoteAddress"), js_string!(p.ip().to_string()), false, ctx);
                                            let _ = req.set(js_string!("remotePort"), JsValue::from(p.port() as f64), false, ctx);
                                        }

                                        // headers 对象
                                        let headers_obj = JsObject::with_object_proto(ctx.intrinsics());
                                        for (k, v) in &headers_vec {
                                            let _ = headers_obj.set(js_string!(k.to_lowercase()), js_string!(v.as_str()), false, ctx);
                                        }
                                        let _ = req.set(js_string!("headers"), JsValue::from(headers_obj), false, ctx);

                                        // rawHeaders
                                        let raw = JsArray::new(ctx);
                                        for (k, v) in &headers_vec {
                                            let _ = raw.push(JsString::from(k.as_str()), ctx);
                                            let _ = raw.push(JsString::from(v.as_str()), ctx);
                                        }
                                        let _ = req.set(js_string!("rawHeaders"), JsValue::from(raw), false, ctx);

                                        // body 可读流
                                        let req_body = JsObject::with_object_proto(ctx.intrinsics());
                                        let _ = req_body.set(js_string!("_readable"), JsValue::from(true), false, ctx);
                                        let _ = req.set(js_string!("_body"), JsValue::from(req_body), false, ctx);

                                        // 创建 res 对象
                                        let res = JsObject::with_object_proto(ctx.intrinsics());
                                        let _ = res.set(js_string!("statusCode"), JsValue::from(200), false, ctx);
                                        let _ = res.set(js_string!("statusMessage"), js_string!("OK"), false, ctx);
                                        let _ = res.set(js_string!("__headers_sent"), JsValue::from(false), false, ctx);
                                        let _ = res.set(js_string!("__headers"), JsValue::from(JsObject::with_object_proto(ctx.intrinsics())), false, ctx);

                                        // setHeader / getHeader / getHeaders / removeHeader
                                        let _ = res.set(js_string!("setHeader"), build_fn(make_native(|this: &JsValue, args: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
                                            let inst = get_obj(this)?;
                                            let name = args.first().and_then(|v| v.to_string(ctx).ok()).map(|s| s.to_std_string_escaped()).unwrap_or_default();
                                            let value = args.get(1).cloned().unwrap_or(JsValue::undefined());
                                            if let Some(headers) = inst.get(js_string!("__headers"), ctx).ok()
                                                .and_then(|v| v.as_object()).map(|o| o.clone()) {
                                                let _ = headers.set(js_string!(name.to_lowercase()), value, false, ctx);
                                            }
                                            Ok(this.clone())
                                        }), "setHeader", 2, ctx), false, ctx);

                                        let _ = res.set(js_string!("getHeader"), build_fn(make_native(|this: &JsValue, args: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
                                            let inst = get_obj(this)?;
                                            let name = args.first().and_then(|v| v.to_string(ctx).ok()).map(|s| s.to_std_string_escaped()).unwrap_or_default();
                                            if let Ok(headers) = inst.get(js_string!("__headers"), ctx) {
                                                if let Some(h) = headers.as_object() {
                                                    if let Ok(val) = h.get(js_string!(name.to_lowercase()), ctx) {
                                                        return Ok(val);
                                                    }
                                                }
                                            }
                                            Ok(JsValue::undefined())
                                        }), "getHeader", 1, ctx), false, ctx);

                                        // writeHead(status, statusText, headers)
                                        let _ = res.set(js_string!("writeHead"), build_fn(make_native(|this: &JsValue, args: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
                                            let inst = get_obj(this)?;
                                            if let Some(sc) = args.first().and_then(|v| v.as_number()) {
                                                let _ = inst.set(js_string!("statusCode"), JsValue::from(sc), false, ctx);
                                                let _ = inst.set(js_string!("statusMessage"), js_string!(status_text(sc as u16)), false, ctx);
                                            }
                                            if let Some(msg) = args.get(1).and_then(|v| v.to_string(ctx).ok()) {
                                                let _ = inst.set(js_string!("statusMessage"), msg, false, ctx);
                                            }
                                            if let Some(headers) = args.get(1).or_else(|| args.get(2)).and_then(|v| v.as_object()) {
                                                if let Some(h) = inst.get(js_string!("__headers"), ctx).ok()
                                                    .and_then(|v| v.as_object()).map(|o| o.clone()) {
                                                    for key in headers.own_property_keys(ctx).unwrap_or_default() {
                                                        if let Ok(val) = headers.get(key.clone(), ctx) {
                                                            let key_str = key.to_string();
                                                            let _ = h.set(js_string!(key_str.to_lowercase()), val, false, ctx);
                                                        }
                                                    }
                                                }
                                            }
                                            let _ = inst.set(js_string!("__headers_sent"), JsValue::from(true), false, ctx);
                                            Ok(JsValue::undefined())
                                        }), "writeHead", 3, ctx), false, ctx);

                                        // write(chunk, encoding, cb)
                                        let _ = res.set(js_string!("write"), build_fn(make_native(|_: &JsValue, _args: &[JsValue], _ctx: &mut Context| -> JsResult<JsValue> {
                                            Ok(JsValue::from(true))
                                        }), "write", 2, ctx), false, ctx);

                                        // end(data, encoding, cb)
                                        let res_clone = res.clone();
                                        let req_clone = req.clone();
                                        let _ = res.set(js_string!("end"), build_fn(make_native(move |_: &JsValue, end_args: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
                                            // 收集 body
                                            let mut body_bytes = Vec::new();
                                            if let Some(data) = end_args.first() {
                                                if let Some(s) = data.as_string() {
                                                    body_bytes = s.to_std_string_escaped().into_bytes();
                                                }
                                            }
                                            let data = body_bytes;

                                            // 构建响应
                                            let sc = res_clone.get(js_string!("statusCode"), ctx).ok().and_then(|v| v.as_number()).unwrap_or(200.0) as u16;
                                            let sm = res_clone.get(js_string!("statusMessage"), ctx).ok()
                                                .and_then(|v| v.to_string(ctx).ok()).map(|s| s.to_std_string_escaped()).unwrap_or_else(|| status_text(sc).to_string());

                                            let mut response = format!("HTTP/1.1 {sc} {sm}\r\n");
                                            response.push_str("Content-Length: ");
                                            response.push_str(&data.len().to_string());
                                            response.push_str("\r\n");
                                            response.push_str("Connection: close\r\n");

                                            if let Ok(headers) = res_clone.get(js_string!("__headers"), ctx) {
                                                if let Some(h) = headers.as_object() {
                                                    for key in h.own_property_keys(ctx).unwrap_or_default() {
                                                        if let Ok(val) = h.get(key.clone(), ctx) {
                                                            if let Some(v) = val.as_string() {
                                                                let ks = key.to_string();
                                                                response.push_str(&ks);
                                                                response.push_str(": ");
                                                                response.push_str(&v.to_std_string_escaped());
                                                                response.push_str("\r\n");
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            response.push_str("\r\n");
                                            if !data.is_empty() {
                                                response.push_str(&String::from_utf8_lossy(&data));
                                            }

                                            if let Some(cb) = end_args.get(1).or_else(|| end_args.get(2))
                                                .and_then(|v| v.as_object().filter(|o| o.is_callable()))
                                            {
                                                let _ = cb.call(&JsValue::undefined(), &[], ctx);
                                            }

                                            Ok(JsValue::from(true))
                                        }), "end", 2, ctx), false, ctx);

                                        // 调用 handler
                                        let handler = inst.get(js_string!("__handler"), ctx).ok()
                                            .and_then(|v| v.as_object().filter(|o| o.is_callable()));

                                        if let Some(ref handler_fn) = handler {
                                            let _ = handler_fn.call(&JsValue::undefined(), &[JsValue::from(req), JsValue::from(res.clone())], ctx);
                                        }

                                        // 发送响应
                                        let body_out: Vec<u8> = Vec::new();
                                        let sc = res.get(js_string!("statusCode"), ctx).ok().and_then(|v| v.as_number()).unwrap_or(200.0) as u16;
                                        let sm = res.get(js_string!("statusMessage"), ctx).ok()
                                            .and_then(|v| v.to_string(ctx).ok()).map(|s| s.to_std_string_escaped()).unwrap_or_else(|| status_text(sc).to_string());

                                        let mut resp = format!("HTTP/1.1 {sc} {sm}\r\n");
                                        resp.push_str(&format!("Content-Length: {}\r\n", body_out.len()));
                                        resp.push_str("Connection: close\r\n");

                                        if let Ok(hdrs) = res.get(js_string!("__headers"), ctx) {
                                            if let Some(h) = hdrs.as_object() {
                                                for key in h.own_property_keys(ctx).unwrap_or_default() {
                                                    if let Ok(val) = h.get(key.clone(), ctx) {
                                                        if let Some(v) = val.as_string() {
                                                            let ks = key.to_string();
                                                            resp.push_str(&format!("{}: {}\r\n", ks, v.to_std_string_escaped()));
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        resp.push_str("\r\n");
                                        let _ = tcp_stream.write_all(resp.as_bytes());
                                    }
                                    Err(_) => break,
                                }
                            }

                            Ok(JsValue::undefined())
                        });
                        let listen_val = FunctionObjectBuilder::new(ctx.realm(), listen_fn).name("listen").length(2).build();
                        let _ = server.set(js_string!("listen"), JsValue::from(listen_val), false, ctx);

                        // 存储 handler
                        if !request_listener.is_undefined() {
                            let _ = server.set(js_string!("__handler"), request_listener, false, ctx);
                        }

                        Ok(JsValue::from(server))
                    }), "createServer", 1, ctx);

                    let sc_for_default = status_codes.clone();
                    let default_obj = JsObject::with_object_proto(ctx.intrinsics());
                    let _ = default_obj.set(js_string!("createServer"), create_server.clone(), false, ctx);
                    let _ = default_obj.set(js_string!("STATUS_CODES"), JsValue::from(sc_for_default), false, ctx);
                    let _ = m.set_export(&js_string!("createServer"), create_server.clone());
                    let _ = m.set_export(&js_string!("STATUS_CODES"), JsValue::from(status_codes));
                    let _ = m.set_export(&js_string!("default"), JsValue::from(default_obj));
                    Ok(())
                },
            )
        },
        None, None, context,
    );
    Ok(module)
}
