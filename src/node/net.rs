use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread;

use boa_engine::module::SyntheticModuleInitializer;
use boa_engine::object::FunctionObjectBuilder;
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

fn is_ip(input: &str) -> u8 {
    if is_ipv4(input) {
        4
    } else if is_ipv6(input) {
        6
    } else {
        0
    }
}

fn is_ipv4(input: &str) -> bool {
    if input.is_empty() || input.starts_with('.') || input.ends_with('.') {
        return false;
    }
    let parts: Vec<&str> = input.split('.').collect();
    if parts.len() != 4 {
        return false;
    }
    for part in &parts {
        if part.is_empty() || part.len() > 3 {
            return false;
        }
        if part.len() > 1 && part.starts_with('0') {
            return false; // no leading zeros
        }
        match part.parse::<u16>() {
            Ok(n) if n <= 255 => {}
            _ => return false,
        }
    }
    true
}

fn is_ipv6(input: &str) -> bool {
    if input.is_empty() {
        return false;
    }
    if input == "::" {
        return true;
    }
    let has_double_colon = input.contains("::");
    let parts: Vec<&str> = input.split(':').collect();
    if parts.len() > 8 || parts.len() < 2 {
        return false;
    }
    if has_double_colon {
        if input.starts_with("::") && input.len() > 2 && !input.as_bytes()[2].is_ascii_hexdigit() {
            return false;
        }
    }
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            if !has_double_colon {
                return false;
            }
            continue;
        }
        if part.len() > 4 {
            return false;
        }
        if !part.chars().all(|c| c.is_ascii_hexdigit()) {
            return false;
        }
    }
    true
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

fn create_socket_object(ctx: &mut Context) -> JsObject {
    let sock = JsObject::with_object_proto(ctx.intrinsics());
    let _ = sock.set(
        js_string!("_events"),
        JsValue::from(JsObject::with_object_proto(ctx.intrinsics())),
        false,
        ctx,
    );
    let _ = sock.set(js_string!("__connected"), JsValue::from(false), false, ctx);
    let _ = sock.set(js_string!("__destroyed"), JsValue::from(false), false, ctx);

    let sock_on = build_fn(
        make_native(
            move |this: &JsValue, args: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
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
    let _ = sock.set(js_string!("on"), sock_on.clone(), false, ctx);
    let _ = sock.set(js_string!("addListener"), sock_on.clone(), false, ctx);
    let _ = sock.set(js_string!("once"), sock_on, false, ctx);

    // connect(port, host, cb)
    let sock_connect = build_fn(
        make_native(
            move |this: &JsValue, args: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
                let inst = get_obj(this)?;
                let port = args.first().and_then(|v| v.as_number()).unwrap_or(0.0) as u16;
                let host = args.get(1)
                    .and_then(|v| v.to_string(ctx).ok())
                    .map(|s| s.to_std_string_escaped())
                    .unwrap_or_else(|| "127.0.0.1".to_string());
                match std::net::TcpStream::connect((host.as_str(), port)) {
                    Ok(stream) => {
                        let _ = inst.set(js_string!("__stream"), JsValue::from(port as f64), false, ctx);
                        let _ = inst.set(js_string!("__connected"), JsValue::from(true), false, ctx);
                        emit(&inst, "connect", &[], ctx);
                        if let Some(cb) = args.get(2).or_else(|| args.get(1))
                            .and_then(|v| v.as_object())
                            .filter(|o| o.is_callable())
                        {
                            let _ = cb.call(&JsValue::undefined(), &[], ctx);
                        }
                    }
                    Err(e) => {
                        let _ = inst.set(js_string!("__connecting"), JsValue::from(false), false, ctx);
                        emit(&inst, "error", &[JsValue::from(js_string!(e.to_string()))], ctx);
                    }
                }
                Ok(this.clone())
            },
        ),
        "connect",
        2,
        ctx,
    );
    let _ = sock.set(js_string!("connect"), sock_connect, false, ctx);

    // write(data, cb)
    let sock_write = build_fn(
        make_native(
            move |this: &JsValue, args: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
                if let Some(data) = args.first() {
                    if let Some(s) = data.as_string() {
                        // Stub: data is written to nowhere since TcpStream isn't stored
                        // Will be implemented properly in the Socket-as-Duplex-Stream refactor
                    }
                }
                if let Some(cb_fn) = args.get(1)
                    .and_then(|v| v.as_object())
                    .filter(|o| o.is_callable())
                {
                    let _ = cb_fn.call(&JsValue::undefined(), &[], ctx);
                }
                Ok(JsValue::from(true))
            },
        ),
        "write",
        1,
        ctx,
    );
    let _ = sock.set(js_string!("write"), sock_write, false, ctx);

    // end([data], cb)
    let sock_end = build_fn(
        make_native(
            move |this: &JsValue, args: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
                let inst = get_obj(this)?;
                let _ = inst.set(js_string!("__destroyed"), JsValue::from(true), false, ctx);
                emit(&inst, "end", &[], ctx);
                emit(&inst, "close", &[], ctx);
                if let Some(cb) = args.first()
                    .and_then(|v| v.as_object())
                    .filter(|o| o.is_callable())
                {
                    let _ = cb.call(&JsValue::undefined(), &[], ctx);
                } else if let Some(cb) = args.get(1)
                    .and_then(|v| v.as_object())
                    .filter(|o| o.is_callable())
                {
                    let _ = cb.call(&JsValue::undefined(), &[], ctx);
                }
                Ok(this.clone())
            },
        ),
        "end",
        1,
        ctx,
    );
    let _ = sock.set(js_string!("end"), sock_end, false, ctx);

    // destroy()
    let sock_destroy = build_fn(
        make_native(
            move |this: &JsValue, _args: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
                let inst = get_obj(this)?;
                let _ = inst.set(js_string!("__destroyed"), JsValue::from(true), false, ctx);
                emit(&inst, "close", &[], ctx);
                Ok(JsValue::undefined())
            },
        ),
        "destroy",
        0,
        ctx,
    );
    let _ = sock.set(js_string!("destroy"), sock_destroy, false, ctx);

    // setTimeout(ms, cb)
    let sock_set_timeout = build_fn(
        make_native(
            move |this: &JsValue, _args: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
                Ok(this.clone())
            },
        ),
        "setTimeout",
        1,
        ctx,
    );
    let _ = sock.set(js_string!("setTimeout"), sock_set_timeout, false, ctx);

    sock
}

pub fn create_node_net_module(context: &mut Context) -> Result<Module, String> {
    let export_names: &[JsString] = &[
        js_string!("createServer"),
        js_string!("Server"),
        js_string!("Socket"),
        js_string!("isIP"),
        js_string!("isIPv4"),
        js_string!("isIPv6"),
        js_string!("default"),
    ];

    let module = Module::synthetic(
        export_names,
        unsafe {
            SyntheticModuleInitializer::from_closure(
                move |m: &boa_engine::module::SyntheticModule, ctx: &mut Context| {
                    let create_server = build_fn(
                        make_native(
                            |_: &JsValue,
                             args: &[JsValue],
                             ctx: &mut Context|
                             -> JsResult<JsValue> {
                                let conn_listener =
                                    args.first().cloned().unwrap_or(JsValue::undefined());
                                let server = JsObject::with_object_proto(ctx.intrinsics());
                                let _ = server.set(
                                    js_string!("__listening"),
                                    JsValue::from(false),
                                    false,
                                    ctx,
                                );
                                let _ = server.set(
                                    js_string!("__connections"),
                                    JsValue::from(0.0),
                                    false,
                                    ctx,
                                );

                                if !conn_listener.is_undefined() {
                                    let _ =
                                        add_listener(&server, "connection", &conn_listener, ctx);
                                }

                                // close(cb)
                                let close_fn = build_fn(make_native(|this: &JsValue, args: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
                            let inst = get_obj(this)?;
                            let _ = inst.set(js_string!("__listening"), JsValue::from(false), false, ctx);
                            if let Some(cb) = args.first().and_then(|v| v.as_object()).filter(|o| o.is_callable()) {
                                let _ = cb.call(&JsValue::undefined(), &[], ctx);
                            }
                            Ok(JsValue::undefined())
                        }), "close", 1, ctx);
                                let _ = server.set(js_string!("close"), close_fn, false, ctx);

                                // listen(port, host, cb)
                                let listen_fn = build_fn(make_native(|this: &JsValue, args: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
                            let port = args.first().and_then(|v| v.as_number()).unwrap_or(0.0) as u16;
                            let host_arg = args.get(1).and_then(|v| v.to_string(ctx).ok()).map(|s| s.to_std_string_escaped());
                            let cb_arg = if host_arg.is_some() { args.get(2) } else { args.get(1) };
                            let host = host_arg.unwrap_or_else(|| "127.0.0.1".to_string());

                            let listener = TcpListener::bind(format!("{host}:{port}")).map_err(|e| -> JsError {
                                JsNativeError::typ().with_message(format!("listen EADDRINUSE: port={port} {e}")).into()
                            })?;
                            let addr = listener.local_addr().map_err(|e| -> JsError {
                                JsNativeError::typ().with_message(format!("address error: {e}")).into()
                            })?;

                            let inst = get_obj(this)?;
                            let _ = inst.set(js_string!("__listening"), JsValue::from(true), false, ctx);

                            // 回调 listen callback
                            if let Some(cb_val) = cb_arg {
                                if let Some(cb_fn) = cb_val.as_object().filter(|o| o.is_callable()) {
                                    let _ = cb_fn.call(&JsValue::undefined(), &[], ctx);
                                }
                            }

                            // 单线程同步接受连接
                            let (tx, rx) = mpsc::channel::<TcpStream>();
                            let _t = thread::spawn(move || {
                                for stream in listener.incoming() {
                                    match stream {
                                        Ok(s) => { let _ = tx.send(s); }
                                        Err(_) => break,
                                    }
                                }
                            });

                            // 主线程处理连接
                            loop {
                                let _listening = inst.get(js_string!("__listening"), ctx)
                                    .ok().and_then(|v| v.as_boolean()).unwrap_or(false);
                                if !_listening { break; }

                                match rx.try_recv() {
                                    Ok(mut stream) => {
                                        // 为每个连接创建 Socket
                                        let socket = JsObject::with_object_proto(ctx.intrinsics());
                                        let _ = socket.set(js_string!("remoteAddress"), JsString::from(addr.ip().to_string()), false, ctx);
                                        let _ = socket.set(js_string!("remotePort"), JsValue::from(addr.port() as f64), false, ctx);
                                        let _ = socket.set(js_string!("__closed"), JsValue::from(false), false, ctx);

                                        // read(fn) - 注册读回调
                                        // 读取 HTTP 请求并交给上层处理
                                        let mut reader = BufReader::new(&mut stream);
                                        let mut request_line = String::new();
                                        if reader.read_line(&mut request_line).is_ok() && !request_line.is_empty() {
                                            emit(&socket, "data", &[JsValue::from(JsString::from(request_line.clone()))], ctx);
                                        }
                                        emit(&socket, "end", &[], ctx);

                                        // 通知 server 有 connection
                                        emit(&inst, "connection", &[JsValue::from(socket)], ctx);
                                    }
                                    Err(mpsc::TryRecvError::Empty) => {
                                        // 无新连接，短暂休眠
                                        thread::sleep(std::time::Duration::from_millis(10));
                                    }
                                    Err(mpsc::TryRecvError::Disconnected) => break,
                                }
                            }

                            Ok(JsValue::undefined())
                        }), "listen", 3, ctx);
                                let _ = server.set(js_string!("listen"), listen_fn, false, ctx);

                                Ok(JsValue::from(server))
                            },
                        ),
                        "createServer",
                        1,
                        ctx,
                    );

                    let default_obj = JsObject::with_object_proto(ctx.intrinsics());
                    let _ = default_obj.set(
                        js_string!("createServer"),
                        create_server.clone(),
                        false,
                        ctx,
                    );
                    let _ = m.set_export(&js_string!("createServer"), create_server.clone());

                    // Socket factory
                    let socket_fn = build_fn(
                        make_native(
                            |_: &JsValue, _args: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
                                Ok(JsValue::from(create_socket_object(ctx)))
                            },
                        ),
                        "Socket",
                        0,
                        ctx,
                    );
                    let _ = m.set_export(&js_string!("Socket"), socket_fn.clone());
                    let _ = default_obj.set(js_string!("Socket"), socket_fn, false, ctx);

                    // isIP(input)
                    let is_ip_fn = build_fn(
                        make_native(
                            |_: &JsValue, args: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
                                let input = args.first()
                                    .and_then(|v| v.to_string(ctx).ok())
                                    .map(|s| s.to_std_string_escaped())
                                    .unwrap_or_default();
                                Ok(JsValue::from(is_ip(&input) as f64))
                            },
                        ),
                        "isIP",
                        1,
                        ctx,
                    );
                    let _ = m.set_export(&js_string!("isIP"), is_ip_fn.clone());
                    let _ = default_obj.set(js_string!("isIP"), is_ip_fn, false, ctx);

                    // isIPv4(input)
                    let is_ipv4_fn = build_fn(
                        make_native(
                            |_: &JsValue, args: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
                                let input = args.first()
                                    .and_then(|v| v.to_string(ctx).ok())
                                    .map(|s| s.to_std_string_escaped())
                                    .unwrap_or_default();
                                Ok(JsValue::from(is_ipv4(&input)))
                            },
                        ),
                        "isIPv4",
                        1,
                        ctx,
                    );
                    let _ = m.set_export(&js_string!("isIPv4"), is_ipv4_fn.clone());
                    let _ = default_obj.set(js_string!("isIPv4"), is_ipv4_fn, false, ctx);

                    // isIPv6(input)
                    let is_ipv6_fn = build_fn(
                        make_native(
                            |_: &JsValue, args: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
                                let input = args.first()
                                    .and_then(|v| v.to_string(ctx).ok())
                                    .map(|s| s.to_std_string_escaped())
                                    .unwrap_or_default();
                                Ok(JsValue::from(is_ipv6(&input)))
                            },
                        ),
                        "isIPv6",
                        1,
                        ctx,
                    );
                    let _ = m.set_export(&js_string!("isIPv6"), is_ipv6_fn.clone());
                    let _ = default_obj.set(js_string!("isIPv6"), is_ipv6_fn, false, ctx);

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
