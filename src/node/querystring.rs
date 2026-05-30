use std::collections::HashMap;

use boa_engine::module::SyntheticModuleInitializer;
use boa_engine::object::builtins::JsArray;
use boa_engine::object::FunctionObjectBuilder;
use boa_engine::{
    Context, JsNativeError, JsObject, JsResult, JsString, JsValue, Module, NativeFunction, js_string,
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

fn percent_encode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            b' ' => result.push('+'),
            _ => {
                result.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    result
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut result = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hi = (bytes[i + 1] as char).to_digit(16);
            let lo = (bytes[i + 2] as char).to_digit(16);
            if let (Some(h), Some(l)) = (hi, lo) {
                result.push((h as u8 * 16 + l as u8) as u8);
                i += 3;
                continue;
            }
        }
        result.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(result).unwrap_or_default()
}

fn escape_impl(_this: &JsValue, args: &[JsValue], _ctx: &mut Context) -> JsResult<JsValue> {
    let s = args
        .first()
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();
    Ok(JsValue::from(js_string!(percent_encode(&s).as_str())))
}

fn unescape_impl(_this: &JsValue, args: &[JsValue], _ctx: &mut Context) -> JsResult<JsValue> {
    let s = args
        .first()
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();
    let decoded = s.replace('+', " ");
    let result = percent_decode(&decoded);
    Ok(JsValue::from(js_string!(result.as_str())))
}

fn parse_impl(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let qs = args
        .first()
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();

    let sep_val = args.get(1).and_then(|v| v.as_string());
    let sep = sep_val
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_else(|| "&".to_string());

    let eq_val = args.get(2).and_then(|v| v.as_string());
    let eq = eq_val
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_else(|| "=".to_string());

    let max_keys = args
        .get(3)
        .and_then(|v| v.as_object())
        .and_then(|o| o.get(js_string!("maxKeys"), ctx).ok())
        .and_then(|v| v.as_number())
        .map(|n| n as usize)
        .unwrap_or(1000);

    let obj = JsObject::with_null_proto();

    if qs.is_empty() {
        return Ok(JsValue::from(obj));
    }

    let parts: Vec<&str> = qs.split(&sep).collect();
    let limit = parts.len().min(max_keys);

    let mut key_counts: HashMap<String, usize> = HashMap::new();

    for i in 0..limit {
        let part = parts[i];
        if part.is_empty() {
            continue;
        }
        let (key, val) = if let Some(idx) = part.find(&eq) {
            let k = percent_decode(&part[..idx].replace('+', " "));
            let v = percent_decode(&part[idx + eq.len()..].replace('+', " "));
            (k, v)
        } else {
            let k = percent_decode(&part.replace('+', " "));
            (k, String::new())
        };

        let entry = key_counts.entry(key.clone()).or_insert(0);
        *entry += 1;

        let js_key = js_string!(key.as_str());
        let js_val = JsValue::from(js_string!(val.as_str()));

        if *entry == 1 {
            obj.set(js_key.clone(), js_val, false, ctx)
                .map_err(|_| JsNativeError::typ().with_message("set failed"))?;
        } else if *entry == 2 {
            let prev = obj
                .get(js_key.clone(), ctx)
                .map_err(|_| JsNativeError::typ().with_message("get failed"))?;
            let arr = JsArray::new(ctx);
            arr.push(prev, ctx)
                .map_err(|_| JsNativeError::typ().with_message("push failed"))?;
            arr.push(js_val, ctx)
                .map_err(|_| JsNativeError::typ().with_message("push failed"))?;
            obj.set(js_key, JsValue::from(arr), false, ctx)
                .map_err(|_| JsNativeError::typ().with_message("set failed"))?;
        } else {
            if let Some(arr_obj) = obj.get(js_key, ctx).ok().and_then(|v| v.as_object()) {
                if let Ok(arr) = JsArray::from_object(arr_obj.clone()) {
                    arr.push(js_val, ctx)
                        .map_err(|_| JsNativeError::typ().with_message("push failed"))?;
                }
            }
        }
    }

    Ok(JsValue::from(obj))
}

fn stringify_impl(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let sep = args
        .get(1)
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_else(|| "&".to_string());

    let eq = args
        .get(2)
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_else(|| "=".to_string());

    let obj_val = args.first().cloned().unwrap_or(JsValue::undefined());
    if obj_val.is_null_or_undefined() {
        return Ok(JsValue::from(js_string!("")));
    }

    let obj = match obj_val.as_object() {
        Some(o) => o.clone(),
        None => return Ok(JsValue::from(js_string!(""))),
    };

    let keys = obj
        .own_property_keys(ctx)
        .map_err(|_| JsNativeError::typ().with_message("keys failed"))?;

    let mut result_parts: Vec<String> = Vec::new();

    for key in keys {
        let key_str = match &key {
            boa_engine::property::PropertyKey::String(s) => s.to_std_string_escaped(),
            _ => continue,
        };
        let escaped_key = percent_encode(&key_str);

        let val = obj
            .get(key, ctx)
            .map_err(|_| JsNativeError::typ().with_message("get failed"))?;

        if let Some(arr_obj) = val.as_object() {
            if let Ok(arr) = JsArray::from_object(arr_obj.clone()) {
                let len = arr.length(ctx).unwrap_or(0);
                for i in 0..len {
                    let item = arr.get(i, ctx).unwrap_or_default();
                    let item_str = stringify_val(&item, ctx);
                    let escaped_val = percent_encode(&item_str);
                    result_parts.push(format!("{}{}{}", escaped_key, eq, escaped_val));
                }
                continue;
            }
        }

        let val_str = stringify_val(&val, ctx);
        let escaped_val = percent_encode(&val_str);
        result_parts.push(format!("{}{}{}", escaped_key, eq, escaped_val));
    }

    Ok(JsValue::from(js_string!(result_parts.join(&sep).as_str())))
}

fn stringify_val(v: &JsValue, ctx: &mut Context) -> String {
    if v.is_null_or_undefined() {
        return String::new();
    }
    if let Some(s) = v.as_string() {
        return s.to_std_string_escaped();
    }
    v.to_string(ctx)
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default()
}

pub fn create_node_querystring_module(context: &mut Context) -> Result<Module, String> {
    let export_names: &[JsString] = &[
        js_string!("parse"),
        js_string!("stringify"),
        js_string!("escape"),
        js_string!("unescape"),
        js_string!("decode"),
        js_string!("encode"),
        js_string!("default"),
    ];

    let module = Module::synthetic(
        export_names,
        SyntheticModuleInitializer::from_copy_closure(
            move |m: &boa_engine::module::SyntheticModule, ctx: &mut Context| {
                let parse_fn = make_fn(parse_impl, "parse", 3, ctx);
                let stringify_fn = make_fn(stringify_impl, "stringify", 3, ctx);
                let escape_fn = make_fn(escape_impl, "escape", 1, ctx);
                let unescape_fn = make_fn(unescape_impl, "unescape", 1, ctx);

                m.set_export(&js_string!("parse"), parse_fn.clone())?;
                m.set_export(&js_string!("stringify"), stringify_fn.clone())?;
                m.set_export(&js_string!("escape"), escape_fn.clone())?;
                m.set_export(&js_string!("unescape"), unescape_fn.clone())?;
                m.set_export(&js_string!("decode"), parse_fn.clone())?;
                m.set_export(&js_string!("encode"), stringify_fn.clone())?;

                let default_obj = JsObject::with_object_proto(ctx.intrinsics());
                default_obj.set(js_string!("parse"), parse_fn.clone(), false, ctx)?;
                default_obj.set(js_string!("stringify"), stringify_fn.clone(), false, ctx)?;
                default_obj.set(js_string!("escape"), escape_fn, false, ctx)?;
                default_obj.set(js_string!("unescape"), unescape_fn, false, ctx)?;
                default_obj.set(js_string!("decode"), parse_fn.clone(), false, ctx)?;
                default_obj.set(js_string!("encode"), stringify_fn.clone(), false, ctx)?;
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
