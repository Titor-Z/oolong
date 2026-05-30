#![allow(non_snake_case)]

use boa_engine::object::ObjectInitializer;
use boa_engine::object::builtins::JsPromise;
use boa_engine::{
    Context, JsData, JsNativeError, JsObject, JsResult, JsValue, boa_class, js_string,
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

fn call_sink_method(
    sink_obj: &JsObject,
    method: &str,
    args: &[JsValue],
    ctx: &mut Context,
) -> JsResult<()> {
    if let Ok(fn_val) = sink_obj.get(js_string!(method), ctx)
        && let Some(fn_obj) = fn_val.as_object().filter(|o| o.is_callable())
    {
        let _ = fn_obj.call(&JsValue::undefined(), args, ctx);
    }
    Ok(())
}

// ── WritableStreamDefaultController ─────────────────────────────────────────

#[derive(Debug, Clone, Trace, Finalize, JsData)]
pub struct WritableStreamDefaultController {
    errored: bool,
    stored_error: JsValue,
    underlying_sink: JsValue,
}

#[boa_class(rename = "WritableStreamDefaultController")]
impl WritableStreamDefaultController {
    #[boa(constructor)]
    pub fn constructor() -> JsResult<Self> {
        Ok(Self {
            errored: false,
            stored_error: JsValue::undefined(),
            underlying_sink: JsValue::undefined(),
        })
    }

    pub fn error(&mut self, e: JsValue) -> JsResult<JsValue> {
        if self.errored {
            return Err(JsNativeError::typ().with_message("控制器已出错").into());
        }
        self.errored = true;
        self.stored_error = e;
        Ok(JsValue::undefined())
    }

    pub fn _sink(&self) -> JsValue {
        self.underlying_sink.clone()
    }

    pub fn _set_sink(&mut self, sink: JsValue) {
        self.underlying_sink = sink;
    }
}

// ── WritableStreamDefaultWriter（直接持有控制器引用）─────────────────────

#[derive(Debug, Clone, Trace, Finalize, JsData)]
pub struct WritableStreamDefaultWriter {
    controller: JsValue,
}

impl WritableStreamDefaultWriter {
    fn new(controller: JsValue) -> Self {
        Self { controller }
    }
}

#[boa_class(rename = "WritableStreamDefaultWriter")]
impl WritableStreamDefaultWriter {
    #[boa(constructor)]
    pub fn constructor(ctrl: JsValue) -> JsResult<Self> {
        Ok(Self::new(ctrl))
    }

    pub fn write(&self, chunk: JsValue, ctx: &mut Context) -> JsResult<JsValue> {
        let sink = if let Some(ctrl_obj) = self.controller.as_object()
            && let Some(ctrl) = ctrl_obj.downcast_mut::<WritableStreamDefaultController>()
        {
            if ctrl.errored {
                return Err(JsNativeError::typ().with_message("写入器已释放").into());
            }
            ctrl.underlying_sink.clone()
        } else {
            return Err(JsNativeError::typ().with_message("写入器已释放").into());
        };
        if let Some(sink_obj) = sink.as_object() {
            call_sink_method(&sink_obj, "write", &[chunk, self.controller.clone()], ctx)?;
        }
        Ok(JsPromise::resolve(JsValue::undefined(), ctx).into())
    }

    pub fn close(&self, ctx: &mut Context) -> JsResult<JsValue> {
        let sink = if let Some(ctrl_obj) = self.controller.as_object()
            && let Some(ctrl) = ctrl_obj.downcast_mut::<WritableStreamDefaultController>()
        {
            if ctrl.errored {
                return Err(JsNativeError::typ().with_message("写入器已释放").into());
            }
            ctrl.underlying_sink.clone()
        } else {
            return Err(JsNativeError::typ().with_message("写入器已释放").into());
        };
        if let Some(sink_obj) = sink.as_object() {
            call_sink_method(&sink_obj, "close", &[], ctx)?;
        }
        Ok(JsPromise::resolve(JsValue::undefined(), ctx).into())
    }

    pub fn abort(&self, reason: Option<JsValue>, ctx: &mut Context) -> JsResult<JsValue> {
        let reason_val = reason.unwrap_or(JsValue::undefined());
        let sink = if let Some(ctrl_obj) = self.controller.as_object()
            && let Some(ctrl) = ctrl_obj.downcast_mut::<WritableStreamDefaultController>()
        {
            ctrl.underlying_sink.clone()
        } else {
            return Err(JsNativeError::typ().with_message("写入器已释放").into());
        };
        if let Some(sink_obj) = sink.as_object() {
                call_sink_method(&sink_obj, "abort", std::slice::from_ref(&reason_val), ctx)?;
        }
        Ok(JsPromise::resolve(reason_val, ctx).into())
    }

    pub fn releaseLock(&mut self, _ctx: &mut Context) -> JsResult<JsValue> {
        self.controller = JsValue::undefined();
        Ok(JsValue::undefined())
    }
}

// ── WritableStream ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Trace, Finalize, JsData)]
pub struct WritableStream {
    writer: JsValue,
    controller: JsValue,
}

#[boa_class(rename = "WritableStream")]
impl WritableStream {
    #[boa(constructor)]
    pub fn constructor(
        underlying_sink: Option<JsValue>,
        _strategy: Option<JsValue>,
        ctx: &mut Context,
    ) -> JsResult<Self> {
        let sink = underlying_sink.unwrap_or(JsValue::undefined());

        let ctrl_proto = get_class_prototype(ctx, "WritableStreamDefaultController");
        let ctrl_data = WritableStreamDefaultController::constructor()?;
        let ctrl_obj =
            ObjectInitializer::with_native_data_and_proto(ctrl_data, ctrl_proto, ctx).build();

        if let Some(mut ctrl) = ctrl_obj.downcast_mut::<WritableStreamDefaultController>() {
            ctrl._set_sink(sink.clone());
        }

        if let Some(sink_obj) = sink.as_object() {
            call_sink_method(&sink_obj, "start", &[JsValue::from(ctrl_obj.clone())], ctx)?;
        }

        Ok(Self {
            writer: JsValue::undefined(),
            controller: JsValue::from(ctrl_obj),
        })
    }

    #[boa(getter)]
    pub fn locked(&self) -> bool {
        !self.writer.is_undefined()
    }

    pub fn abort(&self, reason: Option<JsValue>, ctx: &mut Context) -> JsResult<JsValue> {
        let reason_val = reason.unwrap_or(JsValue::undefined());
        // 标记为出错并调用 sink.abort
        if let Some(ctrl_obj) = self.controller.as_object()
            && let Some(mut ctrl) = ctrl_obj.downcast_mut::<WritableStreamDefaultController>()
        {
            ctrl.errored = true;
            ctrl.stored_error = reason_val.clone();
            let sink = ctrl.underlying_sink.clone();
            if let Some(sink_obj) = sink.as_object() {
            call_sink_method(&sink_obj, "abort", std::slice::from_ref(&reason_val), ctx)?;
            }
        }
        Ok(JsPromise::resolve(reason_val, ctx).into())
    }

    pub fn close(&self, ctx: &mut Context) -> JsResult<JsValue> {
        let sink = if let Some(ctrl_obj) = self.controller.as_object()
            && let Some(ctrl) = ctrl_obj.downcast_mut::<WritableStreamDefaultController>()
        {
            ctrl.underlying_sink.clone()
        } else {
            return Err(JsNativeError::typ().with_message("控制器已释放").into());
        };
        if let Some(sink_obj) = sink.as_object() {
            call_sink_method(&sink_obj, "close", &[], ctx)?;
        }
        Ok(JsPromise::resolve(JsValue::undefined(), ctx).into())
    }

    pub fn getWriter(&mut self, ctx: &mut Context) -> JsResult<JsValue> {
        if !self.writer.is_undefined() {
            return Err(JsNativeError::typ()
                .with_message("WritableStream 已被锁定")
                .into());
        }

        let writer_proto = get_class_prototype(ctx, "WritableStreamDefaultWriter");
        let writer_data = WritableStreamDefaultWriter::new(self.controller.clone());
        let writer_obj =
            ObjectInitializer::with_native_data_and_proto(writer_data, writer_proto, ctx).build();

        self.writer = JsValue::from(writer_obj.clone());
        Ok(JsValue::from(writer_obj))
    }
}

// ── Registration ─────────────────────────────────────────────────────────────

pub fn register_globals(context: &mut Context) -> JsResult<()> {
    context.register_global_class::<WritableStream>()?;
    context.register_global_class::<WritableStreamDefaultWriter>()?;
    context.register_global_class::<WritableStreamDefaultController>()?;
    Ok(())
}
