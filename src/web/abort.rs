#![allow(non_snake_case)]

use boa_engine::{
    Context, JsData, JsObject, JsResult, JsValue, boa_class, js_string, object::ObjectInitializer,
};
use boa_gc::{Finalize, Trace};

// ── AbortSignal ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Trace, Finalize)]
struct AbortListener {
    type_: String,
    callback: JsValue,
}

#[derive(Debug, Clone, Trace, Finalize, JsData)]
pub struct AbortSignal {
    aborted: bool,
    reason: JsValue,
    listeners: Vec<AbortListener>,
}

#[boa_class(rename = "AbortSignal")]
impl AbortSignal {
    #[boa(constructor)]
    pub fn constructor() -> JsResult<Self> {
        Ok(Self {
            aborted: false,
            reason: JsValue::undefined(),
            listeners: Vec::new(),
        })
    }

    #[boa(getter)]
    pub fn aborted(&self) -> bool {
        self.aborted
    }

    #[boa(getter)]
    pub fn reason(&self) -> JsValue {
        self.reason.clone()
    }

    pub fn addEventListener(&mut self, type_: String, callback: JsValue) -> JsResult<JsValue> {
        if callback.is_callable() {
            self.listeners.push(AbortListener { type_, callback });
        }
        Ok(JsValue::undefined())
    }

    pub fn removeEventListener(&mut self, type_: String, callback: JsValue) -> JsResult<JsValue> {
        self.listeners.retain(|l| {
            if l.type_ == type_
                && let Some(obj) = l.callback.as_object()
                && let Some(cb_obj) = callback.as_object()
            {
                return !JsObject::equals(&obj, &cb_obj);
            }
            true
        });
        Ok(JsValue::undefined())
    }
}

// Internal methods not exposed to JS
impl AbortSignal {
    pub fn _trigger_abort(&mut self, reason: JsValue, ctx: &mut Context) {
        if self.aborted {
            return;
        }
        self.aborted = true;
        self.reason = reason;

        let listeners = std::mem::take(&mut self.listeners);
        for listener in &listeners {
            if listener.type_ == "abort"
                && let Some(obj) = listener.callback.as_object()
            {
                let _ = obj.call(&JsValue::undefined(), &[], ctx);
            }
        }
    }
}

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

// ── AbortController ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Trace, Finalize, JsData)]
pub struct AbortController {
    signal_data: JsValue,
}

#[boa_class(rename = "AbortController")]
impl AbortController {
    #[boa(constructor)]
    pub fn constructor(ctx: &mut Context) -> JsResult<Self> {
        let signal = AbortSignal::constructor()?;
        let proto = get_class_prototype(ctx, "AbortSignal");
        let signal_obj = ObjectInitializer::with_native_data_and_proto(signal, proto, ctx).build();
        Ok(Self {
            signal_data: JsValue::from(signal_obj),
        })
    }

    #[boa(getter)]
    pub fn signal(&self) -> JsValue {
        self.signal_data.clone()
    }

    pub fn abort(&mut self, reason: Option<JsValue>, ctx: &mut Context) -> JsResult<JsValue> {
        let reason = reason.unwrap_or(JsValue::undefined());
        if let Some(obj) = self.signal_data.as_object()
            && let Some(mut signal) = obj.downcast_mut::<AbortSignal>()
        {
            signal._trigger_abort(reason, ctx);
        }
        Ok(JsValue::undefined())
    }
}

// ── Registration ─────────────────────────────────────────────────────────────

pub fn register_globals(context: &mut Context) -> JsResult<()> {
    context.register_global_class::<AbortSignal>()?;
    context.register_global_class::<AbortController>()?;
    Ok(())
}
