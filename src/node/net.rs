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
