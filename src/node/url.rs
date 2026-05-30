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

pub fn create_node_url_module(context: &mut Context) -> Result<Module, String> {
    let export_names: &[JsString] = &[
        js_string!("URL"),
        js_string!("URLSearchParams"),
        js_string!("fileURLToPath"),
        js_string!("pathToFileURL"),
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

                m.set_export(&js_string!("URL"), url_class.clone())?;
                m.set_export(&js_string!("URLSearchParams"), search_params_class.clone())?;
                m.set_export(&js_string!("fileURLToPath"), file_url_to_path.clone())?;
                m.set_export(&js_string!("pathToFileURL"), path_to_file_url.clone())?;

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
