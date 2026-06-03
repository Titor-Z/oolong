use std::io::Write;
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

use boa_engine::object::FunctionObjectBuilder;
use boa_engine::{js_string, Context, JsError, JsNativeError, JsObject, JsResult, JsValue};

use super::common::{
    add_listener, build_fn, build_response_string, emit, flush_response, get_obj, make_native,
    parse_http_request,
};
use super::incoming::create_incoming_message;
use super::outgoing::create_outgoing_message;

pub fn create_server(request_listener: JsValue, ctx: &mut Context) -> JsResult<JsValue> {
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
                    let _ = cb.call(&JsValue::undefined(), &[], ctx);
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
                let host = inst
                    .get(js_string!("__host"), ctx)
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

    // listen(port, host, cb) — non-blocking via setInterval polling
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

            listener
                .set_nonblocking(true)
                .map_err(|e| -> JsError {
                    JsNativeError::typ()
                        .with_message(format!(
                            "set_nonblocking error: {e}"
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

            let listener_arc = Arc::new(Mutex::new(listener));
            let server_obj = inst.clone();

            // Poll function: called every ~10ms by setInterval on the JS main thread
            let poll_fn = make_native(
                move |_: &JsValue,
                      _: &[JsValue],
                      ctx: &mut Context|
                      -> JsResult<JsValue> {
                    let listening = server_obj
                        .get(js_string!("__listening"), ctx)
                        .ok()
                        .and_then(|v| v.as_boolean())
                        .unwrap_or(false);
                    if !listening {
                        return Ok(JsValue::undefined());
                    }

                    loop {
                        // Do NOT hold the lock while doing JS work
                        let accept_result = {
                            let guard =
                                listener_arc.lock().unwrap();
                            guard.accept()
                        };

                        match accept_result {
                            Ok((mut tcp_stream, _)) => {
                                let peer =
                                    tcp_stream.peer_addr().ok();
                                let peer_ip = peer
                                    .as_ref()
                                    .map(|p| p.ip().to_string());
                                let peer_port =
                                    peer.map(|p| p.port()).unwrap_or(0);

                                match parse_http_request(
                                    &mut tcp_stream,
                                ) {
                                    Ok((
                                        method,
                                        path,
                                        headers,
                                        body,
                                    )) => {
                                        let stream_arc = Arc::new(
                                            Mutex::new(tcp_stream),
                                        );

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
                                        let res =
                                            create_outgoing_message(
                                                stream_arc.clone(),
                                                ctx,
                                            );

                                        emit(
                                            &server_obj,
                                            "request",
                                            &[
                                                JsValue::from(
                                                    req.clone(),
                                                ),
                                                JsValue::from(
                                                    res.clone(),
                                                ),
                                            ],
                                            ctx,
                                        );

                                        let handler = server_obj
                                            .get(
                                                js_string!(
                                                    "__handler"
                                                ),
                                                ctx,
                                            )
                                            .ok()
                                            .unwrap_or(
                                                JsValue::undefined(),
                                            );
                                        if let Some(handler_fn) =
                                            handler
                                                .as_object()
                                                .filter(|o| {
                                                    o.is_callable()
                                                })
                                        {
                                            let _ = handler_fn.call(
                                                &JsValue::undefined(
                                                ),
                                                &[
                                                    JsValue::from(
                                                        req.clone(),
                                                    ),
                                                    JsValue::from(
                                                        res.clone(),
                                                    ),
                                                ],
                                                ctx,
                                            );
                                        }

                                        let ended = res
                                            .get(
                                                js_string!("__ended"),
                                                ctx,
                                            )
                                            .ok()
                                            .and_then(|v| {
                                                v.as_boolean()
                                            })
                                            .unwrap_or(false);

                                        if !ended {
                                            let body_str =
                                                String::from_utf8_lossy(
                                                    &body,
                                                );
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

                                        let ended2 = res
                                            .get(
                                                js_string!("__ended"),
                                                ctx,
                                            )
                                            .ok()
                                            .and_then(|v| {
                                                v.as_boolean()
                                            })
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
                                            emit(
                                                &res,
                                                "close",
                                                &[],
                                                ctx,
                                            );
                                        }

                                        if let Ok(s) =
                                            stream_arc.lock()
                                        {
                                            let _ = s.shutdown(
                                                std::net::Shutdown::Write,
                                            );
                                        }
                                    }
                                    Err(e) => {
                                        let err_resp =
                                            build_response_string(
                                                400,
                                                "Bad Request",
                                                &[],
                                                format!(
                                                    "Bad Request: {e}"
                                                )
                                                .as_bytes(),
                                            );
                                        let _ = tcp_stream
                                            .write_all(
                                                err_resp.as_bytes(),
                                            );
                                        let _ = tcp_stream
                                            .shutdown(
                                                std::net::Shutdown::Write,
                                            );
                                    }
                                }
                            }
                            Err(ref e)
                                if e.kind()
                                    == std::io::ErrorKind::WouldBlock =>
                            {
                                break;
                            }
                            Err(_) => break,
                        }
                    }

                    // Re-schedule poll via setImmediate if still listening
                    let still_listening = server_obj
                        .get(js_string!("__listening"), ctx)
                        .ok()
                        .and_then(|v| v.as_boolean())
                        .unwrap_or(false);
                    if still_listening {
                        if let Ok(poll_val) =
                            server_obj.get(js_string!("__poll"), ctx)
                        {
                            if let Some(poll_fn_obj) = poll_val
                                .as_object()
                                .filter(|o| o.is_callable())
                            {
                                let global = ctx.global_object();
                                if let Ok(si_val) = global.get(
                                    js_string!("setImmediate"),
                                    ctx,
                                ) {
                                    if let Some(si_fn) = si_val
                                        .as_object()
                                        .filter(|o| o.is_callable())
                                    {
                                        let _ = si_fn.call(
                                            &JsValue::undefined(),
                                            &[JsValue::from(
                                                poll_fn_obj.clone(),
                                            )],
                                            ctx,
                                        );
                                    }
                                }
                            }
                        }
                    }

                    Ok(JsValue::undefined())
                },
            );

            let poll_js = build_fn(poll_fn, "__poll", 0, ctx);
            let _ = inst.set(js_string!("__poll"), poll_js.clone(), false, ctx);

            // Schedule first poll via setImmediate, then it chains itself
            let global = ctx.global_object();
            let set_immediate_val =
                global.get(js_string!("setImmediate"), ctx)?;
            let set_immediate_fn = set_immediate_val.as_object().ok_or_else(
                || {
                    JsNativeError::typ()
                        .with_message("setImmediate not found")
                },
            )?;
            let _ = set_immediate_fn.call(
                &JsValue::undefined(),
                &[poll_js],
                ctx,
            )?;

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
}
