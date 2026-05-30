use boa_engine::module::SyntheticModuleInitializer;
use boa_engine::object::FunctionObjectBuilder;
use boa_engine::{
    Context, JsObject, JsResult, JsString, JsValue, Module, NativeFunction, js_string,
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

fn v4_impl(_: &JsValue, _args: &[JsValue], _ctx: &mut Context) -> JsResult<JsValue> {
    let u = uuid::Uuid::new_v4();
    Ok(JsValue::from(js_string!(u.to_string())))
}

fn validate_impl(_: &JsValue, args: &[JsValue], _ctx: &mut Context) -> JsResult<JsValue> {
    let input = args
        .first()
        .and_then(|v| v.as_string())
        .map(|s| s.to_std_string_escaped())
        .unwrap_or_default();
    let valid = uuid::Uuid::parse_str(&input).is_ok();
    Ok(JsValue::from(valid))
}

pub fn create_uuid_module(context: &mut Context) -> Result<Module, String> {
    let export_names: &[JsString] = &[
        js_string!("v4"),
        js_string!("validate"),
        js_string!("default"),
    ];

    let module = Module::synthetic(
        export_names,
        SyntheticModuleInitializer::from_copy_closure(
            |m: &boa_engine::module::SyntheticModule, ctx: &mut Context| {
                let v4_fn = make_fn(NativeFunction::from_fn_ptr(v4_impl), "v4", 0, ctx);
                let validate_fn = make_fn(
                    NativeFunction::from_fn_ptr(validate_impl),
                    "validate",
                    1,
                    ctx,
                );

                m.set_export(&js_string!("v4"), v4_fn.clone())?;
                m.set_export(&js_string!("validate"), validate_fn.clone())?;

                let default_obj = JsObject::with_object_proto(ctx.intrinsics());
                default_obj.set(js_string!("v4"), v4_fn, false, ctx).ok();
                default_obj
                    .set(js_string!("validate"), validate_fn, false, ctx)
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
