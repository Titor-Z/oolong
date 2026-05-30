#![allow(non_snake_case)]

use crate::web::streams::strategy::StreamQueue;

use boa_engine::object::ObjectInitializer;
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

// ── ReadableStreamDefaultController ─────────────────────────────────────────

#[derive(Debug, Clone, Trace, Finalize, JsData)]
pub struct ReadableStreamDefaultController {
    close_requested: bool,
    errored: bool,
    stored_error: JsValue,
    queue: StreamQueue,
}

#[boa_class(rename = "ReadableStreamDefaultController")]
impl ReadableStreamDefaultController {
    #[boa(constructor)]
    pub fn constructor() -> JsResult<Self> {
        Ok(Self {
            close_requested: false,
            errored: false,
            stored_error: JsValue::undefined(),
            queue: StreamQueue::new(),
        })
    }

    pub fn enqueue(&mut self, chunk: JsValue) -> JsResult<JsValue> {
        if self.close_requested || self.errored {
            return Err(JsNativeError::typ()
                .with_message("无法向已关闭或出错的流 enqueue")
                .into());
        }
        self.queue.enqueue(chunk);
        Ok(JsValue::undefined())
    }

    pub fn close(&mut self) -> JsResult<JsValue> {
        if self.close_requested || self.errored {
            return Err(JsNativeError::typ()
                .with_message("无法关闭已关闭或出错的流")
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

    #[boa(getter)]
    pub fn desiredSize(&self) -> JsValue {
        if self.errored {
            return JsValue::null();
        }
        if self.close_requested {
            return JsValue::from(0);
        }
        JsValue::from(self.queue.size() as f64)
    }
}

// ── ReadableStreamDefaultReader（直接持有控制器引用）────────────────────

#[derive(Debug, Clone, Trace, Finalize, JsData)]
pub struct ReadableStreamDefaultReader {
    controller: JsValue,
}

impl ReadableStreamDefaultReader {
    fn new(controller: JsValue) -> Self {
        Self { controller }
    }
}

#[boa_class(rename = "ReadableStreamDefaultReader")]
impl ReadableStreamDefaultReader {
    #[boa(constructor)]
    pub fn constructor(ctrl: JsValue) -> JsResult<Self> {
        Ok(Self::new(ctrl))
    }

    pub fn read(&mut self, ctx: &mut Context) -> JsResult<JsValue> {
        if let Some(controller) = self.controller.as_object()
            && let Some(mut ctrl) = controller.downcast_mut::<ReadableStreamDefaultController>()
        {
            if ctrl.errored {
                return Err(JsNativeError::typ().with_message("流已出错").into());
            }
            if !ctrl.queue.is_empty() {
                let chunk = ctrl.queue.dequeue().unwrap_or(JsValue::undefined());
                return Ok(make_read_result(chunk, false, ctx));
            }
            if ctrl.close_requested && ctrl.queue.is_empty() {
                return Ok(make_read_result(JsValue::undefined(), true, ctx));
            }
        }
        Ok(make_read_result(JsValue::undefined(), false, ctx))
    }

    pub fn releaseLock(&mut self, _ctx: &mut Context) -> JsResult<JsValue> {
        self.controller = JsValue::undefined();
        Ok(JsValue::undefined())
    }
}

fn make_read_result(value: JsValue, done: bool, ctx: &mut Context) -> JsValue {
    let obj = ObjectInitializer::new(ctx)
        .property(
            js_string!("value"),
            value,
            boa_engine::property::Attribute::WRITABLE
                | boa_engine::property::Attribute::ENUMERABLE
                | boa_engine::property::Attribute::CONFIGURABLE,
        )
        .property(
            js_string!("done"),
            JsValue::from(done),
            boa_engine::property::Attribute::WRITABLE
                | boa_engine::property::Attribute::ENUMERABLE
                | boa_engine::property::Attribute::CONFIGURABLE,
        )
        .build();
    obj.into()
}

// ── ReadableStream ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Trace, Finalize, JsData)]
pub struct ReadableStream {
    state: u8,
    stored_error: JsValue,
    reader: JsValue,
    controller: JsValue,
}

#[boa_class(rename = "ReadableStream")]
impl ReadableStream {
    #[boa(constructor)]
    pub fn constructor(
        underlying_source: Option<JsValue>,
        _strategy: Option<JsValue>,
        ctx: &mut Context,
    ) -> JsResult<Self> {
        let source = underlying_source.unwrap_or(JsValue::undefined());

        let ctrl_proto = get_class_prototype(ctx, "ReadableStreamDefaultController");
        let ctrl_data = ReadableStreamDefaultController::constructor()?;
        let ctrl_obj =
            ObjectInitializer::with_native_data_and_proto(ctrl_data, ctrl_proto, ctx).build();

        if let Some(src_obj) = source.as_object()
            && let Ok(start_val) = src_obj.get(js_string!("start"), ctx)
            && let Some(start_fn) = start_val.as_object().filter(|o| o.is_callable())
        {
            let _ = start_fn.call(
                &JsValue::undefined(),
                &[JsValue::from(ctrl_obj.clone())],
                ctx,
            );
        }

        Ok(Self {
            state: 0,
            stored_error: JsValue::undefined(),
            reader: JsValue::undefined(),
            controller: JsValue::from(ctrl_obj),
        })
    }

    #[boa(getter)]
    pub fn locked(&self) -> bool {
        !self.reader.is_undefined()
    }

    pub fn cancel(&self, reason: Option<JsValue>, ctx: &mut Context) -> JsResult<JsValue> {
        let val = reason.unwrap_or(JsValue::undefined());
        Ok(boa_engine::object::builtins::JsPromise::resolve(val, ctx).into())
    }

    pub fn getReader(&mut self, ctx: &mut Context) -> JsResult<JsValue> {
        if !self.reader.is_undefined() {
            return Err(JsNativeError::typ()
                .with_message("ReadableStream 已被锁定")
                .into());
        }

        let reader_proto = get_class_prototype(ctx, "ReadableStreamDefaultReader");
        let reader_data = ReadableStreamDefaultReader::new(self.controller.clone());
        let reader_obj =
            ObjectInitializer::with_native_data_and_proto(reader_data, reader_proto, ctx).build();

        self.reader = JsValue::from(reader_obj.clone());
        Ok(JsValue::from(reader_obj))
    }
}

// ── Registration ─────────────────────────────────────────────────────────────

pub fn register_globals(context: &mut Context) -> JsResult<()> {
    context.register_global_class::<ReadableStream>()?;
    context.register_global_class::<ReadableStreamDefaultReader>()?;
    context.register_global_class::<ReadableStreamDefaultController>()?;
    Ok(())
}
