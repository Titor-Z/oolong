use std::path::Path;

use boa_engine::{
  Context, JsError, JsNativeError, JsObject, JsResult, JsString, JsValue, Source,
};

/// CJS 模块包装

/// 加载 .cjs 文件并返回 module.exports
/// 供 ModuleLoader 调用
pub fn load_cjs_file(
  resolved: &Path,
  require_fn: Option<JsValue>,
  ctx: &mut Context,
) -> JsResult<JsValue> {
  let source_str = std::fs::read_to_string(resolved).map_err(|e| {
    JsError::from(JsNativeError::typ().with_message(format!(
      "Cannot read module '{}': {e}",
      resolved.display()
    )))
  })?;

  let dir = resolved
    .parent()
    .unwrap_or(Path::new("/"))
    .to_string_lossy()
    .to_string();
  let filename = resolved.to_string_lossy().to_string();

  let wrapped = format!(
    r#"(function(exports, require, module, __filename, __dirname) {{
{source_str}
}})"#,
    source_str = source_str
  );

  let source = Source::from_bytes(wrapped.as_bytes());
  let fn_val = ctx.eval(source).map_err(|e| {
    JsError::from(JsNativeError::syntax().with_message(format!(
      "Error wrapping CJS module '{}': {e}",
      resolved.display()
    )))
  })?;

  let exports = JsObject::with_object_proto(ctx.intrinsics());
  let module_obj = JsObject::with_object_proto(ctx.intrinsics());
  let _ = module_obj.set::<JsString, JsValue>(
    boa_engine::js_string!("exports"),
    exports.clone().into(),
    false,
    ctx,
  );
  let _ = module_obj.set::<JsString, JsValue>(
    boa_engine::js_string!("filename"),
    JsValue::from(JsString::from(filename.clone())),
    false,
    ctx,
  );
  let _ = module_obj.set::<JsString, JsValue>(
    boa_engine::js_string!("id"),
    JsValue::from(JsString::from(filename.clone())),
    false,
    ctx,
  );

  let require_val = require_fn.unwrap_or_else(|| {
    // 默认 require: 仅支持内置模块
    let f: boa_engine::NativeFunction = {
      use boa_engine::IntoJsFunctionCopied;
      (|spec: JsString, _ctx: &mut Context| -> JsResult<JsValue> {
        let s = spec.to_std_string_escaped();
        // 尝试加载为 ESM builtin
        Err(JsError::from(JsNativeError::typ().with_message(format!(
          "require('{}') not yet supported in CJS modules. Try using import instead.",
          s
        ))))
      })
      .into_js_function_copied(ctx)
    };
    boa_engine::object::FunctionObjectBuilder::new(ctx.realm(), f)
      .name(JsString::from("require"))
      .length(1)
      .build()
      .into()
  });

  let args = [
    exports.clone().into(),
    require_val,
    module_obj.clone().into(),
    JsValue::from(JsString::from(filename)),
    JsValue::from(JsString::from(dir)),
  ];
  if let Some(obj) = fn_val.as_object() {
    let _ = obj.call(&JsValue::undefined(), &args, ctx);
  }

  let module_exports = module_obj
    .get(boa_engine::js_string!("exports"), ctx)
    .unwrap_or(exports.into());

  Ok(module_exports)
}
