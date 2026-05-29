use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};

type HttpHeaders = Vec<(String, String)>;
type RequestParts = (String, String, HttpHeaders);

use boa_engine::module::SyntheticModuleInitializer;
use boa_engine::object::builtins::JsFunction;
use boa_engine::{
    Context, IntoJsFunctionCopied, JsError, JsNativeError, JsObject, JsResult, JsString, JsValue,
    Module, js_string, object::FunctionObjectBuilder,
};

fn to_js_fn(f: boa_engine::NativeFunction, name: &str, len: usize, ctx: &mut Context) -> JsValue {
    FunctionObjectBuilder::new(ctx.realm(), f)
        .name(JsString::from(name))
        .length(len)
        .build()
        .into()
}

/// 解析 HTTP 请求行和头部，返回 (method, path, headers)
fn parse_http_request(reader: &mut BufReader<&mut TcpStream>) -> Result<RequestParts, String> {
    let mut request_line = String::new();
    reader.read_line(&mut request_line).map_err(|e| format!("读取请求行失败: {e}"))?;

    let parts: Vec<&str> = request_line.split_whitespace().collect();
    let method = parts.first().unwrap_or(&"GET").to_string();
    let path = parts.get(1).unwrap_or(&"/").to_string();

    let mut headers = Vec::new();
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).map_err(|e| format!("读取头部失败: {e}"))?;
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

/// 读取 HTTP body（根据 Content-Length）
fn read_body(reader: &mut BufReader<&mut TcpStream>, headers: &HttpHeaders) -> Result<Vec<u8>, String> {
    let content_length = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, v)| v.parse::<usize>().ok())
        .unwrap_or(0);

    let mut body = vec![0u8; content_length];
    if content_length > 0 {
        reader.read_exact(&mut body).map_err(|e| format!("读取 body 失败: {e}"))?;
    }
    Ok(body)
}

/// 构建 JS 请求对象（plain object）
fn create_request_obj(
    method: &str,
    path: &str,
    headers: &[(String, String)],
    body: &[u8],
    ctx: &mut Context,
) -> JsObject {
    let obj = JsObject::with_object_proto(ctx.intrinsics());
    let _ = obj.set(js_string!("method"), JsValue::from(js_string!(method)), false, ctx);
    let _ = obj.set(js_string!("url"), JsValue::from(js_string!(path)), false, ctx);

    let headers_obj = JsObject::with_object_proto(ctx.intrinsics());
    for (k, v) in headers {
        let _ = headers_obj.set(
            JsString::from(k.as_str()),
            JsValue::from(js_string!(v.as_str())),
            false,
            ctx,
        );
    }
    let _ = obj.set(js_string!("headers"), JsValue::from(headers_obj), false, ctx);

    if !body.is_empty() {
        let body_str = String::from_utf8_lossy(body).to_string();
        let _ = obj.set(js_string!("body"), JsValue::from(js_string!(body_str)), false, ctx);
    }

    obj
}

/// 从 JS Response 对象提取状态码 + body
fn extract_response(res: &JsValue, ctx: &mut Context) -> Result<(u16, Vec<u8>), String> {
    let obj = res.as_object().ok_or("handler must return a Response object")?;

    let status = obj
        .get(js_string!("status"), ctx)
        .ok()
        .and_then(|v| v.as_number())
        .map(|n| n as u16)
        .unwrap_or(200);

    let body_val = obj.get(js_string!("body"), ctx).map_err(|e| format!("获取 body 失败: {e}"))?;

    let body_bytes = if body_val.is_undefined() || body_val.is_null() {
        Vec::new()
    } else if let Some(s) = body_val.as_string() {
        s.to_std_string_escaped().into_bytes()
    } else {
        // Try to call .text() on the body (Response object)
        if let Some(obj) = body_val.as_object()
            && let Ok(text_fn) = obj.get(js_string!("text"), ctx)
            && let Some(text_obj) = text_fn.as_object()
            && let Some(f) = JsFunction::from_object(text_obj.clone())
            && let Ok(text_val) = f.call(&JsValue::undefined(), &[], ctx)
            && let Some(s) = text_val.as_string()
        {
            s.to_std_string_escaped().into_bytes()
        } else {
            Vec::new()
        }
    };

    Ok((status, body_bytes))
}

/// 发送 HTTP 响应
fn write_response(stream: &mut TcpStream, status: u16, body: &[u8], content_type: &str) -> Result<(), String> {
    let status_text = match status {
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
        413 => "Payload Too Large",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        _ => "Unknown",
    };

    let response = format!(
        "HTTP/1.1 {status} {status_text}\r\nContent-Length: {}\r\nContent-Type: {}\r\nConnection: close\r\n\r\n",
        body.len(),
        content_type
    );

    stream.write_all(response.as_bytes()).map_err(|e| format!("写入头部失败: {e}"))?;
    if !body.is_empty() {
        stream.write_all(body).map_err(|e| format!("写入 body 失败: {e}"))?;
    }
    stream.flush().map_err(|e| format!("刷新失败: {e}"))?;
    Ok(())
}

fn serve_impl(
    port: u16,
    handler: JsFunction,
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let addr = format!("0.0.0.0:{port}");
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

        let _peer = stream.peer_addr().ok();
        let mut buf_reader = BufReader::new(&mut stream);

        let (method, path, headers) = match parse_http_request(&mut buf_reader) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("{e}");
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
        let js_res_val = handler
            .call(&JsValue::undefined(), &[js_req.into()], ctx)
            .map_err(|e| {
                JsError::from(
                    JsNativeError::typ()
                        .with_message(format!("handler error: {e}")),
                )
            })?;

        let (status, body_bytes) = match extract_response(&js_res_val, ctx) {
            Ok(r) => r,
            Err(e) => {
                let _ = write_response(&mut stream, 500, e.as_bytes(), "text/plain");
                continue;
            }
        };

        let content_type = "text/plain; charset=utf-8";
        if let Err(e) = write_response(&mut stream, status, &body_bytes, content_type) {
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
                // ── serve(options) ──────────────────────────────────────────
                // 签名：serve({ port, hostname?, handler })
                let serve_fn = to_js_fn(
                    (|opts: JsValue, ctx: &mut Context| -> JsResult<JsValue> {
                        let opts_obj =
                            opts.as_object().ok_or_else(|| {
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

                        let handler_val = opts_obj.get(js_string!("handler"), ctx).map_err(|_| {
                            JsError::from(JsNativeError::typ().with_message(
                                "serve() missing 'handler' option",
                            ))
                        })?;
                        let handler_obj =
                            handler_val.as_object().ok_or_else(|| {
                                JsError::from(
                                    JsNativeError::typ()
                                        .with_message("serve() 'handler' must be a function"),
                                )
                            })?;
                        let handler = JsFunction::from_object(handler_obj.clone()).ok_or_else(|| {
                            JsError::from(
                                JsNativeError::typ()
                                    .with_message("serve() 'handler' must be a function"),
                            )
                        })?;

                        serve_impl(port, handler, ctx)
                    })
                    .into_js_function_copied(ctx),
                    "serve",
                    1,
                    ctx,
                );
                m.set_export(&js_string!("serve"), serve_fn.clone())?;

                // ── default — 整个 http 对象 ────────────────────────────────
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
