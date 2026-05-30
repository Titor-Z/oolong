#![allow(non_snake_case)]

use boa_engine::object::builtins::JsPromise;
use boa_engine::object::{FunctionObjectBuilder, ObjectInitializer};
use boa_engine::{
    Context, JsData, JsNativeError, JsObject, JsResult, JsValue, NativeFunction, boa_class,
    js_string,
};
use boa_gc::{Finalize, Trace};

fn get_class_prototype(ctx: &mut Context, name: &str) -> JsObject {
    let global = ctx.global_object();
    let ctor = global.get(js_string!(name), ctx).ok().unwrap();
    let proto_val = ctor
        .as_object()
        .unwrap()
        .get(js_string!("prototype"), ctx)
        .ok()
        .unwrap();
    proto_val.as_object().unwrap().clone()
}

fn sink_write_impl(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let chunk = args.first().cloned().unwrap_or(JsValue::undefined());
    let ws_ctrl = args.get(1).cloned().unwrap_or(JsValue::undefined());
    let ts_val = ws_ctrl
        .as_object()
        .and_then(|c| c.get(js_string!("_tsController"), ctx).ok())
        .unwrap_or(JsValue::undefined());
    if let Some(ts_obj) = ts_val.as_object()
        && let Ok(tr_val) = ts_obj.get(js_string!("_transformer"), ctx)
        && let Some(tr_obj) = tr_val.as_object()
        && let Ok(tr_fn) = tr_obj.get(js_string!("transform"), ctx)
        && let Some(fn_obj) = tr_fn.as_object().filter(|o| o.is_callable())
    {
        let _ = fn_obj.call(&JsValue::undefined(), &[chunk, ts_val], ctx);
    }
    Ok(JsPromise::resolve(JsValue::undefined(), ctx).into())
}

fn sink_close_impl(this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    // close() 不带参数，但从 this（sink 对象）找 tsController
    let ts_val = this
        .as_object()
        .and_then(|c| c.get(js_string!("_tsController"), ctx).ok())
        .or_else(|| {
            args.first()
                .and_then(|a| a.as_object())
                .and_then(|c| c.get(js_string!("_tsController"), ctx).ok())
        })
        .unwrap_or(JsValue::undefined());
    if let Some(ts_obj) = ts_val.as_object()
        && let Ok(tr_val) = ts_obj.get(js_string!("_transformer"), ctx)
        && let Some(tr_obj) = tr_val.as_object()
        && let Ok(flush_fn) = tr_obj.get(js_string!("flush"), ctx)
        && let Some(fn_obj) = flush_fn.as_object().filter(|o| o.is_callable())
    {
        let _ = fn_obj.call(&JsValue::undefined(), &[ts_val], ctx);
    }
    Ok(JsPromise::resolve(JsValue::undefined(), ctx).into())
}

// ── TransformStreamDefaultController ─────────────────────────────────────────

#[derive(Debug, Clone, Trace, Finalize, JsData)]
pub struct TransformStreamDefaultController {
    pub readable_controller: JsValue,
    close_requested: bool,
    errored: bool,
    stored_error: JsValue,
}

#[boa_class(rename = "TransformStreamDefaultController")]
impl TransformStreamDefaultController {
    #[boa(constructor)]
    pub fn constructor() -> JsResult<Self> {
        Ok(Self {
            readable_controller: JsValue::undefined(),
            close_requested: false,
            errored: false,
            stored_error: JsValue::undefined(),
        })
    }

    pub fn enqueue(&mut self, chunk: JsValue) -> JsResult<JsValue> {
        if self.errored || self.close_requested {
            return Err(JsNativeError::typ()
                .with_message("TransformStream 控制器已关闭或出错")
                .into());
        }
        if let Some(ctrl_obj) = self.readable_controller.as_object()
            && let Some(mut ctrl) = ctrl_obj
                .downcast_mut::<crate::web::streams::readable::ReadableStreamDefaultController>(
            )
        {
            let _ = ctrl.enqueue(chunk);
        }
        Ok(JsValue::undefined())
    }

    pub fn close(&mut self) -> JsResult<JsValue> {
        if self.errored || self.close_requested {
            return Err(JsNativeError::typ()
                .with_message("TransformStream 控制器已关闭或出错")
                .into());
        }
        self.close_requested = true;
        Ok(JsValue::undefined())
    }

    pub fn error(&mut self, e: JsValue) -> JsResult<JsValue> {
        if self.errored {
            return Err(JsNativeError::typ().with_message("控制器已出错").into());
        }
        self.errored = true;
        self.stored_error = e;
        Ok(JsValue::undefined())
    }

    pub fn terminate(&mut self) -> JsResult<JsValue> {
        self.close_requested = true;
        if let Some(ctrl_obj) = self.readable_controller.as_object()
            && let Some(mut ctrl) = ctrl_obj
                .downcast_mut::<crate::web::streams::readable::ReadableStreamDefaultController>(
            )
        {
            let _ = ctrl.close();
        }
        Ok(JsValue::undefined())
    }

    #[boa(getter)]
    pub fn desiredSize(&self) -> JsValue {
        if self.errored {
            return JsValue::null();
        }
        if self.close_requested {
            return JsValue::from(0);
        }
        JsValue::from(1)
    }
}

// ── TransformStream ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Trace, Finalize, JsData)]
pub struct TransformStream {
    readable: JsValue,
    writable: JsValue,
    pub controller: JsValue,
}

#[boa_class(rename = "TransformStream")]
impl TransformStream {
    #[boa(constructor)]
    pub fn constructor(
        transformer: Option<JsValue>,
        _writable_strategy: Option<JsValue>,
        _readable_strategy: Option<JsValue>,
        ctx: &mut Context,
    ) -> JsResult<Self> {
        let transformer_val = transformer.unwrap_or(JsValue::undefined());

        // 1. 创建 ReadableStream
        let rs_proto = get_class_prototype(ctx, "ReadableStream");
        let rs = crate::web::streams::readable::ReadableStream::constructor(
            Some(JsValue::undefined()),
            None,
            ctx,
        )?;
        let rs_obj = ObjectInitializer::with_native_data_and_proto(rs, rs_proto, ctx).build();
        let readable_ctrl = rs_obj
            .downcast_mut::<crate::web::streams::readable::ReadableStream>()
            .map(|r| r.controller.clone())
            .unwrap_or(JsValue::undefined());

        // 2. 创建 TransformStreamDefaultController
        let ts_ctrl_proto = get_class_prototype(ctx, "TransformStreamDefaultController");
        let mut ts_data = TransformStreamDefaultController::constructor()?;
        ts_data.readable_controller = readable_ctrl;
        let ts_obj =
            ObjectInitializer::with_native_data_and_proto(ts_data, ts_ctrl_proto, ctx).build();

        ts_obj.set(
            js_string!("_transformer"),
            transformer_val.clone(),
            false,
            ctx,
        )?;

        // 3. 创建自定义 sink，用 FunctionObjectBuilder
        let sink_obj = JsObject::with_object_proto(ctx.intrinsics());

        // 在 sink 上存储 tsController 引用（供 close/abort 的 this 访问）
        sink_obj.set(
            js_string!("_tsController"),
            JsValue::from(ts_obj.clone()),
            false,
            ctx,
        )?;

        let write_fn =
            FunctionObjectBuilder::new(ctx.realm(), NativeFunction::from_fn_ptr(sink_write_impl))
                .name("write")
                .length(2)
                .build();
        sink_obj.set(js_string!("write"), JsValue::from(write_fn), false, ctx)?;

        let close_fn =
            FunctionObjectBuilder::new(ctx.realm(), NativeFunction::from_fn_ptr(sink_close_impl))
                .name("close")
                .length(0)
                .build();
        sink_obj.set(js_string!("close"), JsValue::from(close_fn), false, ctx)?;

        // 4. 创建 WritableStream
        let ws_proto = get_class_prototype(ctx, "WritableStream");
        let ws = crate::web::streams::writable::WritableStream::constructor(
            Some(JsValue::from(sink_obj)),
            None,
            ctx,
        )?;
        let ws_obj = ObjectInitializer::with_native_data_and_proto(ws, ws_proto, ctx).build();

        // 5. wsController._tsController = tsController
        if let Some(ws_data) = ws_obj
            .downcast_mut::<crate::web::streams::writable::WritableStream>()
            && let Some(ws_ctrl_obj) = ws_data.controller.as_object()
        {
            ws_ctrl_obj.set(
                js_string!("_tsController"),
                JsValue::from(ts_obj.clone()),
                false,
                ctx,
            )?;
        }

        // 6. transformer.start(tsController)
        if let Some(t_obj) = transformer_val.as_object()
            && let Ok(start_val) = t_obj.get(js_string!("start"), ctx)
            && let Some(start_fn) = start_val.as_object().filter(|o| o.is_callable())
        {
            start_fn.call(&JsValue::undefined(), &[JsValue::from(ts_obj.clone())], ctx)?;
        }

        Ok(Self {
            readable: JsValue::from(rs_obj),
            writable: JsValue::from(ws_obj),
            controller: JsValue::from(ts_obj),
        })
    }

    #[boa(getter)]
    pub fn readable(&self) -> JsValue {
        self.readable.clone()
    }

    #[boa(getter)]
    pub fn writable(&self) -> JsValue {
        self.writable.clone()
    }
}

// ── Registration ─────────────────────────────────────────────────────────────

pub fn register_globals(context: &mut Context) -> JsResult<()> {
    context.register_global_class::<TransformStream>()?;
    context.register_global_class::<TransformStreamDefaultController>()?;
    Ok(())
}
