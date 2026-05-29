use std::time::UNIX_EPOCH;

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

use boa_engine::module::SyntheticModuleInitializer;
use boa_engine::object::builtins::{JsArray, JsArrayBuffer, JsFunction, JsPromise};
use boa_engine::{
    Context, JsError, JsNativeError, JsObject, JsResult, JsString, JsValue, Module, NativeFunction,
    js_string, object::FunctionObjectBuilder,
};

fn to_js_fn(f: NativeFunction, name: &str, len: usize, ctx: &mut Context) -> JsValue {
    FunctionObjectBuilder::new(ctx.realm(), f)
        .name(JsString::from(name))
        .length(len)
        .build()
        .into()
}

fn resolved(val: JsValue, ctx: &mut Context) -> JsValue {
    JsPromise::resolve(val, ctx).into()
}

fn rejected(msg: &str, ctx: &mut Context) -> JsValue {
    let err = JsNativeError::typ().with_message(msg.to_string());
    let (promise, resolvers) = JsPromise::new_pending(ctx);
    let _ = resolvers
        .reject
        .call(&JsValue::undefined(), &[err.to_opaque(ctx).into()], ctx);
    promise.into()
}

fn js_err(msg: &str) -> JsError {
    JsError::from(JsNativeError::typ().with_message(msg.to_string()))
}

fn call_cb_ok(cb: &JsValue, result: JsValue, ctx: &mut Context) {
    if let Some(obj) = cb.as_object()
        && let Some(func) = JsFunction::from_object(obj.clone()) {
            let _ = func.call(&JsValue::undefined(), &[JsValue::null(), result], ctx);
        }
}

fn call_cb_err(cb: &JsValue, msg: &str, ctx: &mut Context) {
    if let Some(obj) = cb.as_object()
        && let Some(func) = JsFunction::from_object(obj.clone()) {
            let err = JsNativeError::typ().with_message(msg.to_string());
            let _ = func.call(
                &JsValue::undefined(),
                &[err.to_opaque(ctx).into(), JsValue::undefined()],
                ctx,
            );
        }
}

fn extract_path(args: &[JsValue]) -> Result<String, JsError> {
    args.first()
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .ok_or_else(|| js_err("path must be a string"))
}

fn extract_two_paths(args: &[JsValue]) -> Result<(String, String), JsError> {
    let a = args
        .first()
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .ok_or_else(|| js_err("first argument must be a string"))?;
    let b = args
        .get(1)
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .ok_or_else(|| js_err("second argument must be a string"))?;
    Ok((a, b))
}

fn extract_cb(args: &[JsValue]) -> Option<JsValue> {
    args.iter().find(|v| v.is_callable()).cloned()
}

fn extract_recursive(args: &[JsValue], ctx: &mut Context) -> bool {
    args.iter().any(|v| {
        v.as_object()
            .and_then(|o| o.get(js_string!("recursive"), ctx).ok())
            .map(|r| r.to_boolean())
            .unwrap_or(false)
    })
}

fn extract_bytes(value: &JsValue, ctx: &mut Context) -> Option<Vec<u8>> {
    if let Some(s) = value.as_string() {
        return Some(s.to_std_string_escaped().into_bytes());
    }
    if let Some(obj) = value.as_object() {
        if let Ok(buf) = JsArrayBuffer::from_object(obj.clone())
            && let Some(data) = buf.data()
        {
            return Some(data.to_vec());
        }
        if let Ok(v) = obj.get(js_string!("buffer"), ctx)
            && let Some(buf_obj) = v.as_object()
            && let Ok(buf) = JsArrayBuffer::from_object(buf_obj.clone())
            && let Some(data) = buf.data()
        {
            return Some(data.to_vec());
        }
    }
    None
}

fn encode_data(data: &[u8], encoding: Option<&str>, ctx: &mut Context) -> JsValue {
    match encoding {
        Some("utf8") | Some("utf-8") | None => {
            JsValue::from(JsString::from(String::from_utf8_lossy(data)))
        }
        Some("hex") => {
            let hex: String = data.iter().map(|b| format!("{:02x}", b)).collect();
            JsValue::from(JsString::from(hex))
        }
        Some("base64") => {
            const CHARS: &[u8] =
                b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
            let mut out = Vec::with_capacity(data.len().div_ceil(3) * 4);
            for chunk in data.chunks(3) {
                let b0 = chunk[0] as u32;
                let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
                let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
                let triple = (b0 << 16) | (b1 << 8) | b2;
                out.push(CHARS[((triple >> 18) & 0x3F) as usize]);
                out.push(CHARS[((triple >> 12) & 0x3F) as usize]);
                if chunk.len() > 1 {
                    out.push(CHARS[((triple >> 6) & 0x3F) as usize]);
                } else {
                    out.push(b'=');
                }
                if chunk.len() > 2 {
                    out.push(CHARS[(triple & 0x3F) as usize]);
                } else {
                    out.push(b'=');
                }
            }
            JsValue::from(JsString::from(String::from_utf8(out).unwrap_or_default()))
        }
        Some("buffer") | Some("latin1") | Some("ascii") => {
            match JsArrayBuffer::new(data.len(), ctx) {
                Ok(buf) => {
                    if let Some(mut d) = buf.data_mut() {
                        d.copy_from_slice(data);
                    }
                    buf.into()
                }
                Err(_) => JsValue::undefined(),
            }
        }
        Some(_) => JsValue::from(JsString::from(String::from_utf8_lossy(data))),
    }
}

fn metadata_to_stats(md: &std::fs::Metadata, ctx: &mut Context) -> JsResult<JsValue> {
    let obj = JsObject::with_object_proto(ctx.intrinsics());

    #[cfg(unix)]
    {
        let _ = obj.set(
            js_string!("dev"),
            JsValue::from(md.dev() as f64),
            false,
            ctx,
        );
        let _ = obj.set(
            js_string!("ino"),
            JsValue::from(md.ino() as f64),
            false,
            ctx,
        );
        let _ = obj.set(
            js_string!("mode"),
            JsValue::from(md.mode() as f64),
            false,
            ctx,
        );
        let _ = obj.set(
            js_string!("nlink"),
            JsValue::from(md.nlink() as f64),
            false,
            ctx,
        );
        let _ = obj.set(
            js_string!("uid"),
            JsValue::from(md.uid() as f64),
            false,
            ctx,
        );
        let _ = obj.set(
            js_string!("gid"),
            JsValue::from(md.gid() as f64),
            false,
            ctx,
        );
        let _ = obj.set(
            js_string!("rdev"),
            JsValue::from(md.rdev() as f64),
            false,
            ctx,
        );
        let _ = obj.set(
            js_string!("blksize"),
            JsValue::from(md.blksize() as f64),
            false,
            ctx,
        );
        let _ = obj.set(
            js_string!("blocks"),
            JsValue::from(md.blocks() as f64),
            false,
            ctx,
        );
    }
    #[cfg(not(unix))]
    {
        let _ = obj.set(js_string!("dev"), JsValue::from(0), false, ctx);
        let _ = obj.set(js_string!("ino"), JsValue::from(0), false, ctx);
        let _ = obj.set(js_string!("mode"), JsValue::from(0), false, ctx);
        let _ = obj.set(js_string!("nlink"), JsValue::from(0), false, ctx);
        let _ = obj.set(js_string!("uid"), JsValue::from(0), false, ctx);
        let _ = obj.set(js_string!("gid"), JsValue::from(0), false, ctx);
        let _ = obj.set(js_string!("rdev"), JsValue::from(0), false, ctx);
        let _ = obj.set(js_string!("blksize"), JsValue::from(0), false, ctx);
        let _ = obj.set(js_string!("blocks"), JsValue::from(0), false, ctx);
    }

    let size = md.len() as f64;
    let atime_ms = md
        .accessed()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0);
    let mtime_ms = md
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0);
    let ctime_ms = md
        .created()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as f64)
        .unwrap_or(mtime_ms);
    let birthtime_ms = md
        .created()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0);

    let _ = obj.set(js_string!("size"), JsValue::from(size), false, ctx);
    let _ = obj.set(js_string!("atimeMs"), JsValue::from(atime_ms), false, ctx);
    let _ = obj.set(js_string!("mtimeMs"), JsValue::from(mtime_ms), false, ctx);
    let _ = obj.set(js_string!("ctimeMs"), JsValue::from(ctime_ms), false, ctx);
    let _ = obj.set(
        js_string!("birthtimeMs"),
        JsValue::from(birthtime_ms),
        false,
        ctx,
    );
    let _ = obj.set(
        js_string!("isFile"),
        JsValue::from(md.is_file()),
        false,
        ctx,
    );
    let _ = obj.set(
        js_string!("isDirectory"),
        JsValue::from(md.is_dir()),
        false,
        ctx,
    );
    let _ = obj.set(
        js_string!("isSymbolicLink"),
        JsValue::from(md.is_symlink()),
        false,
        ctx,
    );
    let _ = obj.set(
        js_string!("isBlockDevice"),
        JsValue::from(false),
        false,
        ctx,
    );
    let _ = obj.set(
        js_string!("isCharacterDevice"),
        JsValue::from(false),
        false,
        ctx,
    );
    let _ = obj.set(js_string!("isFIFO"), JsValue::from(false), false, ctx);
    let _ = obj.set(js_string!("isSocket"), JsValue::from(false), false, ctx);

    Ok(obj.into())
}

pub fn create_node_fs_module(context: &mut Context) -> Result<Module, String> {
    let export_names: &[JsString] = &[
        js_string!("readFile"),
        js_string!("readFileSync"),
        js_string!("writeFile"),
        js_string!("writeFileSync"),
        js_string!("appendFile"),
        js_string!("appendFileSync"),
        js_string!("mkdir"),
        js_string!("mkdirSync"),
        js_string!("readdir"),
        js_string!("readdirSync"),
        js_string!("rmdir"),
        js_string!("rmdirSync"),
        js_string!("rm"),
        js_string!("rmSync"),
        js_string!("unlink"),
        js_string!("unlinkSync"),
        js_string!("stat"),
        js_string!("statSync"),
        js_string!("lstat"),
        js_string!("lstatSync"),
        js_string!("access"),
        js_string!("accessSync"),
        js_string!("chmod"),
        js_string!("chmodSync"),
        js_string!("rename"),
        js_string!("renameSync"),
        js_string!("copyFile"),
        js_string!("copyFileSync"),
        js_string!("existsSync"),
        js_string!("realpath"),
        js_string!("realpathSync"),
        js_string!("symlink"),
        js_string!("symlinkSync"),
        js_string!("link"),
        js_string!("linkSync"),
        js_string!("truncate"),
        js_string!("truncateSync"),
        js_string!("watch"),
        js_string!("constants"),
        js_string!("promises"),
        js_string!("default"),
    ];

    let module = Module::synthetic(
        export_names,
        // SAFETY: The closure captures JsValue references, which are Trace.
        unsafe {
            SyntheticModuleInitializer::from_closure(
                move |m: &boa_engine::module::SyntheticModule, ctx: &mut Context| {
                    let read_file_val = to_js_fn(
                        NativeFunction::from_closure(
                            |_this: &JsValue,
                             args: &[JsValue],
                             ctx: &mut Context|
                             -> JsResult<JsValue> {
                                let path = extract_path(args)?;
                                let cb = extract_cb(args);
                                match std::fs::read(&path) {
                                    Ok(data) => {
                                        let result = encode_data(&data, None, ctx);
                                        if let Some(ref c) = cb {
                                            call_cb_ok(c, result, ctx);
                                            Ok(JsValue::undefined())
                                        } else {
                                            Ok(resolved(result, ctx))
                                        }
                                    }
                                    Err(_e) => {
                                        let msg = format!(
                                            "ENOENT: no such file or directory, open '{path}'"
                                        );
                                        if let Some(ref c) = cb {
                                            call_cb_err(c, &msg, ctx);
                                            Ok(JsValue::undefined())
                                        } else {
                                            Ok(rejected(&msg, ctx))
                                        }
                                    }
                                }
                            },
                        ),
                        "readFile",
                        3,
                        ctx,
                    );

                    let read_file_sync_val = to_js_fn(
                        NativeFunction::from_closure(
                            |_this: &JsValue,
                             args: &[JsValue],
                             ctx: &mut Context|
                             -> JsResult<JsValue> {
                                let path = extract_path(args)?;
                                match std::fs::read(&path) {
                                    Ok(data) => Ok(encode_data(&data, None, ctx)),
                                    Err(e) => Err(js_err(&format!(
                                        "ENOENT: no such file or directory, open '{path}': {e}"
                                    ))),
                                }
                            },
                        ),
                        "readFileSync",
                        2,
                        ctx,
                    );

                    let write_file_val = to_js_fn(
                        NativeFunction::from_closure(
                            |_this: &JsValue,
                             args: &[JsValue],
                             ctx: &mut Context|
                             -> JsResult<JsValue> {
                                let path = extract_path(args)?;
                                let data = args.get(1).cloned().unwrap_or(JsValue::undefined());
                                let cb = extract_cb(args);
                                let bytes = extract_bytes(&data, ctx).unwrap_or_default();
                                match std::fs::write(&path, &bytes) {
                                    Ok(()) => {
                                        if let Some(ref c) = cb {
                                            call_cb_ok(c, JsValue::undefined(), ctx);
                                            Ok(JsValue::undefined())
                                        } else {
                                            Ok(resolved(JsValue::undefined(), ctx))
                                        }
                                    }
                                    Err(e) => {
                                        let msg = format!("writeFile: {e}");
                                        if let Some(ref c) = cb {
                                            call_cb_err(c, &msg, ctx);
                                            Ok(JsValue::undefined())
                                        } else {
                                            Ok(rejected(&msg, ctx))
                                        }
                                    }
                                }
                            },
                        ),
                        "writeFile",
                        4,
                        ctx,
                    );

                    let write_file_sync_val = to_js_fn(
                        NativeFunction::from_closure(
                            |_this: &JsValue,
                             args: &[JsValue],
                             ctx: &mut Context|
                             -> JsResult<JsValue> {
                                let path = extract_path(args)?;
                                let data = args.get(1).cloned().unwrap_or(JsValue::undefined());
                                let bytes = extract_bytes(&data, ctx).unwrap_or_default();
                                std::fs::write(&path, &bytes)
                                    .map_err(|e| js_err(&format!("writeFileSync: {e}")))?;
                                Ok(JsValue::undefined())
                            },
                        ),
                        "writeFileSync",
                        3,
                        ctx,
                    );

                    let append_file_val = to_js_fn(
                        NativeFunction::from_closure(
                            |_this: &JsValue,
                             args: &[JsValue],
                             ctx: &mut Context|
                             -> JsResult<JsValue> {
                                let path = extract_path(args)?;
                                let data = args.get(1).cloned().unwrap_or(JsValue::undefined());
                                let cb = extract_cb(args);
                                let bytes = extract_bytes(&data, ctx).unwrap_or_default();
                                let result = std::fs::OpenOptions::new()
                                    .append(true)
                                    .create(true)
                                    .open(&path)
                                    .and_then(|mut f| {
                                        use std::io::Write;
                                        f.write_all(&bytes)
                                    });
                                match result {
                                    Ok(()) => {
                                        if let Some(ref c) = cb {
                                            call_cb_ok(c, JsValue::undefined(), ctx);
                                            Ok(JsValue::undefined())
                                        } else {
                                            Ok(resolved(JsValue::undefined(), ctx))
                                        }
                                    }
                                    Err(e) => {
                                        let msg = format!("appendFile: {e}");
                                        if let Some(ref c) = cb {
                                            call_cb_err(c, &msg, ctx);
                                            Ok(JsValue::undefined())
                                        } else {
                                            Ok(rejected(&msg, ctx))
                                        }
                                    }
                                }
                            },
                        ),
                        "appendFile",
                        4,
                        ctx,
                    );

                    let append_file_sync_val = to_js_fn(
                        NativeFunction::from_closure(
                            |_this: &JsValue,
                             args: &[JsValue],
                             ctx: &mut Context|
                             -> JsResult<JsValue> {
                                let path = extract_path(args)?;
                                let data = args.get(1).cloned().unwrap_or(JsValue::undefined());
                                let bytes = extract_bytes(&data, ctx).unwrap_or_default();
                                std::fs::OpenOptions::new()
                                    .append(true)
                                    .create(true)
                                    .open(&path)
                                    .and_then(|mut f| {
                                        use std::io::Write;
                                        f.write_all(&bytes)
                                    })
                                    .map_err(|e| js_err(&format!("appendFileSync: {e}")))?;
                                Ok(JsValue::undefined())
                            },
                        ),
                        "appendFileSync",
                        3,
                        ctx,
                    );

                    let mkdir_val = to_js_fn(
                        NativeFunction::from_closure(
                            |_this: &JsValue,
                             args: &[JsValue],
                             ctx: &mut Context|
                             -> JsResult<JsValue> {
                                let path = extract_path(args)?;
                                let recursive = extract_recursive(args, ctx);
                                let cb = extract_cb(args);
                                let result = if recursive {
                                    std::fs::create_dir_all(&path)
                                } else {
                                    std::fs::create_dir(&path)
                                };
                                match result {
                                    Ok(()) => {
                                        if let Some(ref c) = cb {
                                            call_cb_ok(c, JsValue::undefined(), ctx);
                                            Ok(JsValue::undefined())
                                        } else {
                                            Ok(resolved(JsValue::undefined(), ctx))
                                        }
                                    }
                                    Err(e) => {
                                        let msg = format!("mkdir: {e}");
                                        if let Some(ref c) = cb {
                                            call_cb_err(c, &msg, ctx);
                                            Ok(JsValue::undefined())
                                        } else {
                                            Ok(rejected(&msg, ctx))
                                        }
                                    }
                                }
                            },
                        ),
                        "mkdir",
                        2,
                        ctx,
                    );

                    let mkdir_sync_val = to_js_fn(
                        NativeFunction::from_closure(
                            |_this: &JsValue,
                             args: &[JsValue],
                             ctx: &mut Context|
                             -> JsResult<JsValue> {
                                let path = extract_path(args)?;
                                let recursive = extract_recursive(args, ctx);
                                if recursive {
                                    std::fs::create_dir_all(&path)
                                        .map_err(|e| js_err(&format!("mkdirSync: {e}")))?;
                                } else {
                                    std::fs::create_dir(&path)
                                        .map_err(|e| js_err(&format!("mkdirSync: {e}")))?;
                                }
                                Ok(JsValue::undefined())
                            },
                        ),
                        "mkdirSync",
                        2,
                        ctx,
                    );

                    let readdir_val = to_js_fn(
                        NativeFunction::from_closure(
                            |_this: &JsValue,
                             args: &[JsValue],
                             ctx: &mut Context|
                             -> JsResult<JsValue> {
                                let path = extract_path(args)?;
                                let cb = extract_cb(args);
                                match std::fs::read_dir(&path) {
                                    Ok(entries) => {
                                        let arr = JsArray::new(ctx);
                                        for entry in entries.flatten() {
                                            if let Some(name) = entry.file_name().to_str() {
                                                let _ = arr
                                                    .push(JsValue::from(JsString::from(name)), ctx);
                                            }
                                        }
                                        if let Some(ref c) = cb {
                                            call_cb_ok(c, arr.into(), ctx);
                                            Ok(JsValue::undefined())
                                        } else {
                                            Ok(resolved(arr.into(), ctx))
                                        }
                                    }
                                    Err(e) => {
                                        let msg = format!("readdir: {e}");
                                        if let Some(ref c) = cb {
                                            call_cb_err(c, &msg, ctx);
                                            Ok(JsValue::undefined())
                                        } else {
                                            Ok(rejected(&msg, ctx))
                                        }
                                    }
                                }
                            },
                        ),
                        "readdir",
                        2,
                        ctx,
                    );

                    let readdir_sync_val = to_js_fn(
                        NativeFunction::from_closure(
                            |_this: &JsValue,
                             args: &[JsValue],
                             ctx: &mut Context|
                             -> JsResult<JsValue> {
                                let path = extract_path(args)?;
                                match std::fs::read_dir(&path) {
                                    Ok(entries) => {
                                        let arr = JsArray::new(ctx);
                                        for entry in entries.flatten() {
                                            if let Some(name) = entry.file_name().to_str() {
                                                let _ = arr
                                                    .push(JsValue::from(JsString::from(name)), ctx);
                                            }
                                        }
                                        Ok(arr.into())
                                    }
                                    Err(e) => Err(js_err(&format!("readdirSync: {e}"))),
                                }
                            },
                        ),
                        "readdirSync",
                        1,
                        ctx,
                    );

                    let rmdir_val = to_js_fn(
                        NativeFunction::from_closure(
                            |_this: &JsValue,
                             args: &[JsValue],
                             ctx: &mut Context|
                             -> JsResult<JsValue> {
                                let path = extract_path(args)?;
                                let recursive = extract_recursive(args, ctx);
                                let cb = extract_cb(args);
                                let result = if recursive {
                                    std::fs::remove_dir_all(&path)
                                } else {
                                    std::fs::remove_dir(&path)
                                };
                                match result {
                                    Ok(()) => {
                                        if let Some(ref c) = cb {
                                            call_cb_ok(c, JsValue::undefined(), ctx);
                                            Ok(JsValue::undefined())
                                        } else {
                                            Ok(resolved(JsValue::undefined(), ctx))
                                        }
                                    }
                                    Err(e) => {
                                        let msg = format!("rmdir: {e}");
                                        if let Some(ref c) = cb {
                                            call_cb_err(c, &msg, ctx);
                                            Ok(JsValue::undefined())
                                        } else {
                                            Ok(rejected(&msg, ctx))
                                        }
                                    }
                                }
                            },
                        ),
                        "rmdir",
                        3,
                        ctx,
                    );

                    let rmdir_sync_val = to_js_fn(
                        NativeFunction::from_closure(
                            |_this: &JsValue,
                             args: &[JsValue],
                             ctx: &mut Context|
                             -> JsResult<JsValue> {
                                let path = extract_path(args)?;
                                let recursive = extract_recursive(args, ctx);
                                if recursive {
                                    std::fs::remove_dir_all(&path)
                                        .map_err(|e| js_err(&format!("rmdirSync: {e}")))?;
                                } else {
                                    std::fs::remove_dir(&path)
                                        .map_err(|e| js_err(&format!("rmdirSync: {e}")))?;
                                }
                                Ok(JsValue::undefined())
                            },
                        ),
                        "rmdirSync",
                        2,
                        ctx,
                    );

                    let unlink_val = to_js_fn(
                        NativeFunction::from_closure(
                            |_this: &JsValue,
                             args: &[JsValue],
                             ctx: &mut Context|
                             -> JsResult<JsValue> {
                                let path = extract_path(args)?;
                                let cb = extract_cb(args);
                                match std::fs::remove_file(&path) {
                                    Ok(()) => {
                                        if let Some(ref c) = cb {
                                            call_cb_ok(c, JsValue::undefined(), ctx);
                                            Ok(JsValue::undefined())
                                        } else {
                                            Ok(resolved(JsValue::undefined(), ctx))
                                        }
                                    }
                                    Err(e) => {
                                        let msg = format!("unlink: {e}");
                                        if let Some(ref c) = cb {
                                            call_cb_err(c, &msg, ctx);
                                            Ok(JsValue::undefined())
                                        } else {
                                            Ok(rejected(&msg, ctx))
                                        }
                                    }
                                }
                            },
                        ),
                        "unlink",
                        2,
                        ctx,
                    );

                    let unlink_sync_val = to_js_fn(
                        NativeFunction::from_closure(
                            |_this: &JsValue,
                             args: &[JsValue],
                             _ctx: &mut Context|
                             -> JsResult<JsValue> {
                                let path = extract_path(args)?;
                                std::fs::remove_file(&path)
                                    .map_err(|e| js_err(&format!("unlinkSync: {e}")))?;
                                Ok(JsValue::undefined())
                            },
                        ),
                        "unlinkSync",
                        1,
                        ctx,
                    );

                    let stat_val = to_js_fn(
                        NativeFunction::from_closure(
                            |_this: &JsValue,
                             args: &[JsValue],
                             ctx: &mut Context|
                             -> JsResult<JsValue> {
                                let path = extract_path(args)?;
                                let cb = extract_cb(args);
                                match std::fs::metadata(&path) {
                                    Ok(md) => {
                                        let stats = metadata_to_stats(&md, ctx)?;
                                        if let Some(ref c) = cb {
                                            call_cb_ok(c, stats, ctx);
                                            Ok(JsValue::undefined())
                                        } else {
                                            Ok(resolved(stats, ctx))
                                        }
                                    }
                                    Err(e) => {
                                        let msg = format!("stat: {e}");
                                        if let Some(ref c) = cb {
                                            call_cb_err(c, &msg, ctx);
                                            Ok(JsValue::undefined())
                                        } else {
                                            Ok(rejected(&msg, ctx))
                                        }
                                    }
                                }
                            },
                        ),
                        "stat",
                        2,
                        ctx,
                    );

                    let stat_sync_val = to_js_fn(
                        NativeFunction::from_closure(
                            |_this: &JsValue,
                             args: &[JsValue],
                             ctx: &mut Context|
                             -> JsResult<JsValue> {
                                let path = extract_path(args)?;
                                match std::fs::metadata(&path) {
                                    Ok(md) => metadata_to_stats(&md, ctx),
                                    Err(e) => Err(js_err(&format!("statSync: {e}"))),
                                }
                            },
                        ),
                        "statSync",
                        1,
                        ctx,
                    );

                    let lstat_val = to_js_fn(
                        NativeFunction::from_closure(
                            |_this: &JsValue,
                             args: &[JsValue],
                             ctx: &mut Context|
                             -> JsResult<JsValue> {
                                let path = extract_path(args)?;
                                let cb = extract_cb(args);
                                match std::fs::symlink_metadata(&path) {
                                    Ok(md) => {
                                        let stats = metadata_to_stats(&md, ctx)?;
                                        if let Some(ref c) = cb {
                                            call_cb_ok(c, stats, ctx);
                                            Ok(JsValue::undefined())
                                        } else {
                                            Ok(resolved(stats, ctx))
                                        }
                                    }
                                    Err(e) => {
                                        let msg = format!("lstat: {e}");
                                        if let Some(ref c) = cb {
                                            call_cb_err(c, &msg, ctx);
                                            Ok(JsValue::undefined())
                                        } else {
                                            Ok(rejected(&msg, ctx))
                                        }
                                    }
                                }
                            },
                        ),
                        "lstat",
                        2,
                        ctx,
                    );

                    let lstat_sync_val = to_js_fn(
                        NativeFunction::from_closure(
                            |_this: &JsValue,
                             args: &[JsValue],
                             ctx: &mut Context|
                             -> JsResult<JsValue> {
                                let path = extract_path(args)?;
                                match std::fs::symlink_metadata(&path) {
                                    Ok(md) => metadata_to_stats(&md, ctx),
                                    Err(e) => Err(js_err(&format!("lstatSync: {e}"))),
                                }
                            },
                        ),
                        "lstatSync",
                        1,
                        ctx,
                    );

                    let access_val = to_js_fn(
                        NativeFunction::from_closure(
                            |_this: &JsValue,
                             args: &[JsValue],
                             ctx: &mut Context|
                             -> JsResult<JsValue> {
                                let path = extract_path(args)?;
                                let cb = extract_cb(args);
                                if !std::path::Path::new(&path).exists() {
                                    let msg = format!(
                                        "ENOENT: no such file or directory, access '{path}'"
                                    );
                                    if let Some(ref c) = cb {
                                        call_cb_err(c, &msg, ctx);
                                        Ok(JsValue::undefined())
                                    } else {
                                        Ok(rejected(&msg, ctx))
                                    }
                                } else if let Some(ref c) = cb {
                                    call_cb_ok(c, JsValue::undefined(), ctx);
                                    Ok(JsValue::undefined())
                                } else {
                                    Ok(resolved(JsValue::undefined(), ctx))
                                }
                            },
                        ),
                        "access",
                        3,
                        ctx,
                    );

                    let access_sync_val = to_js_fn(
                        NativeFunction::from_closure(
                            |_this: &JsValue,
                             args: &[JsValue],
                             _ctx: &mut Context|
                             -> JsResult<JsValue> {
                                let path = extract_path(args)?;
                                if !std::path::Path::new(&path).exists() {
                                    Err(js_err(&format!(
                                        "ENOENT: no such file or directory, access '{path}'"
                                    )))
                                } else {
                                    Ok(JsValue::undefined())
                                }
                            },
                        ),
                        "accessSync",
                        1,
                        ctx,
                    );

                    let chmod_val = to_js_fn(
                        NativeFunction::from_closure(
                            |_this: &JsValue,
                             args: &[JsValue],
                             ctx: &mut Context|
                             -> JsResult<JsValue> {
                                let path = extract_path(args)?;
                                let cb = extract_cb(args);
                                let mode = args
                                    .get(1)
                                    .and_then(|v: &JsValue| v.as_number())
                                    .unwrap_or(420.0)
                                    as u32;
                                #[cfg(unix)]
                                {
                                    use std::os::unix::fs::PermissionsExt;
                                    match std::fs::set_permissions(
                                        &path,
                                        std::fs::Permissions::from_mode(mode),
                                    ) {
                                        Ok(()) => {
                                            if let Some(ref c) = cb {
                                                call_cb_ok(c, JsValue::undefined(), ctx);
                                                Ok(JsValue::undefined())
                                            } else {
                                                Ok(resolved(JsValue::undefined(), ctx))
                                            }
                                        }
                                        Err(e) => {
                                            let msg = format!("chmod: {e}");
                                            if let Some(ref c) = cb {
                                                call_cb_err(c, &msg, ctx);
                                                Ok(JsValue::undefined())
                                            } else {
                                                Ok(rejected(&msg, ctx))
                                            }
                                        }
                                    }
                                }
                                #[cfg(not(unix))]
                                {
                                    let _ = mode;
                                    let msg = "chmod: not supported on this platform".to_string();
                                    if let Some(ref c) = cb {
                                        call_cb_err(c, &msg, ctx);
                                        Ok(JsValue::undefined())
                                    } else {
                                        Ok(rejected(&msg, ctx))
                                    }
                                }
                            },
                        ),
                        "chmod",
                        3,
                        ctx,
                    );

                    let chmod_sync_val = to_js_fn(
                        NativeFunction::from_closure(
                            |_this: &JsValue,
                             args: &[JsValue],
                             _ctx: &mut Context|
                             -> JsResult<JsValue> {
                                let path = extract_path(args)?;
                                let mode = args
                                    .get(1)
                                    .and_then(|v: &JsValue| v.as_number())
                                    .unwrap_or(420.0)
                                    as u32;
                                #[cfg(unix)]
                                {
                                    use std::os::unix::fs::PermissionsExt;
                                    std::fs::set_permissions(
                                        &path,
                                        std::fs::Permissions::from_mode(mode),
                                    )
                                    .map_err(|e| js_err(&format!("chmodSync: {e}")))?;
                                    Ok(JsValue::undefined())
                                }
                                #[cfg(not(unix))]
                                {
                                    let _ = (path, mode);
                                    Err(js_err("chmodSync: not supported on this platform"))
                                }
                            },
                        ),
                        "chmodSync",
                        2,
                        ctx,
                    );

                    let rename_val = to_js_fn(
                        NativeFunction::from_closure(
                            |_this: &JsValue,
                             args: &[JsValue],
                             ctx: &mut Context|
                             -> JsResult<JsValue> {
                                let (old, new) = extract_two_paths(args)?;
                                let cb = extract_cb(args);
                                match std::fs::rename(&old, &new) {
                                    Ok(()) => {
                                        if let Some(ref c) = cb {
                                            call_cb_ok(c, JsValue::undefined(), ctx);
                                            Ok(JsValue::undefined())
                                        } else {
                                            Ok(resolved(JsValue::undefined(), ctx))
                                        }
                                    }
                                    Err(e) => {
                                        let msg = format!("rename: {e}");
                                        if let Some(ref c) = cb {
                                            call_cb_err(c, &msg, ctx);
                                            Ok(JsValue::undefined())
                                        } else {
                                            Ok(rejected(&msg, ctx))
                                        }
                                    }
                                }
                            },
                        ),
                        "rename",
                        3,
                        ctx,
                    );

                    let rename_sync_val = to_js_fn(
                        NativeFunction::from_closure(
                            |_this: &JsValue,
                             args: &[JsValue],
                             _ctx: &mut Context|
                             -> JsResult<JsValue> {
                                let (old, new) = extract_two_paths(args)?;
                                std::fs::rename(&old, &new)
                                    .map_err(|e| js_err(&format!("renameSync: {e}")))?;
                                Ok(JsValue::undefined())
                            },
                        ),
                        "renameSync",
                        2,
                        ctx,
                    );

                    let copy_file_val = to_js_fn(
                        NativeFunction::from_closure(
                            |_this: &JsValue,
                             args: &[JsValue],
                             ctx: &mut Context|
                             -> JsResult<JsValue> {
                                let (src, dst) = extract_two_paths(args)?;
                                let cb = extract_cb(args);
                                match std::fs::copy(&src, &dst) {
                                    Ok(_) => {
                                        if let Some(ref c) = cb {
                                            call_cb_ok(c, JsValue::undefined(), ctx);
                                            Ok(JsValue::undefined())
                                        } else {
                                            Ok(resolved(JsValue::undefined(), ctx))
                                        }
                                    }
                                    Err(e) => {
                                        let msg = format!("copyFile: {e}");
                                        if let Some(ref c) = cb {
                                            call_cb_err(c, &msg, ctx);
                                            Ok(JsValue::undefined())
                                        } else {
                                            Ok(rejected(&msg, ctx))
                                        }
                                    }
                                }
                            },
                        ),
                        "copyFile",
                        3,
                        ctx,
                    );

                    let copy_file_sync_val = to_js_fn(
                        NativeFunction::from_closure(
                            |_this: &JsValue,
                             args: &[JsValue],
                             _ctx: &mut Context|
                             -> JsResult<JsValue> {
                                let (src, dst) = extract_two_paths(args)?;
                                std::fs::copy(&src, &dst)
                                    .map_err(|e| js_err(&format!("copyFileSync: {e}")))?;
                                Ok(JsValue::undefined())
                            },
                        ),
                        "copyFileSync",
                        2,
                        ctx,
                    );

                    let exists_sync_val = to_js_fn(
                        NativeFunction::from_closure(
                            |_this: &JsValue,
                             args: &[JsValue],
                             _ctx: &mut Context|
                             -> JsResult<JsValue> {
                                let path = extract_path(args)?;
                                Ok(JsValue::from(std::path::Path::new(&path).exists()))
                            },
                        ),
                        "existsSync",
                        1,
                        ctx,
                    );

                    let realpath_val = to_js_fn(
                        NativeFunction::from_closure(
                            |_this: &JsValue,
                             args: &[JsValue],
                             ctx: &mut Context|
                             -> JsResult<JsValue> {
                                let path = extract_path(args)?;
                                let cb = extract_cb(args);
                                match std::fs::canonicalize(&path) {
                                    Ok(p) => {
                                        let s = JsString::from(p.to_string_lossy().to_string());
                                        if let Some(ref c) = cb {
                                            call_cb_ok(c, JsValue::from(s), ctx);
                                            Ok(JsValue::undefined())
                                        } else {
                                            Ok(resolved(JsValue::from(s), ctx))
                                        }
                                    }
                                    Err(e) => {
                                        let msg = format!("realpath: {e}");
                                        if let Some(ref c) = cb {
                                            call_cb_err(c, &msg, ctx);
                                            Ok(JsValue::undefined())
                                        } else {
                                            Ok(rejected(&msg, ctx))
                                        }
                                    }
                                }
                            },
                        ),
                        "realpath",
                        3,
                        ctx,
                    );

                    let realpath_sync_val = to_js_fn(
                        NativeFunction::from_closure(
                            |_this: &JsValue,
                             args: &[JsValue],
                             _ctx: &mut Context|
                             -> JsResult<JsValue> {
                                let path = extract_path(args)?;
                                match std::fs::canonicalize(&path) {
                                    Ok(p) => Ok(JsValue::from(JsString::from(
                                        p.to_string_lossy().to_string(),
                                    ))),
                                    Err(e) => Err(js_err(&format!("realpathSync: {e}"))),
                                }
                            },
                        ),
                        "realpathSync",
                        1,
                        ctx,
                    );

                    let symlink_val = to_js_fn(
                        NativeFunction::from_closure(
                            |_this: &JsValue,
                             args: &[JsValue],
                             ctx: &mut Context|
                             -> JsResult<JsValue> {
                                let (target, path) = extract_two_paths(args)?;
                                let cb = extract_cb(args);
                                #[cfg(unix)]
                                {
                                    match std::os::unix::fs::symlink(&target, &path) {
                                        Ok(()) => {
                                            if let Some(ref c) = cb {
                                                call_cb_ok(c, JsValue::undefined(), ctx);
                                                Ok(JsValue::undefined())
                                            } else {
                                                Ok(resolved(JsValue::undefined(), ctx))
                                            }
                                        }
                                        Err(e) => {
                                            let msg = format!("symlink: {e}");
                                            if let Some(ref c) = cb {
                                                call_cb_err(c, &msg, ctx);
                                                Ok(JsValue::undefined())
                                            } else {
                                                Ok(rejected(&msg, ctx))
                                            }
                                        }
                                    }
                                }
                                #[cfg(not(unix))]
                                {
                                    let msg = "symlink: not supported on this platform".to_string();
                                    if let Some(ref c) = cb {
                                        call_cb_err(c, &msg, ctx);
                                        Ok(JsValue::undefined())
                                    } else {
                                        Ok(rejected(&msg, ctx))
                                    }
                                }
                            },
                        ),
                        "symlink",
                        4,
                        ctx,
                    );

                    let symlink_sync_val = to_js_fn(
                        NativeFunction::from_closure(
                            |_this: &JsValue,
                             args: &[JsValue],
                             _ctx: &mut Context|
                             -> JsResult<JsValue> {
                                let (target, path) = extract_two_paths(args)?;
                                #[cfg(unix)]
                                {
                                    std::os::unix::fs::symlink(&target, &path)
                                        .map_err(|e| js_err(&format!("symlinkSync: {e}")))?;
                                    Ok(JsValue::undefined())
                                }
                                #[cfg(not(unix))]
                                {
                                    let _ = (target, path);
                                    Err(js_err("symlinkSync: not supported on this platform"))
                                }
                            },
                        ),
                        "symlinkSync",
                        2,
                        ctx,
                    );

                    let link_val = to_js_fn(
                        NativeFunction::from_closure(
                            |_this: &JsValue,
                             args: &[JsValue],
                             ctx: &mut Context|
                             -> JsResult<JsValue> {
                                let (existing, new_path) = extract_two_paths(args)?;
                                let cb = extract_cb(args);
                                match std::fs::hard_link(&existing, &new_path) {
                                    Ok(()) => {
                                        if let Some(ref c) = cb {
                                            call_cb_ok(c, JsValue::undefined(), ctx);
                                            Ok(JsValue::undefined())
                                        } else {
                                            Ok(resolved(JsValue::undefined(), ctx))
                                        }
                                    }
                                    Err(e) => {
                                        let msg = format!("link: {e}");
                                        if let Some(ref c) = cb {
                                            call_cb_err(c, &msg, ctx);
                                            Ok(JsValue::undefined())
                                        } else {
                                            Ok(rejected(&msg, ctx))
                                        }
                                    }
                                }
                            },
                        ),
                        "link",
                        3,
                        ctx,
                    );

                    let link_sync_val = to_js_fn(
                        NativeFunction::from_closure(
                            |_this: &JsValue,
                             args: &[JsValue],
                             _ctx: &mut Context|
                             -> JsResult<JsValue> {
                                let (existing, new_path) = extract_two_paths(args)?;
                                std::fs::hard_link(&existing, &new_path)
                                    .map_err(|e| js_err(&format!("linkSync: {e}")))?;
                                Ok(JsValue::undefined())
                            },
                        ),
                        "linkSync",
                        2,
                        ctx,
                    );

                    let truncate_val = to_js_fn(
                        NativeFunction::from_closure(
                            |_this: &JsValue,
                             args: &[JsValue],
                             ctx: &mut Context|
                             -> JsResult<JsValue> {
                                let path = extract_path(args)?;
                                let len =
                                    args.get(1).and_then(|v| v.as_number()).unwrap_or(0.0) as u64;
                                let cb = extract_cb(args);
                                let result = std::fs::OpenOptions::new()
                                    .write(true)
                                    .open(&path)
                                    .and_then(|f| f.set_len(len));
                                match result {
                                    Ok(()) => {
                                        if let Some(ref c) = cb {
                                            call_cb_ok(c, JsValue::undefined(), ctx);
                                            Ok(JsValue::undefined())
                                        } else {
                                            Ok(resolved(JsValue::undefined(), ctx))
                                        }
                                    }
                                    Err(e) => {
                                        let msg = format!("truncate: {e}");
                                        if let Some(ref c) = cb {
                                            call_cb_err(c, &msg, ctx);
                                            Ok(JsValue::undefined())
                                        } else {
                                            Ok(rejected(&msg, ctx))
                                        }
                                    }
                                }
                            },
                        ),
                        "truncate",
                        3,
                        ctx,
                    );

                    let truncate_sync_val = to_js_fn(
                        NativeFunction::from_closure(
                            |_this: &JsValue,
                             args: &[JsValue],
                             _ctx: &mut Context|
                             -> JsResult<JsValue> {
                                let path = extract_path(args)?;
                                let len =
                                    args.get(1).and_then(|v| v.as_number()).unwrap_or(0.0) as u64;
                                std::fs::OpenOptions::new()
                                    .write(true)
                                    .open(&path)
                                    .and_then(|f| f.set_len(len))
                                    .map_err(|e| js_err(&format!("truncateSync: {e}")))?;
                                Ok(JsValue::undefined())
                            },
                        ),
                        "truncateSync",
                        2,
                        ctx,
                    );

                    // ── constants ───────────────────────────────────────────────────
                    let constants_obj = JsObject::with_object_proto(ctx.intrinsics());
                    let _ = constants_obj.set(js_string!("F_OK"), JsValue::from(0), false, ctx);
                    let _ = constants_obj.set(js_string!("R_OK"), JsValue::from(4), false, ctx);
                    let _ = constants_obj.set(js_string!("W_OK"), JsValue::from(2), false, ctx);
                    let _ = constants_obj.set(js_string!("X_OK"), JsValue::from(1), false, ctx);
                    let _ = constants_obj.set(
                        js_string!("COPYFILE_EXCL"),
                        JsValue::from(1),
                        false,
                        ctx,
                    );
                    let _ = constants_obj.set(js_string!("O_RDONLY"), JsValue::from(0), false, ctx);
                    let _ = constants_obj.set(js_string!("O_WRONLY"), JsValue::from(1), false, ctx);
                    let _ = constants_obj.set(js_string!("O_RDWR"), JsValue::from(2), false, ctx);
                    let _ = constants_obj.set(js_string!("O_CREAT"), JsValue::from(64), false, ctx);
                    let _ = constants_obj.set(js_string!("O_EXCL"), JsValue::from(128), false, ctx);
                    let _ =
                        constants_obj.set(js_string!("O_TRUNC"), JsValue::from(512), false, ctx);
                    let _ =
                        constants_obj.set(js_string!("O_APPEND"), JsValue::from(1024), false, ctx);
                    #[cfg(unix)]
                    {
                        let _ = constants_obj.set(
                            js_string!("S_IRUSR"),
                            JsValue::from(0o400),
                            false,
                            ctx,
                        );
                        let _ = constants_obj.set(
                            js_string!("S_IWUSR"),
                            JsValue::from(0o200),
                            false,
                            ctx,
                        );
                        let _ = constants_obj.set(
                            js_string!("S_IXUSR"),
                            JsValue::from(0o100),
                            false,
                            ctx,
                        );
                        let _ = constants_obj.set(
                            js_string!("S_IRGRP"),
                            JsValue::from(0o040),
                            false,
                            ctx,
                        );
                        let _ = constants_obj.set(
                            js_string!("S_IWGRP"),
                            JsValue::from(0o020),
                            false,
                            ctx,
                        );
                        let _ = constants_obj.set(
                            js_string!("S_IXGRP"),
                            JsValue::from(0o010),
                            false,
                            ctx,
                        );
                        let _ = constants_obj.set(
                            js_string!("S_IROTH"),
                            JsValue::from(0o004),
                            false,
                            ctx,
                        );
                        let _ = constants_obj.set(
                            js_string!("S_IWOTH"),
                            JsValue::from(0o002),
                            false,
                            ctx,
                        );
                        let _ = constants_obj.set(
                            js_string!("S_IXOTH"),
                            JsValue::from(0o001),
                            false,
                            ctx,
                        );
                    }
                    let constants_val: JsValue = constants_obj.into();

                    // ── promises ───────────────────────────────────────────────────
                    let promises_obj = JsObject::with_object_proto(ctx.intrinsics());
                    let _ =
                        promises_obj.set(js_string!("readFile"), read_file_val.clone(), false, ctx);
                    let _ = promises_obj.set(
                        js_string!("writeFile"),
                        write_file_val.clone(),
                        false,
                        ctx,
                    );
                    let _ = promises_obj.set(
                        js_string!("appendFile"),
                        append_file_val.clone(),
                        false,
                        ctx,
                    );
                    let _ = promises_obj.set(js_string!("mkdir"), mkdir_val.clone(), false, ctx);
                    let _ =
                        promises_obj.set(js_string!("readdir"), readdir_val.clone(), false, ctx);
                    let _ = promises_obj.set(js_string!("rmdir"), rmdir_val.clone(), false, ctx);
                    let _ = promises_obj.set(js_string!("unlink"), unlink_val.clone(), false, ctx);
                    let _ = promises_obj.set(js_string!("stat"), stat_val.clone(), false, ctx);
                    let _ = promises_obj.set(js_string!("lstat"), lstat_val.clone(), false, ctx);
                    let _ = promises_obj.set(js_string!("access"), access_val.clone(), false, ctx);
                    let _ = promises_obj.set(js_string!("chmod"), chmod_val.clone(), false, ctx);
                    let _ = promises_obj.set(js_string!("rename"), rename_val.clone(), false, ctx);
                    let _ =
                        promises_obj.set(js_string!("copyFile"), copy_file_val.clone(), false, ctx);
                    let _ =
                        promises_obj.set(js_string!("realpath"), realpath_val.clone(), false, ctx);
                    let _ =
                        promises_obj.set(js_string!("symlink"), symlink_val.clone(), false, ctx);
                    let _ = promises_obj.set(js_string!("link"), link_val.clone(), false, ctx);
                    let _ =
                        promises_obj.set(js_string!("truncate"), truncate_val.clone(), false, ctx);
                    let promises_val: JsValue = promises_obj.into();

                    // ── Set module exports ─────────────────────────────────────────
                    let _ = m.set_export(&js_string!("readFile"), read_file_val.clone());
                    let _ = m.set_export(&js_string!("readFileSync"), read_file_sync_val.clone());
                    let _ = m.set_export(&js_string!("writeFile"), write_file_val.clone());
                    let _ = m.set_export(&js_string!("writeFileSync"), write_file_sync_val.clone());
                    let _ = m.set_export(&js_string!("appendFile"), append_file_val.clone());
                    let _ =
                        m.set_export(&js_string!("appendFileSync"), append_file_sync_val.clone());
                    let _ = m.set_export(&js_string!("mkdir"), mkdir_val.clone());
                    let _ = m.set_export(&js_string!("mkdirSync"), mkdir_sync_val.clone());
                    let _ = m.set_export(&js_string!("readdir"), readdir_val.clone());
                    let _ = m.set_export(&js_string!("readdirSync"), readdir_sync_val.clone());
                    let _ = m.set_export(&js_string!("rmdir"), rmdir_val.clone());
                    let _ = m.set_export(&js_string!("rmdirSync"), rmdir_sync_val.clone());
                    let _ = m.set_export(&js_string!("rm"), rmdir_val.clone());
                    let _ = m.set_export(&js_string!("rmSync"), rmdir_sync_val.clone());
                    let _ = m.set_export(&js_string!("unlink"), unlink_val.clone());
                    let _ = m.set_export(&js_string!("unlinkSync"), unlink_sync_val.clone());
                    let _ = m.set_export(&js_string!("stat"), stat_val.clone());
                    let _ = m.set_export(&js_string!("statSync"), stat_sync_val.clone());
                    let _ = m.set_export(&js_string!("lstat"), lstat_val.clone());
                    let _ = m.set_export(&js_string!("lstatSync"), lstat_sync_val.clone());
                    let _ = m.set_export(&js_string!("access"), access_val.clone());
                    let _ = m.set_export(&js_string!("accessSync"), access_sync_val.clone());
                    let _ = m.set_export(&js_string!("chmod"), chmod_val.clone());
                    let _ = m.set_export(&js_string!("chmodSync"), chmod_sync_val.clone());
                    let _ = m.set_export(&js_string!("rename"), rename_val.clone());
                    let _ = m.set_export(&js_string!("renameSync"), rename_sync_val.clone());
                    let _ = m.set_export(&js_string!("copyFile"), copy_file_val.clone());
                    let _ = m.set_export(&js_string!("copyFileSync"), copy_file_sync_val.clone());
                    let _ = m.set_export(&js_string!("existsSync"), exists_sync_val.clone());
                    let _ = m.set_export(&js_string!("realpath"), realpath_val.clone());
                    let _ = m.set_export(&js_string!("realpathSync"), realpath_sync_val.clone());
                    let _ = m.set_export(&js_string!("symlink"), symlink_val.clone());
                    let _ = m.set_export(&js_string!("symlinkSync"), symlink_sync_val.clone());
                    let _ = m.set_export(&js_string!("link"), link_val.clone());
                    let _ = m.set_export(&js_string!("linkSync"), link_sync_val.clone());
                    let _ = m.set_export(&js_string!("truncate"), truncate_val.clone());
                    let _ = m.set_export(&js_string!("truncateSync"), truncate_sync_val.clone());
                    let _ = m.set_export(&js_string!("watch"), JsValue::undefined());
                    let _ = m.set_export(&js_string!("constants"), constants_val.clone());
                    let _ = m.set_export(&js_string!("promises"), promises_val.clone());

                    // default export: object with all the same properties (like Node.js)
                    let default_obj = JsObject::with_object_proto(ctx.intrinsics());
                    let _ =
                        default_obj.set(js_string!("readFile"), read_file_val.clone(), false, ctx);
                    let _ = default_obj.set(
                        js_string!("readFileSync"),
                        read_file_sync_val.clone(),
                        false,
                        ctx,
                    );
                    let _ = default_obj.set(
                        js_string!("writeFile"),
                        write_file_val.clone(),
                        false,
                        ctx,
                    );
                    let _ = default_obj.set(
                        js_string!("writeFileSync"),
                        write_file_sync_val.clone(),
                        false,
                        ctx,
                    );
                    let _ = default_obj.set(
                        js_string!("appendFile"),
                        append_file_val.clone(),
                        false,
                        ctx,
                    );
                    let _ = default_obj.set(
                        js_string!("appendFileSync"),
                        append_file_sync_val.clone(),
                        false,
                        ctx,
                    );
                    let _ = default_obj.set(js_string!("mkdir"), mkdir_val.clone(), false, ctx);
                    let _ = default_obj.set(
                        js_string!("mkdirSync"),
                        mkdir_sync_val.clone(),
                        false,
                        ctx,
                    );
                    let _ = default_obj.set(js_string!("readdir"), readdir_val.clone(), false, ctx);
                    let _ = default_obj.set(
                        js_string!("readdirSync"),
                        readdir_sync_val.clone(),
                        false,
                        ctx,
                    );
                    let _ = default_obj.set(js_string!("rmdir"), rmdir_val.clone(), false, ctx);
                    let _ = default_obj.set(
                        js_string!("rmdirSync"),
                        rmdir_sync_val.clone(),
                        false,
                        ctx,
                    );
                    let _ = default_obj.set(js_string!("rm"), rmdir_val.clone(), false, ctx);
                    let _ =
                        default_obj.set(js_string!("rmSync"), rmdir_sync_val.clone(), false, ctx);
                    let _ = default_obj.set(js_string!("unlink"), unlink_val.clone(), false, ctx);
                    let _ = default_obj.set(
                        js_string!("unlinkSync"),
                        unlink_sync_val.clone(),
                        false,
                        ctx,
                    );
                    let _ = default_obj.set(js_string!("stat"), stat_val.clone(), false, ctx);
                    let _ =
                        default_obj.set(js_string!("statSync"), stat_sync_val.clone(), false, ctx);
                    let _ = default_obj.set(js_string!("lstat"), lstat_val.clone(), false, ctx);
                    let _ = default_obj.set(
                        js_string!("lstatSync"),
                        lstat_sync_val.clone(),
                        false,
                        ctx,
                    );
                    let _ = default_obj.set(js_string!("access"), access_val.clone(), false, ctx);
                    let _ = default_obj.set(
                        js_string!("accessSync"),
                        access_sync_val.clone(),
                        false,
                        ctx,
                    );
                    let _ = default_obj.set(js_string!("chmod"), chmod_val.clone(), false, ctx);
                    let _ = default_obj.set(
                        js_string!("chmodSync"),
                        chmod_sync_val.clone(),
                        false,
                        ctx,
                    );
                    let _ = default_obj.set(js_string!("rename"), rename_val.clone(), false, ctx);
                    let _ = default_obj.set(
                        js_string!("renameSync"),
                        rename_sync_val.clone(),
                        false,
                        ctx,
                    );
                    let _ =
                        default_obj.set(js_string!("copyFile"), copy_file_val.clone(), false, ctx);
                    let _ = default_obj.set(
                        js_string!("copyFileSync"),
                        copy_file_sync_val.clone(),
                        false,
                        ctx,
                    );
                    let _ = default_obj.set(
                        js_string!("existsSync"),
                        exists_sync_val.clone(),
                        false,
                        ctx,
                    );
                    let _ =
                        default_obj.set(js_string!("realpath"), realpath_val.clone(), false, ctx);
                    let _ = default_obj.set(
                        js_string!("realpathSync"),
                        realpath_sync_val.clone(),
                        false,
                        ctx,
                    );
                    let _ = default_obj.set(js_string!("symlink"), symlink_val.clone(), false, ctx);
                    let _ = default_obj.set(
                        js_string!("symlinkSync"),
                        symlink_sync_val.clone(),
                        false,
                        ctx,
                    );
                    let _ = default_obj.set(js_string!("link"), link_val.clone(), false, ctx);
                    let _ =
                        default_obj.set(js_string!("linkSync"), link_sync_val.clone(), false, ctx);
                    let _ =
                        default_obj.set(js_string!("truncate"), truncate_val.clone(), false, ctx);
                    let _ = default_obj.set(
                        js_string!("truncateSync"),
                        truncate_sync_val.clone(),
                        false,
                        ctx,
                    );
                    let _ =
                        default_obj.set(js_string!("constants"), constants_val.clone(), false, ctx);
                    let _ =
                        default_obj.set(js_string!("promises"), promises_val.clone(), false, ctx);
                    let _ = default_obj.set(js_string!("watch"), JsValue::undefined(), false, ctx);
                    let _ = m.set_export(&js_string!("default"), default_obj.into());

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
