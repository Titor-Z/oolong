use boa_engine::module::SyntheticModuleInitializer;
use boa_engine::{
    Context, JsNativeError, JsObject, JsResult, JsString, JsValue, Module, NativeFunction,
    js_string, object::FunctionObjectBuilder,
};

fn make_fn<F>(f: F, name: &str, len: usize, ctx: &mut Context) -> JsValue
where
    F: Fn(&JsValue, &[JsValue], &mut Context) -> JsResult<JsValue> + 'static,
{
    let native = unsafe { NativeFunction::from_closure(f) };
    FunctionObjectBuilder::new(ctx.realm(), native)
        .name(JsString::from(name))
        .length(len)
        .build()
        .into()
}

fn get_str_arg(args: &[JsValue]) -> JsResult<String> {
    args.first()
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .ok_or_else(|| {
            JsNativeError::typ()
                .with_message("Expected a string")
                .into()
        })
}

fn file_url_to_path_impl(
    _this: &JsValue,
    args: &[JsValue],
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let url_ctor = ctx
        .global_object()
        .get(js_string!("URL"), ctx)
        .map_err(|_| JsNativeError::typ().with_message("URL constructor not found"))?;

    let url_obj = if let Some(obj) = args.first().and_then(|v| v.as_object()) {
        let protocol = obj
            .get(js_string!("protocol"), ctx)
            .and_then(|v| v.to_string(ctx))
            .map(|s| s.to_std_string_escaped())
            .unwrap_or_default();
        if protocol != "file:" {
            return Err(JsNativeError::typ()
                .with_message("The URL must be of scheme file")
                .into());
        }
        obj.clone()
    } else {
        let url_str = get_str_arg(args)?;
        let ctor_obj = url_ctor
            .as_object()
            .ok_or_else(|| JsNativeError::typ().with_message("URL constructor not callable"))?;
        let instance = ctor_obj
            .construct(&[JsValue::from(js_string!(url_str.as_str()))], None, ctx)
            .map_err(|_| JsNativeError::typ().with_message("Failed to construct URL"))?;
        let protocol = instance
            .get(js_string!("protocol"), ctx)
            .and_then(|v| v.to_string(ctx))
            .map(|s| s.to_std_string_escaped())
            .unwrap_or_default();
        if protocol != "file:" {
            return Err(JsNativeError::typ()
                .with_message("The URL must be of scheme file")
                .into());
        }
        instance
    };

    let pathname = url_obj
        .get(js_string!("pathname"), ctx)
        .and_then(|v| v.to_string(ctx))
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();

    let decoded = percent_encoding::percent_decode(pathname.as_bytes())
        .decode_utf8()
        .map(|c| c.into_owned())
        .unwrap_or(pathname);
    Ok(JsValue::from(js_string!(decoded.as_str())))
}

fn path_to_file_url_impl(
    _this: &JsValue,
    args: &[JsValue],
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let path = get_str_arg(args)?;

    let resolved = if path.starts_with('/') {
        path.clone()
    } else {
        format!("/{}", path)
    };

    let url_str = format!("file://{}", resolved);

    let url_ctor = ctx
        .global_object()
        .get(js_string!("URL"), ctx)
        .map_err(|_| JsNativeError::typ().with_message("URL constructor not found"))?;
    let ctor_obj = url_ctor
        .as_object()
        .ok_or_else(|| JsNativeError::typ().with_message("URL constructor not callable"))?;
    let instance = ctor_obj
        .construct(&[JsValue::from(js_string!(url_str.as_str()))], None, ctx)
        .map_err(|_| JsNativeError::typ().with_message("Failed to construct URL"))?;

    Ok(JsValue::from(instance))
}

// ── url.parse(urlStr, parseQueryString, slashesDenoteHost) ──────────

fn parse_impl(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let url_str = get_str_arg(args)?;
    let parse_query = args.get(1).and_then(|v| v.as_boolean()).unwrap_or(false);
    let _slashes_denote_host = args.get(2).and_then(|v| v.as_boolean()).unwrap_or(false);

    let result = JsObject::with_object_proto(ctx.intrinsics());

    // Split off hash
    let (before_hash, hash) = url_str.split_once('#').unwrap_or((&url_str, ""));
    let hash = if hash.is_empty() {
        String::new()
    } else {
        format!("#{hash}")
    };
    let _ = result.set(js_string!("hash"), js_string!(hash.as_str()), false, ctx);

    // Split off search/query
    let (before_query, query) = before_hash.split_once('?').unwrap_or((before_hash, ""));
    let search = if query.is_empty() {
        String::new()
    } else {
        format!("?{query}")
    };
    let _ = result.set(js_string!("search"), js_string!(search.as_str()), false, ctx);

    if parse_query && !query.is_empty() {
        let query_obj = JsObject::with_object_proto(ctx.intrinsics());
        for pair in query.split('&') {
            let mut parts = pair.splitn(2, '=');
            let key = parts.next().unwrap_or("");
            let val = parts.next().unwrap_or("");
            let decoded_key = percent_encoding::percent_decode_str(key).decode_utf8_lossy();
            let decoded_val = percent_encoding::percent_decode_str(val).decode_utf8_lossy();
            let _ = query_obj.set(js_string!(decoded_key.as_ref()), js_string!(decoded_val.as_ref()), false, ctx);
        }
        let _ = result.set(js_string!("query"), JsValue::from(query_obj), false, ctx);
    } else if !query.is_empty() {
        let _ = result.set(js_string!("query"), js_string!(query), false, ctx);
    }

    // Parse protocol + host/path
    let (protocol, rest) = if let Some((proto, r)) = before_query.split_once("://") {
        let has_www = r.starts_with("www.") || r.contains('/');
        if has_www || proto_has_slashes(proto) {
            (format!("{proto}:"), r)
        } else {
            (String::new(), before_query)
        }
    } else if let Some((proto, r)) = before_query.split_once(':') {
        if r.starts_with("//") {
            let r = &r[2..];
            (format!("{proto}:"), r)
        } else {
            (String::new(), before_query)
        }
    } else {
        (String::new(), before_query)
    };

    if !protocol.is_empty() {
        let _ = result.set(js_string!("protocol"), js_string!(protocol.as_str()), false, ctx);
        let _ = result.set(js_string!("slashes"), JsValue::from(true), false, ctx);
    }

    // Split rest into host and path
    let (host_part, path_part) = if !protocol.is_empty() {
        // Has protocol — first segment is host
        if let Some((host, path)) = rest.split_once('/') {
            (Some(host), format!("/{path}"))
        } else {
            (Some(rest), String::new())
        }
    } else {
        // No protocol — everything is path
        (None, before_query.to_string())
    };

    // Pathname and path
    let path = if path_part.is_empty() && !search.is_empty() {
        String::new()
    } else if path_part.is_empty() {
        "/".to_string()
    } else {
        path_part.clone()
    };
    let _ = result.set(js_string!("pathname"), js_string!(path_part.as_str()), false, ctx);

    let full_path = if search.is_empty() {
        path.clone()
    } else {
        format!("{path}{search}")
    };
    let _ = result.set(js_string!("path"), js_string!(full_path.as_str()), false, ctx);

    // Host parsing
    if let Some(host_str) = host_part {
        let _ = result.set(js_string!("host"), js_string!(host_str), false, ctx);
        if let Some((hostname, port)) = host_str.rsplit_once(':') {
            if port.parse::<u16>().is_ok() {
                let _ = result.set(js_string!("hostname"), js_string!(hostname), false, ctx);
                let _ = result.set(js_string!("port"), js_string!(port), false, ctx);
            } else {
                let _ = result.set(js_string!("hostname"), js_string!(host_str), false, ctx);
            }
        } else {
            let _ = result.set(js_string!("hostname"), js_string!(host_str), false, ctx);
        }
    }

    // href
    let _ = result.set(js_string!("href"), js_string!(url_str.as_str()), false, ctx);

    Ok(JsValue::from(result))
}

fn proto_has_slashes(_proto: &str) -> bool {
    matches!(_proto, "http" | "https" | "ftp" | "ws" | "wss" | "file")
}

// ── url.format(urlObj) ─────────────────────────────────────────────

fn format_impl(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let obj = match args.first().and_then(|v| v.as_object()) {
        Some(o) => o.clone(),
        None => return Err(JsNativeError::typ().with_message("url.format: first arg must be object").into()),
    };

    let mut get_s = |key: &str| -> String {
        obj.get(js_string!(key), ctx).ok()
            .and_then(|v| v.to_string(ctx).ok())
            .map(|s| s.to_std_string_escaped())
            .unwrap_or_default()
    };

    let protocol = get_s("protocol");
    let host = get_s("host");
    let hostname = get_s("hostname");
    let port = get_s("port");
    let pathname = get_s("pathname");
    let search = get_s("search");
    let hash = get_s("hash");
    let auth = get_s("auth");

    let mut result = String::new();

    if !protocol.is_empty() {
        if !protocol.ends_with(':') {
            result.push_str(&protocol);
            result.push(':');
        } else {
            result.push_str(&protocol);
        }
    }

    if !host.is_empty() {
        result.push_str("//");
        if !auth.is_empty() {
            result.push_str(&auth);
            result.push('@');
        }
        result.push_str(&host);
    } else if !hostname.is_empty() {
        result.push_str("//");
        if !auth.is_empty() {
            result.push_str(&auth);
            result.push('@');
        }
        result.push_str(&hostname);
        if !port.is_empty() {
            result.push(':');
            result.push_str(&port);
        }
    }

    if !pathname.is_empty() {
        if !pathname.starts_with('/') && !result.is_empty() && result.chars().last() != Some('.' as char) {
            // relative path
            result.push('/');
        }
        result.push_str(&pathname);
    }

    if !search.is_empty() {
        if !search.starts_with('?') {
            result.push('?');
        }
        result.push_str(&search);
    }

    if !hash.is_empty() {
        if !hash.starts_with('#') {
            result.push('#');
        }
        result.push_str(&hash);
    }

    Ok(JsValue::from(js_string!(result.as_str())))
}

// ── url.resolve(from, to) ──────────────────────────────────────────

fn resolve_impl(_this: &JsValue, args: &[JsValue], _ctx: &mut Context) -> JsResult<JsValue> {
    let from_str = get_str_arg(args)?;
    let to_str_val = match args.get(1).and_then(|v| v.as_string()) {
        Some(s) => s.to_std_string_escaped(),
        None => return Err(JsNativeError::typ().with_message("url.resolve: second arg must be a string").into()),
    };

    // Simple resolver: use a URL-like resolution
    let result = if from_str.contains("://") {
        // Absolute URL: resolve relative to it
        let from_has_trailing_slash = from_str.ends_with('/');
        let base = if from_has_trailing_slash {
            from_str.clone()
        } else {
            // Remove last path segment
            if let Some(pos) = from_str.rfind('/') {
                from_str[..=pos].to_string()
            } else {
                format!("{from_str}/")
            }
        };
        if to_str_val.starts_with("://") {
            // Protocol relative
            let proto_end = from_str.find("://").unwrap_or(0);
            format!("{}{}", &from_str[..proto_end], to_str_val)
        } else if to_str_val.starts_with('/') {
            // Absolute path — replace path
            if let Some(pos) = base.find("://") {
                if let Some(slash_pos) = base[pos + 3..].find('/') {
                    format!("{}{}", &base[..=pos + 3 + slash_pos], to_str_val)
                } else {
                    format!("{base}{to_str_val}")
                }
            } else {
                format!("{base}{to_str_val}")
            }
        } else if to_str_val.starts_with('#') || to_str_val.starts_with('?') {
            format!("{base}{to_str_val}")
        } else {
            // Relative path
            format!("{base}{to_str_val}")
        }
    } else {
        // Relative from — just return to
        to_str_val
    };

    // Clean up ".." and "."
    let cleaned = resolve_dot_segments(&result);
    Ok(JsValue::from(js_string!(cleaned.as_str())))
}

fn resolve_dot_segments(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    let is_absolute = path.starts_with('/');
    for segment in path.split('/') {
        match segment {
            "." | "" => { /* skip */ }
            ".." => { parts.pop(); }
            _ => { parts.push(segment); }
        }
    }
    if is_absolute {
        format!("/{}", parts.join("/"))
    } else {
        parts.join("/")
    }
}

pub fn create_node_url_module(context: &mut Context) -> Result<Module, String> {
    let export_names: &[JsString] = &[
        js_string!("URL"),
        js_string!("URLSearchParams"),
        js_string!("fileURLToPath"),
        js_string!("pathToFileURL"),
        js_string!("parse"),
        js_string!("format"),
        js_string!("resolve"),
        js_string!("default"),
    ];

    let module = Module::synthetic(
        export_names,
        SyntheticModuleInitializer::from_copy_closure(
            move |m: &boa_engine::module::SyntheticModule, ctx: &mut Context| {
                let url_class = ctx
                    .global_object()
                    .get(js_string!("URL"), ctx)
                    .map_err(|_| JsNativeError::typ().with_message("no URL global"))?;
                let search_params_class = ctx
                    .global_object()
                    .get(js_string!("URLSearchParams"), ctx)
                    .map_err(|_| JsNativeError::typ().with_message("no URLSearchParams global"))?;

                let file_url_to_path = make_fn(file_url_to_path_impl, "fileURLToPath", 1, ctx);
                let path_to_file_url = make_fn(path_to_file_url_impl, "pathToFileURL", 1, ctx);
                let parse_fn = make_fn(parse_impl, "parse", 1, ctx);
                let format_fn = make_fn(format_impl, "format", 1, ctx);
                let resolve_fn = make_fn(resolve_impl, "resolve", 2, ctx);

                m.set_export(&js_string!("URL"), url_class.clone())?;
                m.set_export(&js_string!("URLSearchParams"), search_params_class.clone())?;
                m.set_export(&js_string!("fileURLToPath"), file_url_to_path.clone())?;
                m.set_export(&js_string!("pathToFileURL"), path_to_file_url.clone())?;
                m.set_export(&js_string!("parse"), parse_fn.clone())?;
                m.set_export(&js_string!("format"), format_fn.clone())?;
                m.set_export(&js_string!("resolve"), resolve_fn.clone())?;

                let default_obj = JsObject::with_object_proto(ctx.intrinsics());
                default_obj.set(js_string!("URL"), url_class, false, ctx)?;
                default_obj.set(
                    js_string!("URLSearchParams"),
                    search_params_class,
                    false,
                    ctx,
                )?;
                default_obj.set(js_string!("fileURLToPath"), file_url_to_path, false, ctx)?;
                default_obj.set(js_string!("pathToFileURL"), path_to_file_url, false, ctx)?;
                default_obj.set(js_string!("parse"), parse_fn, false, ctx)?;
                default_obj.set(js_string!("format"), format_fn, false, ctx)?;
                default_obj.set(js_string!("resolve"), resolve_fn, false, ctx)?;
                m.set_export(&js_string!("default"), JsValue::from(default_obj))?;

                Ok(())
            },
        ),
        None,
        None,
        context,
    );

    Ok(module)
}
