pub mod client;
pub mod common;
pub mod incoming;
pub mod outgoing;
pub mod server;

use boa_engine::module::SyntheticModuleInitializer;
use boa_engine::object::builtins::JsArray;
use boa_engine::{js_string, Context, JsObject, JsString, JsValue, Module};

pub fn create_node_http_module(context: &mut Context) -> Result<Module, String> {
    let export_names: &[JsString] = &[
        js_string!("createServer"),
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
                        let _ = status_codes.set(
                            JsString::from(code.to_string()),
                            js_string!(text),
                            false,
                            ctx,
                        );
                    }

                    // createServer(requestListener)
                    let create_server_fn = common::build_fn(
                        common::make_native(
                            |_: &JsValue, args: &[JsValue], ctx: &mut Context| -> Result<JsValue, _> {
                                let listener = args.first().cloned().unwrap_or(JsValue::undefined());
                                server::create_server(listener, ctx)
                            },
                        ),
                        "createServer",
                        1,
                        ctx,
                    );

                    // request(options, callback)
                    let http_request = common::build_fn(
                        common::make_native(
                            |_: &JsValue, args: &[JsValue], ctx: &mut Context| -> Result<JsValue, _> {
                                let callback = args.get(1).cloned().unwrap_or(JsValue::undefined());

                                let creq = JsObject::with_object_proto(ctx.intrinsics());
                                let _ = creq.set(js_string!("__method"), js_string!("GET"), false, ctx);
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
                                let _ = creq.set(js_string!("__ended"), JsValue::from(false), false, ctx);

                                if let Some(opts) = args.first() {
                                    if let Some(s) = opts.as_string() {
                                        let _ = creq.set(js_string!("__url"), js_string!(s.to_std_string_escaped()), false, ctx);
                                    } else if let Some(obj) = opts.as_object() {
                                        if let Ok(method) = obj.get(js_string!("method"), ctx) {
                                            if !method.is_undefined() {
                                                let _ = creq.set(js_string!("__method"), method, false, ctx);
                                            }
                                        }
                                        if let Ok(hostname) = obj.get(js_string!("hostname"), ctx) {
                                            let port_val = obj.get(js_string!("port"), ctx).ok()
                                                .and_then(|v| v.as_number()).unwrap_or(80.0) as u16;
                                            let path_val = obj.get(js_string!("path"), ctx).ok()
                                                .and_then(|v| v.to_string(ctx).ok())
                                                .map(|s| s.to_std_string_escaped())
                                                .unwrap_or_else(|| "/".to_string());
                                            if let Some(h) = hostname.as_string() {
                                                let url = format!("http://{}:{}{}", h.to_std_string_escaped(), port_val, path_val);
                                                let _ = creq.set(js_string!("__url"), js_string!(url), false, ctx);
                                            }
                                        }
                                        if let Ok(hdr_obj) = obj.get(js_string!("headers"), ctx) {
                                            if let Some(_h) = hdr_obj.as_object() {
                                                let _ = creq.set(js_string!("__headers"), hdr_obj, false, ctx);
                                            }
                                        }
                                    }
                                }

                                let _ = creq.set(js_string!("write"), client::create_request_write(ctx), false, ctx);
                                let _ = creq.set(js_string!("end"), client::create_request_end(ctx), false, ctx);

                                Ok(JsValue::from(creq))
                            },
                        ),
                        "request",
                        2,
                        ctx,
                    );

                    // get(url, callback)
                    let http_get = common::build_fn(
                        common::make_native(
                            |_: &JsValue, args: &[JsValue], ctx: &mut Context| -> Result<JsValue, _> {
                                let opts = args.first().cloned().unwrap_or(JsValue::undefined());
                                let callback_val = args.get(1).cloned().unwrap_or(JsValue::undefined());

                                let url = if let Some(s) = opts.as_string() {
                                    s.to_std_string_escaped()
                                } else if let Some(obj) = opts.as_object() {
                                    let hostname = obj.get(js_string!("hostname"), ctx).ok()
                                        .and_then(|v| v.to_string(ctx).ok())
                                        .map(|s| s.to_std_string_escaped())
                                        .unwrap_or_else(|| "localhost".to_string());
                                    let port = obj.get(js_string!("port"), ctx).ok()
                                        .and_then(|v| v.as_number()).unwrap_or(80.0) as u16;
                                    let path = obj.get(js_string!("path"), ctx).ok()
                                        .and_then(|v| v.to_string(ctx).ok())
                                        .map(|s| s.to_std_string_escaped())
                                        .unwrap_or_else(|| "/".to_string());
                                    let proto = if port == 443 { "https" } else { "http" };
                                    format!("{proto}://{hostname}:{port}{path}")
                                } else {
                                    return Err(boa_engine::JsNativeError::typ()
                                        .with_message("http.get: invalid URL").into());
                                };

                                let client = reqwest::blocking::Client::builder()
                                    .danger_accept_invalid_certs(true).build()
                                    .map_err(|e| boa_engine::JsNativeError::typ()
                                        .with_message(format!("http.get: {e}")))?;

                                match client.get(&url).send() {
                                    Ok(resp) => {
                                        let res = common::create_response_from_reqwest(resp, ctx)?;
                                        if let Some(cb) = callback_val.as_object().filter(|o| o.is_callable()) {
                                            let _ = cb.call(&JsValue::undefined(), &[JsValue::from(res.clone())], ctx);
                                        }
                                        if let Ok(body_val) = res.get(js_string!("__body"), ctx) {
                                            if let Some(arr_obj) = body_val.as_object() {
                                                if let Ok(arr) = JsArray::from_object(arr_obj.clone()) {
                                                    for i in 0..arr.length(ctx).unwrap_or(0) {
                                                        if let Ok(item) = arr.get(i, ctx) {
                                                            common::emit(&res, "data", &[item], ctx);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        common::emit(&res, "end", &[], ctx);
                                        Ok(JsValue::from(res))
                                    }
                                    Err(e) => Err(boa_engine::JsNativeError::typ()
                                        .with_message(format!("http.get failed: {e}")).into()),
                                }
                            },
                        ),
                        "get",
                        2,
                        ctx,
                    );

                    let _ = m.set_export(&js_string!("createServer"), create_server_fn.clone());
                    let _ = m.set_export(&js_string!("request"), http_request.clone());
                    let _ = m.set_export(&js_string!("get"), http_get.clone());
                    let _ = m.set_export(&js_string!("STATUS_CODES"), JsValue::from(status_codes.clone()));

                    let default_obj = JsObject::with_object_proto(ctx.intrinsics());
                    let _ = default_obj.set(js_string!("createServer"), create_server_fn, false, ctx);
                    let _ = default_obj.set(js_string!("request"), http_request, false, ctx);
                    let _ = default_obj.set(js_string!("get"), http_get, false, ctx);
                    let _ = default_obj.set(js_string!("STATUS_CODES"), JsValue::from(status_codes), false, ctx);
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
