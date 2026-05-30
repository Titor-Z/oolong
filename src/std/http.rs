use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};

use boa_engine::class::Class;
use boa_engine::module::SyntheticModuleInitializer;
use boa_engine::object::builtins::{JsFunction, JsPromise};
use boa_engine::{
    Context, IntoJsFunctionCopied, JsError, JsNativeError, JsObject, JsResult, JsString, JsValue,
    Module, js_string, object::FunctionObjectBuilder,
};

type HttpHeaders = Vec<(String, String)>;

fn to_js_fn(f: boa_engine::NativeFunction, name: &str, len: usize, ctx: &mut Context) -> JsValue {
    FunctionObjectBuilder::new(ctx.realm(), f)
        .name(JsString::from(name))
        .length(len)
        .build()
        .into()
}

fn parse_http_request(
    reader: &mut BufReader<&mut TcpStream>,
) -> Result<(String, String, HttpHeaders), String> {
    let mut request_line = String::new();
    reader
        .read_line(&mut request_line)
        .map_err(|e| format!("读取请求行失败: {e}"))?;

    let parts: Vec<&str> = request_line.split_whitespace().collect();
    let method = parts.first().unwrap_or(&"GET").to_string();
    let path = parts.get(1).unwrap_or(&"/").to_string();

    let mut headers = Vec::new();
    loop {
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .map_err(|e| format!("读取头部失败: {e}"))?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            break;
        }
        if let Some(pos) = trimmed.find(':') {
            let key = trimmed[..pos].trim().to_string();
            let val = trimmed[pos + 1..].trim().to_string();
            headers.push((key, val));
        }
    }

    Ok((method, path, headers))
}

fn read_body(
    reader: &mut BufReader<&mut TcpStream>,
    headers: &HttpHeaders,
) -> Result<Vec<u8>, String> {
    let content_length = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, v)| v.parse::<usize>().ok())
        .unwrap_or(0);

    let mut body = vec![0u8; content_length];
    if content_length > 0 {
        reader
            .read_exact(&mut body)
            .map_err(|e| format!("读取 body 失败: {e}"))?;
    }
    Ok(body)
}

fn create_request_obj(
    method: &str,
    path: &str,
    headers: &[(String, String)],
    body: &[u8],
    ctx: &mut Context,
) -> JsValue {
    let mut js_headers = crate::web::headers::JsHeaders::new();
    for (k, v) in headers {
        let _ = js_headers.append(js_string!(k.as_str()), js_string!(v.as_str()));
    }

    let request = crate::web::request::JsRequest::new(
        js_string!(method),
        js_string!(path),
        js_headers,
        body.to_vec(),
    );
    Class::from_data(request, ctx).unwrap().into()
}

fn resolve_value(val: JsValue, ctx: &mut Context) -> JsResult<JsValue> {
    if let Some(obj) = val.as_object()
        && let Ok(promise) = JsPromise::from_object(obj.clone())
    {
        return promise.await_blocking(ctx);
    }
    Ok(val)
}

fn extract_status(obj: &JsObject, ctx: &mut Context) -> u16 {
    obj.get(js_string!("status"), ctx)
        .ok()
        .and_then(|v| v.as_number())
        .map(|n| n as u16)
        .unwrap_or(200)
}

fn extract_content_type(obj: &JsObject, ctx: &mut Context) -> String {
    if let Ok(headers_val) = obj.get(js_string!("headers"), ctx)
        && let Some(h_obj) = headers_val.as_object()
        && let Some(js_headers) = h_obj.downcast_ref::<crate::web::headers::JsHeaders>()
        && let Ok(ct_val) = js_headers.get(js_string!("content-type"), ctx)
        && let Some(s) = ct_val.as_string()
    {
        return s.to_std_string_escaped();
    }
    "text/plain; charset=utf-8".into()
}

fn try_get_text_body(obj: &JsObject, ctx: &mut Context) -> Option<Vec<u8>> {
    let text_fn = obj.get(js_string!("text"), ctx).ok()?;
    let text_obj = text_fn.as_object()?;
    let f = JsFunction::from_object(text_obj.clone())?;
    let promise_val = f.call(&JsValue::from(obj.clone()), &[], ctx).ok()?;

    let promise_obj = promise_val.as_object()?;
    let promise = JsPromise::from_object(promise_obj.clone()).ok()?;
    let val = promise.await_blocking(ctx).ok()?;
    val.as_string()
        .map(|s| s.to_std_string_escaped().into_bytes())
}

fn try_get_object_body(obj: &JsObject, ctx: &mut Context) -> Option<Vec<u8>> {
    let body_val = obj.get(js_string!("body"), ctx).ok()?;
    body_val
        .as_string()
        .map(|s| s.to_std_string_escaped().into_bytes())
}

fn extract_response(res: &JsValue, ctx: &mut Context) -> Result<(u16, Vec<u8>, String), String> {
    if let Some(s) = res.as_string() {
        return Ok((
            200,
            s.to_std_string_escaped().into_bytes(),
            "text/plain; charset=utf-8".into(),
        ));
    }

    let obj = res
        .as_object()
        .ok_or("handler must return a string, Response, or object")?;
    let obj_ref: &JsObject = &obj;

    let status = extract_status(obj_ref, ctx);
    let content_type = extract_content_type(obj_ref, ctx);

    let body_bytes = try_get_text_body(obj_ref, ctx)
        .or_else(|| try_get_object_body(obj_ref, ctx))
        .unwrap_or_default();

    Ok((status, body_bytes, content_type))
}

fn status_text(status: u16) -> &'static str {
    match status {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        301 => "Moved Permanently",
        302 => "Found",
        304 => "Not Modified",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        408 => "Request Timeout",
        413 => "Payload Too Large",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        _ => "Unknown",
    }
}

fn write_response(
    stream: &mut TcpStream,
    status: u16,
    body: &[u8],
    content_type: &str,
) -> Result<(), String> {
    let response = format!(
        "HTTP/1.1 {status} {}\r\nContent-Length: {}\r\nContent-Type: {}\r\nConnection: close\r\n\r\n",
        status_text(status),
        body.len(),
        content_type
    );

    stream
        .write_all(response.as_bytes())
        .map_err(|e| format!("写入头部失败: {e}"))?;
    if !body.is_empty() {
        stream
            .write_all(body)
            .map_err(|e| format!("写入 body 失败: {e}"))?;
    }
    stream.flush().map_err(|e| format!("刷新失败: {e}"))?;
    Ok(())
}

fn serve_impl(
    port: u16,
    hostname: &str,
    handler: JsFunction,
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let addr = format!("{hostname}:{port}");
    let listener = TcpListener::bind(&addr).map_err(|e| {
        JsError::from(JsNativeError::typ().with_message(format!("无法绑定 {addr}: {e}")))
    })?;

    for stream in listener.incoming() {
        let mut stream = match stream {
            Ok(s) => s,
            Err(e) => {
                eprintln!("接受连接失败: {e}");
                continue;
            }
        };

        let mut buf_reader = BufReader::new(&mut stream);

        let (method, path, headers) = match parse_http_request(&mut buf_reader) {
            Ok(r) => r,
            Err(e) => {
                let _ = write_response(&mut stream, 400, e.as_bytes(), "text/plain");
                continue;
            }
        };

        let body = match read_body(&mut buf_reader, &headers) {
            Ok(b) => b,
            Err(e) => {
                let _ = write_response(&mut stream, 400, e.as_bytes(), "text/plain");
                continue;
            }
        };

        let js_req = create_request_obj(&method, &path, &headers, &body, ctx);
        let js_res_val = match handler.call(&JsValue::undefined(), &[js_req], ctx) {
            Ok(v) => v,
            Err(e) => {
                let _ = write_response(
                    &mut stream,
                    500,
                    format!("handler error: {e}").as_bytes(),
                    "text/plain",
                );
                continue;
            }
        };

        let js_resolved = match resolve_value(js_res_val, ctx) {
            Ok(v) => v,
            Err(e) => {
                let _ = write_response(
                    &mut stream,
                    500,
                    format!("handler promise rejected: {e}").as_bytes(),
                    "text/plain",
                );
                continue;
            }
        };

        let (status, body_bytes, content_type) = match extract_response(&js_resolved, ctx) {
            Ok(r) => r,
            Err(e) => {
                let _ = write_response(&mut stream, 500, e.as_bytes(), "text/plain");
                continue;
            }
        };

        if let Err(e) = write_response(&mut stream, status, &body_bytes, &content_type) {
            eprintln!("响应写入失败: {e}");
        }
    }

    Ok(JsValue::undefined())
}

pub fn create_http_module(context: &mut Context) -> Result<Module, String> {
    let export_names: &[JsString] = &[js_string!("serve"), js_string!("default")];

    let module = Module::synthetic(
        export_names,
        SyntheticModuleInitializer::from_copy_closure(
            |m: &boa_engine::module::SyntheticModule, ctx: &mut Context| {
                let serve_fn = to_js_fn(
                    (|opts: JsValue, ctx: &mut Context| -> JsResult<JsValue> {
                        let opts_obj = opts.as_object().ok_or_else(|| {
                            JsError::from(JsNativeError::typ().with_message(
                                "serve() expects an options object { port, handler }",
                            ))
                        })?;

                        let port_val = opts_obj.get(js_string!("port"), ctx).map_err(|_| {
                            JsError::from(
                                JsNativeError::typ().with_message("serve() missing 'port' option"),
                            )
                        })?;
                        let port = port_val.as_number().ok_or_else(|| {
                            JsError::from(
                                JsNativeError::typ()
                                    .with_message("serve() 'port' must be a number"),
                            )
                        })? as u16;

                        let hostname = opts_obj
                            .get(js_string!("hostname"), ctx)
                            .ok()
                            .and_then(|v| v.as_string().map(|s| s.to_std_string_escaped()))
                            .unwrap_or_else(|| "0.0.0.0".into());

                        let handler_val =
                            opts_obj.get(js_string!("handler"), ctx).map_err(|_| {
                                JsError::from(
                                    JsNativeError::typ()
                                        .with_message("serve() missing 'handler' option"),
                                )
                            })?;
                        let handler_obj = handler_val.as_object().ok_or_else(|| {
                            JsError::from(
                                JsNativeError::typ()
                                    .with_message("serve() 'handler' must be a function"),
                            )
                        })?;
                        let handler =
                            JsFunction::from_object(handler_obj.clone()).ok_or_else(|| {
                                JsError::from(
                                    JsNativeError::typ()
                                        .with_message("serve() 'handler' must be a function"),
                                )
                            })?;

                        serve_impl(port, &hostname, handler, ctx)
                    })
                    .into_js_function_copied(ctx),
                    "serve",
                    1,
                    ctx,
                );
                m.set_export(&js_string!("serve"), serve_fn.clone())?;

                let obj = JsObject::with_object_proto(ctx.intrinsics());
                let _ = obj.set(js_string!("serve"), serve_fn, false, ctx);
                m.set_export(&js_string!("default"), obj.into())?;

                Ok(())
            },
        ),
        None,
        None,
        context,
    );

    Ok(module)
}
