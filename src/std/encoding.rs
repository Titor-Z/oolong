use base64::Engine;
use boa_engine::module::SyntheticModuleInitializer;
use boa_engine::object::builtins::{JsArrayBuffer, JsUint8Array};
use boa_engine::{
    Context, IntoJsFunctionCopied, JsNativeError, JsObject, JsResult, JsString, JsValue, Module,
    js_string, object::FunctionObjectBuilder,
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

fn extract_bytes(value: &JsValue, ctx: &mut Context) -> Option<Vec<u8>> {
    if let Some(s) = value.as_string() {
        return Some(s.to_std_string_escaped().into_bytes());
    }
    let obj = value.as_object()?;
    if let Ok(buf) = JsArrayBuffer::from_object(obj.clone())
        && let Some(data) = buf.data()
    {
        return Some(data.to_vec());
    }
    if let Ok(buf_val) = obj.get(js_string!("buffer"), ctx)
        && let Some(buf_obj) = buf_val.as_object()
        && let Ok(buf) = JsArrayBuffer::from_object(buf_obj.clone())
        && let Some(data) = buf.data()
    {
        return Some(data.to_vec());
    }
    None
}

fn make_uint8array(bytes: Vec<u8>, ctx: &mut Context) -> JsValue {
    JsUint8Array::from_iter(bytes, ctx)
        .map(Into::into)
        .unwrap_or(JsValue::undefined())
}

fn hex_decode(s: &str) -> Result<Vec<u8>, String> {
    let s = s.trim();
    if !s.len().is_multiple_of(2) {
        return Err("hex 字符串长度必须为偶数".into());
    }
    (0..s.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&s[i..i + 2], 16)
                .map_err(|_| format!("无效十六进制字符: {}", &s[i..i + 2]))
        })
        .collect()
}

pub fn create_encoding_module(context: &mut Context) -> Result<Module, String> {
    let export_names: &[JsString] = &[
        js_string!("base64"),
        js_string!("hex"),
        js_string!("default"),
    ];

    let module = Module::synthetic(
        export_names,
        SyntheticModuleInitializer::from_copy_closure(
            |m: &boa_engine::module::SyntheticModule, ctx: &mut Context| {
                // ── base64 ────────────────────────────────────────────────────
                let b64_encode_fn = make_fn(
                    (|data: JsValue, ctx: &mut Context| -> JsResult<JsValue> {
                        let bytes = extract_bytes(&data, ctx).ok_or_else(|| {
                            JsNativeError::typ()
                                .with_message("base64.encode: 参数必须是字符串或 Uint8Array")
                        })?;
                        let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
                        Ok(JsValue::from(js_string!(encoded)))
                    })
                    .into_js_function_copied(ctx),
                    "encode",
                    1,
                    ctx,
                );

                let b64_decode_fn = make_fn(
                    (|str: JsString, ctx: &mut Context| -> JsResult<JsValue> {
                        let s = str.to_std_string_escaped();
                        let bytes = base64::engine::general_purpose::STANDARD
                            .decode(s.as_bytes())
                            .map_err(|e| {
                                JsNativeError::typ().with_message(format!("base64.decode: {e}"))
                            })?;
                        Ok(make_uint8array(bytes, ctx))
                    })
                    .into_js_function_copied(ctx),
                    "decode",
                    1,
                    ctx,
                );

                let base64_ns = JsObject::with_object_proto(ctx.intrinsics());
                let _ = base64_ns.set(js_string!("encode"), b64_encode_fn, false, ctx);
                let _ = base64_ns.set(js_string!("decode"), b64_decode_fn, false, ctx);
                m.set_export(&js_string!("base64"), base64_ns.clone().into())?;

                // ── hex ───────────────────────────────────────────────────────
                let hex_encode_fn = make_fn(
                    (|data: JsValue, ctx: &mut Context| -> JsResult<JsValue> {
                        let bytes = extract_bytes(&data, ctx).ok_or_else(|| {
                            JsNativeError::typ()
                                .with_message("hex.encode: 参数必须是字符串或 Uint8Array")
                        })?;
                        let encoded: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
                        Ok(JsValue::from(js_string!(encoded)))
                    })
                    .into_js_function_copied(ctx),
                    "encode",
                    1,
                    ctx,
                );

                let hex_decode_fn = make_fn(
                    (|str: JsString, ctx: &mut Context| -> JsResult<JsValue> {
                        let s = str.to_std_string_escaped();
                        let bytes = hex_decode(&s).map_err(|e| {
                            JsNativeError::typ().with_message(format!("hex.decode: {e}"))
                        })?;
                        Ok(make_uint8array(bytes, ctx))
                    })
                    .into_js_function_copied(ctx),
                    "decode",
                    1,
                    ctx,
                );

                let hex_ns = JsObject::with_object_proto(ctx.intrinsics());
                let _ = hex_ns.set(js_string!("encode"), hex_encode_fn, false, ctx);
                let _ = hex_ns.set(js_string!("decode"), hex_decode_fn, false, ctx);
                m.set_export(&js_string!("hex"), hex_ns.clone().into())?;

                // ── default ───────────────────────────────────────────────────
                let default_obj = JsObject::with_object_proto(ctx.intrinsics());
                let _ = default_obj.set(js_string!("base64"), base64_ns, false, ctx);
                let _ = default_obj.set(js_string!("hex"), hex_ns, false, ctx);
                m.set_export(&js_string!("default"), default_obj.into())?;

                Ok(())
            },
        ),
        None,
        None,
        context,
    );

    Ok(module)
}
