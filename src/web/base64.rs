use boa_engine::{
    Context, JsNativeError, JsResult, JsString, JsValue, NativeFunction, js_string,
    object::FunctionObjectBuilder, property::Attribute,
};

fn atob_impl(_: &JsValue, args: &[JsValue], _ctx: &mut Context) -> JsResult<JsValue> {
    let input = args
        .first()
        .ok_or_else(|| JsNativeError::typ().with_message("atob requires 1 argument"))?
        .to_string(_ctx)?;
    let s = input.to_std_string_escaped();
    let s = s.trim();
    use base64::Engine;
    let engine = base64::engine::general_purpose::STANDARD;
    let bytes = engine
        .decode(s)
        .map_err(|e| JsNativeError::typ().with_message(format!("atob: {e}")))?;
    let result = String::from_utf8(bytes)
        .map_err(|_| JsNativeError::typ().with_message("atob: invalid UTF-8 after decode"))?;
    Ok(JsValue::from(JsString::from(result)))
}

fn btoa_impl(_: &JsValue, args: &[JsValue], _ctx: &mut Context) -> JsResult<JsValue> {
    let input = args
        .first()
        .ok_or_else(|| JsNativeError::typ().with_message("btoa requires 1 argument"))?
        .to_string(_ctx)?;
    let s = input.to_std_string_escaped();
    if !s.is_ascii() {
        return Err(JsNativeError::typ()
            .with_message("btoa: string contains non-Latin1 characters")
            .into());
    }
    use base64::Engine;
    let engine = base64::engine::general_purpose::STANDARD;
    let encoded = engine.encode(s.as_bytes());
    Ok(JsValue::from(JsString::from(encoded)))
}

pub fn register_globals(context: &mut Context) -> JsResult<()> {
    let atob_fn =
        FunctionObjectBuilder::new(context.realm(), NativeFunction::from_fn_ptr(atob_impl))
            .name(js_string!("atob"))
            .length(1)
            .build();
    context.register_global_property(js_string!("atob"), atob_fn, Attribute::all())?;

    let btoa_fn =
        FunctionObjectBuilder::new(context.realm(), NativeFunction::from_fn_ptr(btoa_impl))
            .name(js_string!("btoa"))
            .length(1)
            .build();
    context.register_global_property(js_string!("btoa"), btoa_fn, Attribute::all())?;

    Ok(())
}
