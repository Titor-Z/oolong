use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use boa_engine::{Context, JsError, JsNativeError, JsObject, JsResult, JsString, JsValue, Source};

// CJS 模块缓存（thread_local，避免跨线程访问非 Send 的 JsValue）
thread_local! {
    pub static CJS_CACHE: RefCell<HashMap<PathBuf, JsValue>> = RefCell::new(HashMap::new());
}

/// 清空 CJS 缓存
pub fn clear_cjs_cache() {
    CJS_CACHE.with(|c| c.borrow_mut().clear());
}

/// CJS 模块包装
///
/// 加载 .cjs 文件并返回 module.exports
/// 调用者必须提供 require 函数（通过 JsValue）
pub fn load_cjs_file(
    resolved: &Path,
    require_fn: JsValue,
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let source_str = std::fs::read_to_string(resolved).map_err(|e| {
        JsError::from(
            JsNativeError::typ()
                .with_message(format!("Cannot read module '{}': {e}", resolved.display())),
        )
    })?;

    let dir = resolved
        .parent()
        .unwrap_or(Path::new("/"))
        .to_string_lossy()
        .to_string();
    let filename = resolved.to_string_lossy().to_string();

    // NOTE: 前置 eval 防止 Boa 0.21.1 的 scope analyzer 优化掉非转义 var
    // 没有此 workaround，DefInitVar 会访问函数环境中不存在的 slot 而崩溃
    let wrapped = format!(
        r#"(function(exports, require, module, __filename, __dirname) {{
void eval("");
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

    let args = [
        exports.clone().into(),
        require_fn,
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
