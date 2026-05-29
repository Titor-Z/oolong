use boa_engine::module::SyntheticModuleInitializer;
use boa_engine::object::FunctionObjectBuilder;
use boa_engine::{
    Context, IntoJsFunctionCopied, JsError, JsNativeError, JsObject, JsResult, JsString, JsValue,
    Module, js_string, interop::JsRest,
};

fn is_sep(ch: u8, win: bool) -> bool {
    ch == b'/' || (win && ch == b'\\')
}

fn normalize_internal(path: &str, win: bool) -> String {
    let sep = if win { "\\" } else { "/" };
    let s = path.trim_end_matches(|ch| is_sep(ch as u8, win));
    let abs = !s.is_empty() && is_sep(s.as_bytes()[0], win);
    let mut stack: Vec<&str> = Vec::new();
    for seg in s.split(|ch| is_sep(ch as u8, win)) {
        if seg.is_empty() || seg == "." { continue; }
        if seg == ".." {
            if !stack.is_empty() && stack.last().is_some_and(|&x| x != "..") {
                stack.pop();
            } else if !abs {
                stack.push("..");
            }
        } else {
            stack.push(seg);
        }
    }
    if abs { format!("{}{}", sep, stack.join(sep)) }
    else { stack.join(sep) }
}

fn last_sep_idx(s: &str, win: bool) -> Option<usize> {
    if win { s.rfind(['/', '\\']) }
    else { s.rfind('/') }
}

fn basename_internal(s: &str, win: bool) -> &str {
    match last_sep_idx(s, win) {
        None => s,
        Some(i) => &s[i + 1..],
    }
}

fn str_arg(val: &JsValue) -> Result<String, JsError> {
    val.as_string()
        .map(|s| s.to_std_string_escaped())
        .ok_or_else(|| JsError::from(JsNativeError::typ().with_message("Expected a string")))
}

fn make_path_obj(win: bool, ctx: &mut Context) -> JsResult<JsObject> {
    let obj = JsObject::with_object_proto(ctx.intrinsics());
    let sep = if win { "\\" } else { "/" };

    let _ = obj.set(js_string!("sep"), JsValue::from(js_string!(sep)), false, ctx);
    let delim = if win { ";" } else { ":" };
    let _ = obj.set(js_string!("delimiter"), JsValue::from(js_string!(delim)), false, ctx);

    // join(...paths)
    let f = (move |rest: JsRest<'_>, _ctx: &mut Context| -> JsResult<JsValue> {
        let parts: Vec<String> = rest.0.iter().map(str_arg).collect::<Result<_, _>>()?;
        Ok(JsValue::from(js_string!(normalize_internal(&parts.join(sep), win).as_str())))
    }).into_js_function_copied(ctx);
    let _ = obj.set(js_string!("join"), FunctionObjectBuilder::new(ctx.realm(), f).name("join").length(1).build(), false, ctx);

    // dirname(path)
    let f = (move |path: JsValue, _ctx: &mut Context| -> JsResult<JsValue> {
        let p = str_arg(&path)?;
        let n = normalize_internal(&p, win);
        let result = if n == sep { n }
        else { match last_sep_idx(&n, win) {
            None => ".".to_string(),
        Some(0) => sep.to_string(),
        Some(i) => n[..i].to_string(),
        }};
        Ok(JsValue::from(js_string!(result.as_str())))
    }).into_js_function_copied(ctx);
    let _ = obj.set(js_string!("dirname"), FunctionObjectBuilder::new(ctx.realm(), f).name("dirname").length(1).build(), false, ctx);

    // basename(path, ext?)
    let f = (move |path: JsValue, ext: Option<JsValue>, _ctx: &mut Context| -> JsResult<JsValue> {
        let p = str_arg(&path)?;
        let n = normalize_internal(&p, win);
        let base = basename_internal(&n, win).to_string();
        let base = if let Some(ext_val) = ext {
            if let Some(ext_s) = ext_val.as_string() {
                let ext_str = ext_s.to_std_string_escaped();
                if base.ends_with(&ext_str) {
                    base[..base.len() - ext_str.len()].to_string()
                } else { base }
            } else { base }
        } else { base };
        Ok(JsValue::from(js_string!(base.as_str())))
    }).into_js_function_copied(ctx);
    let _ = obj.set(js_string!("basename"), FunctionObjectBuilder::new(ctx.realm(), f).name("basename").length(1).build(), false, ctx);

    // extname(path)
    let f = (move |path: JsValue, _ctx: &mut Context| -> JsResult<JsValue> {
        let p = str_arg(&path)?;
        let n = normalize_internal(&p, win);
        let base = basename_internal(&n, win);
        let ext = base.rfind('.').filter(|&i| i > 0).map(|i| base[i..].to_string()).unwrap_or_default();
        Ok(JsValue::from(js_string!(ext.as_str())))
    }).into_js_function_copied(ctx);
    let _ = obj.set(js_string!("extname"), FunctionObjectBuilder::new(ctx.realm(), f).name("extname").length(1).build(), false, ctx);

    // isAbsolute(path)
    let f = (move |path: JsValue, _ctx: &mut Context| -> JsResult<JsValue> {
        let p = str_arg(&path)?;
        Ok(JsValue::from(!p.is_empty() && is_sep(p.as_bytes()[0], win)))
    }).into_js_function_copied(ctx);
    let _ = obj.set(js_string!("isAbsolute"), FunctionObjectBuilder::new(ctx.realm(), f).name("isAbsolute").length(1).build(), false, ctx);

    // normalize(path)
    let f = (move |path: JsValue, _ctx: &mut Context| -> JsResult<JsValue> {
        let p = str_arg(&path)?;
        Ok(JsValue::from(js_string!(normalize_internal(&p, win).as_str())))
    }).into_js_function_copied(ctx);
    let _ = obj.set(js_string!("normalize"), FunctionObjectBuilder::new(ctx.realm(), f).name("normalize").length(1).build(), false, ctx);

    // relative(from, to)
    let f = (move |from: JsValue, to: JsValue, _ctx: &mut Context| -> JsResult<JsValue> {
        let f = str_arg(&from)?;
        let t = str_arg(&to)?;
        let fnorm = normalize_internal(&f, win);
        let tnorm = normalize_internal(&t, win);
        let fparts: Vec<&str> = fnorm.split(|ch| is_sep(ch as u8, win)).filter(|s| !s.is_empty()).collect();
        let tparts: Vec<&str> = tnorm.split(|ch| is_sep(ch as u8, win)).filter(|s| !s.is_empty()).collect();
        let mut i = 0;
        while i < fparts.len() && i < tparts.len() && fparts[i] == tparts[i] { i += 1; }
        let up: Vec<&str> = fparts[i..].iter().map(|_| "..").collect();
        let down: Vec<&str> = tparts[i..].to_vec();
        let r = [up.as_slice(), down.as_slice()].concat().join(sep);
        let result = if r.is_empty() { "." } else { &r };
        Ok(JsValue::from(js_string!(result)))
    }).into_js_function_copied(ctx);
    let _ = obj.set(js_string!("relative"), FunctionObjectBuilder::new(ctx.realm(), f).name("relative").length(2).build(), false, ctx);

    // resolve(...paths)
    let f = (move |rest: JsRest<'_>, _ctx: &mut Context| -> JsResult<JsValue> {
        let cwd = std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| "/".to_string());
        let parts: Vec<String> = rest.0.iter().map(str_arg).collect::<Result<_, _>>()?;
        let mut abs = false;
        let mut start = 0;
        for (i, p) in parts.iter().enumerate() {
            if !p.is_empty() && is_sep(p.as_bytes()[0], win) { abs = true; start = i; break; }
        }
        let base = if abs { String::new() } else { format!("{}{}", cwd, sep) };
        let joined = base + &parts[start..].join(sep);
        Ok(JsValue::from(js_string!(normalize_internal(&joined, win).as_str())))
    }).into_js_function_copied(ctx);
    let _ = obj.set(js_string!("resolve"), FunctionObjectBuilder::new(ctx.realm(), f).name("resolve").length(1).build(), false, ctx);

    // parse(path)
    let f = (move |path: JsValue, ctx: &mut Context| -> JsResult<JsValue> {
        let p = str_arg(&path)?;
        let n = normalize_internal(&p, win);
        let result = JsObject::with_object_proto(ctx.intrinsics());
        if n.is_empty() || n == "." {
            let _ = result.set(js_string!("root"), JsValue::from(js_string!("")), false, ctx);
            let _ = result.set(js_string!("dir"), JsValue::from(js_string!(".")), false, ctx);
            let _ = result.set(js_string!("base"), JsValue::from(js_string!(n.as_str())), false, ctx);
            let _ = result.set(js_string!("ext"), JsValue::from(js_string!("")), false, ctx);
            let _ = result.set(js_string!("name"), JsValue::from(js_string!(n.as_str())), false, ctx);
        } else {
            let abs = !n.is_empty() && is_sep(n.as_bytes()[0], win);
            let root = if abs { sep } else { "" };
            let base = basename_internal(&n, win).to_string();
            let ext_i = base.rfind('.');
            let ext = ext_i.filter(|&i| i > 0).map(|i| base[i..].to_string()).unwrap_or_default();
            let name = if !ext.is_empty() { base[..ext_i.unwrap()].to_string() } else { base.clone() };
            let dir = match last_sep_idx(&n, win) {
                None => ".".to_string(),
                Some(0) => sep.to_string(),
                Some(i) => n[..i].to_string(),
            };
            let _ = result.set(js_string!("root"), JsValue::from(js_string!(root)), false, ctx);
            let _ = result.set(js_string!("dir"), JsValue::from(js_string!(dir.as_str())), false, ctx);
            let _ = result.set(js_string!("base"), JsValue::from(js_string!(base.as_str())), false, ctx);
            let _ = result.set(js_string!("ext"), JsValue::from(js_string!(ext.as_str())), false, ctx);
            let _ = result.set(js_string!("name"), JsValue::from(js_string!(name.as_str())), false, ctx);
        }
        Ok(result.into())
    }).into_js_function_copied(ctx);
    let _ = obj.set(js_string!("parse"), FunctionObjectBuilder::new(ctx.realm(), f).name("parse").length(1).build(), false, ctx);

    // format(obj)
    let f = (move |obj_val: JsValue, ctx: &mut Context| -> JsResult<JsValue> {
        let obj = obj_val.as_object().ok_or_else(|| {
            JsError::from(JsNativeError::typ().with_message("path.format: argument must be an object"))
        })?;
        let dir = obj.get(js_string!("dir"), ctx).ok()
            .and_then(|v| v.as_string().map(|s| s.to_std_string_escaped()))
            .unwrap_or_default();
        let base = if let Some(base_v) = obj.get(js_string!("base"), ctx).ok()
            .and_then(|v| v.as_string().map(|s| s.to_std_string_escaped()))
        { base_v } else {
            let name = obj.get(js_string!("name"), ctx).ok()
                .and_then(|v| v.as_string().map(|s| s.to_std_string_escaped()))
                .unwrap_or_default();
            let ext = obj.get(js_string!("ext"), ctx).ok()
                .and_then(|v| v.as_string().map(|s| s.to_std_string_escaped()))
                .unwrap_or_default();
            name + &ext
        };
        let result = if dir.is_empty() { base }
        else if dir.ends_with(sep) { format!("{}{}", dir, base) }
        else { format!("{}{}{}", dir, sep, base) };
        Ok(JsValue::from(js_string!(result.as_str())))
    }).into_js_function_copied(ctx);
    let _ = obj.set(js_string!("format"), FunctionObjectBuilder::new(ctx.realm(), f).name("format").length(1).build(), false, ctx);

    // toNamespacedPath(path)
    let f = (move |path: JsValue, _ctx: &mut Context| -> JsResult<JsValue> {
        let p = str_arg(&path)?;
        Ok(JsValue::from(js_string!(p.as_str())))
    }).into_js_function_copied(ctx);
    let _ = obj.set(js_string!("toNamespacedPath"), FunctionObjectBuilder::new(ctx.realm(), f).name("toNamespacedPath").length(1).build(), false, ctx);

    Ok(obj)
}

pub fn create_node_path_module(context: &mut Context) -> Result<Module, String> {
    let export_names: &[JsString] = &[
        js_string!("sep"), js_string!("delimiter"),
        js_string!("join"), js_string!("dirname"), js_string!("basename"),
        js_string!("extname"), js_string!("isAbsolute"), js_string!("normalize"),
        js_string!("relative"), js_string!("resolve"), js_string!("parse"),
        js_string!("format"), js_string!("toNamespacedPath"),
        js_string!("posix"), js_string!("win32"), js_string!("default"),
    ];

    let module = Module::synthetic(
        export_names,
        SyntheticModuleInitializer::from_copy_closure(
            |m: &boa_engine::module::SyntheticModule, ctx: &mut Context| {
                let path_obj = make_path_obj(cfg!(windows), ctx)?;

                let sep_v = path_obj.get(js_string!("sep"), ctx).map_err(|_| JsError::from(JsNativeError::typ().with_message("no sep")))?;
                let delim_v = JsValue::from(js_string!(if cfg!(windows) { ";" } else { ":" }));
                let join_v = path_obj.get(js_string!("join"), ctx).map_err(|_| JsError::from(JsNativeError::typ().with_message("no join")))?;
                let dirname_v = path_obj.get(js_string!("dirname"), ctx).map_err(|_| JsError::from(JsNativeError::typ().with_message("no dirname")))?;
                let basename_v = path_obj.get(js_string!("basename"), ctx).map_err(|_| JsError::from(JsNativeError::typ().with_message("no basename")))?;
                let extname_v = path_obj.get(js_string!("extname"), ctx).map_err(|_| JsError::from(JsNativeError::typ().with_message("no extname")))?;
                let is_abs_v = path_obj.get(js_string!("isAbsolute"), ctx).map_err(|_| JsError::from(JsNativeError::typ().with_message("no isAbsolute")))?;
                let normalize_v = path_obj.get(js_string!("normalize"), ctx).map_err(|_| JsError::from(JsNativeError::typ().with_message("no normalize")))?;
                let relative_v = path_obj.get(js_string!("relative"), ctx).map_err(|_| JsError::from(JsNativeError::typ().with_message("no relative")))?;
                let resolve_v = path_obj.get(js_string!("resolve"), ctx).map_err(|_| JsError::from(JsNativeError::typ().with_message("no resolve")))?;
                let parse_v = path_obj.get(js_string!("parse"), ctx).map_err(|_| JsError::from(JsNativeError::typ().with_message("no parse")))?;
                let format_v = path_obj.get(js_string!("format"), ctx).map_err(|_| JsError::from(JsNativeError::typ().with_message("no format")))?;
                let to_ns_v = path_obj.get(js_string!("toNamespacedPath"), ctx).map_err(|_| JsError::from(JsNativeError::typ().with_message("no toNamespacedPath")))?;

                m.set_export(&js_string!("sep"), sep_v.clone())?;
                m.set_export(&js_string!("delimiter"), delim_v)?;
                m.set_export(&js_string!("join"), join_v.clone())?;
                m.set_export(&js_string!("dirname"), dirname_v.clone())?;
                m.set_export(&js_string!("basename"), basename_v.clone())?;
                m.set_export(&js_string!("extname"), extname_v.clone())?;
                m.set_export(&js_string!("isAbsolute"), is_abs_v.clone())?;
                m.set_export(&js_string!("normalize"), normalize_v.clone())?;
                m.set_export(&js_string!("relative"), relative_v.clone())?;
                m.set_export(&js_string!("resolve"), resolve_v.clone())?;
                m.set_export(&js_string!("parse"), parse_v.clone())?;
                m.set_export(&js_string!("format"), format_v.clone())?;
                m.set_export(&js_string!("toNamespacedPath"), to_ns_v.clone())?;

                m.set_export(&js_string!("posix"), make_path_obj(false, ctx)?.into())?;
                m.set_export(&js_string!("win32"), make_path_obj(true, ctx)?.into())?;
                m.set_export(&js_string!("default"), path_obj.into())?;

                Ok(())
            },
        ),
        None,
        None,
        context,
    );

    Ok(module)
}
