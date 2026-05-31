use boa_engine::module::SyntheticModuleInitializer;
use boa_engine::object::FunctionObjectBuilder;
use boa_engine::{
    Context, JsObject, JsResult, JsString, JsValue, Module, NativeFunction,
    js_string,
};

fn make_fn(
    native: boa_engine::NativeFunction,
    name: &str,
    len: usize,
    ctx: &mut Context,
) -> JsValue {
    FunctionObjectBuilder::new(ctx.realm(), native)
        .name(JsString::from(name))
        .length(len)
        .build()
        .into()
}

fn extract_text(args: &[JsValue]) -> String {
    args.first()
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default()
}

fn red_impl(_: &JsValue, args: &[JsValue], _: &mut Context) -> JsResult<JsValue> {
    let t = extract_text(args);
    Ok(JsValue::from(js_string!(format!("\x1b[31m{}\x1b[0m", t))))
}
fn green_impl(_: &JsValue, args: &[JsValue], _: &mut Context) -> JsResult<JsValue> {
    let t = extract_text(args);
    Ok(JsValue::from(js_string!(format!("\x1b[32m{}\x1b[0m", t))))
}
fn yellow_impl(_: &JsValue, args: &[JsValue], _: &mut Context) -> JsResult<JsValue> {
    let t = extract_text(args);
    Ok(JsValue::from(js_string!(format!("\x1b[33m{}\x1b[0m", t))))
}
fn blue_impl(_: &JsValue, args: &[JsValue], _: &mut Context) -> JsResult<JsValue> {
    let t = extract_text(args);
    Ok(JsValue::from(js_string!(format!("\x1b[34m{}\x1b[0m", t))))
}
fn magenta_impl(_: &JsValue, args: &[JsValue], _: &mut Context) -> JsResult<JsValue> {
    let t = extract_text(args);
    Ok(JsValue::from(js_string!(format!("\x1b[35m{}\x1b[0m", t))))
}
fn cyan_impl(_: &JsValue, args: &[JsValue], _: &mut Context) -> JsResult<JsValue> {
    let t = extract_text(args);
    Ok(JsValue::from(js_string!(format!("\x1b[36m{}\x1b[0m", t))))
}
fn white_impl(_: &JsValue, args: &[JsValue], _: &mut Context) -> JsResult<JsValue> {
    let t = extract_text(args);
    Ok(JsValue::from(js_string!(format!("\x1b[37m{}\x1b[0m", t))))
}
fn gray_impl(_: &JsValue, args: &[JsValue], _: &mut Context) -> JsResult<JsValue> {
    let t = extract_text(args);
    Ok(JsValue::from(js_string!(format!("\x1b[90m{}\x1b[0m", t))))
}
fn bold_impl(_: &JsValue, args: &[JsValue], _: &mut Context) -> JsResult<JsValue> {
    let t = extract_text(args);
    Ok(JsValue::from(js_string!(format!("\x1b[1m{}\x1b[22m", t))))
}
fn dim_impl(_: &JsValue, args: &[JsValue], _: &mut Context) -> JsResult<JsValue> {
    let t = extract_text(args);
    Ok(JsValue::from(js_string!(format!("\x1b[2m{}\x1b[22m", t))))
}
fn italic_impl(_: &JsValue, args: &[JsValue], _: &mut Context) -> JsResult<JsValue> {
    let t = extract_text(args);
    Ok(JsValue::from(js_string!(format!("\x1b[3m{}\x1b[23m", t))))
}
fn underline_impl(_: &JsValue, args: &[JsValue], _: &mut Context) -> JsResult<JsValue> {
    let t = extract_text(args);
    Ok(JsValue::from(js_string!(format!("\x1b[4m{}\x1b[24m", t))))
}
fn strip_color_impl(_: &JsValue, args: &[JsValue], _: &mut Context) -> JsResult<JsValue> {
    let t = extract_text(args);
    Ok(JsValue::from(js_string!(strip_ansi(&t))))
}

fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' && chars.next() == Some('[') {
            while let Some(n) = chars.next() {
                if n == 'm' {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn js_value_to_string(v: &JsValue, ctx: &mut Context) -> String {
    if let Some(s) = v.as_string() {
        s.to_std_string_escaped()
    } else if v.is_null() {
        "null".to_string()
    } else if v.is_undefined() {
        "undefined".to_string()
    } else if let Some(n) = v.as_number() {
        if n.fract() == 0.0 && n.is_finite() {
            format!("{}", n as i64)
        } else {
            format!("{}", n)
        }
    } else if v.is_boolean() {
        (if v.as_boolean().unwrap_or(false) {
            "true"
        } else {
            "false"
        })
        .to_string()
    } else if let Ok(s) = v.to_string(ctx) {
        s.to_std_string_escaped()
    } else {
        String::new()
    }
}

fn sprintf_impl(_: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    if args.is_empty() {
        return Ok(JsValue::from(js_string!("")));
    }
    let fmt = args[0]
        .as_string()
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();
    let mut result = String::new();
    let mut chars = fmt.chars();
    let mut arg_idx = 1;

    while let Some(c) = chars.next() {
        if c == '%' {
            match chars.next() {
                Some('s') => {
                    let arg = args
                        .get(arg_idx)
                        .map(|v| js_value_to_string(v, ctx))
                        .unwrap_or_default();
                    result.push_str(&arg);
                    arg_idx += 1;
                }
                Some('d') | Some('i') => {
                    let v = args.get(arg_idx);
                    let s = if let Some(n) = v.and_then(|v| v.as_number()) {
                        format!("{}", n as i64)
                    } else {
                        v.map(|v| js_value_to_string(v, ctx)).unwrap_or_default()
                    };
                    result.push_str(&s);
                    arg_idx += 1;
                }
                Some('f') => {
                    let v = args.get(arg_idx);
                    let s = if let Some(n) = v.and_then(|v| v.as_number()) {
                        format!("{}", n)
                    } else {
                        v.map(|v| js_value_to_string(v, ctx)).unwrap_or_default()
                    };
                    result.push_str(&s);
                    arg_idx += 1;
                }
                Some('j') => {
                    let s = args
                        .get(arg_idx)
                        .map(|v| js_value_to_string(v, ctx))
                        .unwrap_or_default();
                    result.push_str(&s);
                    arg_idx += 1;
                }
                Some('%') => result.push('%'),
                Some(c) => {
                    result.push('%');
                    result.push(c);
                }
                None => result.push('%'),
            }
        } else {
            result.push(c);
        }
    }

    Ok(JsValue::from(js_string!(result)))
}

fn build_colors_object(ctx: &mut Context) -> JsValue {
    let obj = JsObject::with_object_proto(ctx.intrinsics());

    let pairs: &[(&str, boa_engine::NativeFunction, usize)] = &[
        ("red", NativeFunction::from_fn_ptr(red_impl), 1),
        ("green", NativeFunction::from_fn_ptr(green_impl), 1),
        ("yellow", NativeFunction::from_fn_ptr(yellow_impl), 1),
        ("blue", NativeFunction::from_fn_ptr(blue_impl), 1),
        ("magenta", NativeFunction::from_fn_ptr(magenta_impl), 1),
        ("cyan", NativeFunction::from_fn_ptr(cyan_impl), 1),
        ("white", NativeFunction::from_fn_ptr(white_impl), 1),
        ("gray", NativeFunction::from_fn_ptr(gray_impl), 1),
        ("bold", NativeFunction::from_fn_ptr(bold_impl), 1),
        ("dim", NativeFunction::from_fn_ptr(dim_impl), 1),
        ("italic", NativeFunction::from_fn_ptr(italic_impl), 1),
        ("underline", NativeFunction::from_fn_ptr(underline_impl), 1),
        (
            "stripColor",
            NativeFunction::from_fn_ptr(strip_color_impl),
            1,
        ),
    ];

    for &(name, ref func, len) in pairs {
        let v = make_fn(func.clone(), name, len, ctx);
        obj.set(js_string!(name), v, false, ctx).ok();
    }

    obj.into()
}

pub fn create_fmt_module(context: &mut Context) -> Result<Module, String> {
    let export_names: &[JsString] = &[
        js_string!("colors"),
        js_string!("sprintf"),
        js_string!("default"),
    ];

    let module = Module::synthetic(
        export_names,
        SyntheticModuleInitializer::from_copy_closure(
            |m: &boa_engine::module::SyntheticModule, ctx: &mut Context| {
                let colors_obj = build_colors_object(ctx);
                let sprintf_fn =
                    make_fn(NativeFunction::from_fn_ptr(sprintf_impl), "sprintf", 1, ctx);

                m.set_export(&js_string!("colors"), colors_obj.clone())?;
                m.set_export(&js_string!("sprintf"), sprintf_fn.clone())?;

                let default_obj = JsObject::with_object_proto(ctx.intrinsics());
                default_obj
                    .set(js_string!("colors"), colors_obj, false, ctx)
                    .ok();
                default_obj
                    .set(js_string!("sprintf"), sprintf_fn, false, ctx)
                    .ok();
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
