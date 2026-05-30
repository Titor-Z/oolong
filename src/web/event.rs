#![allow(non_snake_case)]

use boa_engine::{
    Context, JsData, JsObject, JsResult, JsString, JsValue, boa_class, js_error, js_string,
};
use boa_gc::{Finalize, Trace};

// ── Event ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Trace, Finalize, JsData)]
pub struct Event {
    type_: String,
    target: JsValue,
    current_target: JsValue,
    default_prevented: bool,
    propagation_stopped: bool,
    cancelable: bool,
    bubbles: bool,
}

impl Event {
    pub fn set_target(&mut self, target: JsValue) {
        self.target = target;
    }
    pub fn set_current_target(&mut self, target: JsValue) {
        self.current_target = target;
    }
    pub fn get_type(&self) -> &str {
        &self.type_
    }
    pub fn is_propagation_stopped(&self) -> bool {
        self.propagation_stopped
    }
    pub fn is_default_prevented(&self) -> bool {
        self.default_prevented
    }
}

#[boa_class(rename = "Event")]
impl Event {
    #[boa(constructor)]
    pub fn constructor(type_: JsString, options: Option<JsObject>) -> JsResult<Self> {
        let mut bubbles = false;
        let mut cancelable = false;
        if let Some(opts) = options {
            if let Ok(val) = opts.get(js_string!("bubbles"), &mut Context::default()) {
                bubbles = val.to_boolean();
            }
            if let Ok(val) = opts.get(js_string!("cancelable"), &mut Context::default()) {
                cancelable = val.to_boolean();
            }
        }
        Ok(Self {
            type_: type_.to_std_string_escaped(),
            target: JsValue::undefined(),
            current_target: JsValue::undefined(),
            default_prevented: false,
            propagation_stopped: false,
            cancelable,
            bubbles,
        })
    }

    #[boa(getter)]
    #[boa(rename = "type")]
    pub fn r#type(&self) -> JsString {
        JsString::from(self.type_.as_str())
    }

    #[boa(getter)]
    pub fn target(&self) -> JsValue {
        self.target.clone()
    }

    #[boa(getter)]
    pub fn currentTarget(&self) -> JsValue {
        self.current_target.clone()
    }

    #[boa(getter)]
    pub fn defaultPrevented(&self) -> bool {
        self.default_prevented
    }

    #[boa(getter)]
    pub fn cancelable(&self) -> bool {
        self.cancelable
    }

    #[boa(getter)]
    pub fn bubbles(&self) -> bool {
        self.bubbles
    }

    pub fn stopPropagation(&mut self) {
        self.propagation_stopped = true;
    }

    pub fn preventDefault(&mut self) {
        if self.cancelable {
            self.default_prevented = true;
        }
    }
}

// ── EventTarget ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Trace, Finalize)]
struct ListenerEntry {
    type_: String,
    callback: JsValue,
}

#[derive(Debug, Clone, Trace, Finalize, JsData)]
pub struct EventTarget {
    listeners: Vec<ListenerEntry>,
}

#[boa_class(rename = "EventTarget")]
impl EventTarget {
    #[boa(constructor)]
    pub fn constructor() -> JsResult<Self> {
        Ok(Self {
            listeners: Vec::new(),
        })
    }

    pub fn addEventListener(&mut self, type_: JsString, callback: JsValue) -> JsResult<JsValue> {
        if callback.is_callable() {
            self.listeners.push(ListenerEntry {
                type_: type_.to_std_string_escaped(),
                callback,
            });
        }
        Ok(JsValue::undefined())
    }

    pub fn removeEventListener(&mut self, type_: JsString, callback: JsValue) -> JsResult<JsValue> {
        let type_str = type_.to_std_string_escaped();
        self.listeners.retain(|l| {
            if l.type_ == type_str
                && let Some(obj) = l.callback.as_object()
                && let Some(cb_obj) = callback.as_object()
            {
                return !JsObject::equals(&obj, &cb_obj);
            }
            true
        });
        Ok(JsValue::undefined())
    }

    pub fn dispatchEvent(&mut self, event: JsValue, ctx: &mut Context) -> JsResult<bool> {
        let event_obj = event
            .as_object()
            .ok_or_else(|| js_error!(TypeError: "Event expected"))?;

        let type_str = event_obj
            .downcast_ref::<Event>()
            .ok_or_else(|| js_error!(TypeError: "Value is not an Event"))?
            .get_type()
            .to_string();

        // Take all listeners, separate matching from non-matching to avoid reentrancy issues
        let all_listeners = std::mem::take(&mut self.listeners);
        let mut matching = Vec::new();
        let mut non_matching = Vec::new();
        for listener in all_listeners {
            if listener.type_ == type_str {
                matching.push(listener);
            } else {
                non_matching.push(listener);
            }
        }
        self.listeners = non_matching;

        for entry in &matching {
            if event_obj
                .downcast_ref::<Event>()
                .map(|e| e.is_propagation_stopped())
                .unwrap_or(false)
            {
                break;
            }

            if let Some(obj) = entry.callback.as_object() {
                let _ = obj.call(&JsValue::undefined(), std::slice::from_ref(&event), ctx);
            }

            if event_obj
                .downcast_ref::<Event>()
                .map(|e| e.is_propagation_stopped())
                .unwrap_or(false)
            {
                break;
            }
        }

        // Restore all listeners (matching ones persist for future dispatches)
        self.listeners.extend(matching);

        let default_prevented = event_obj
            .downcast_ref::<Event>()
            .map(|e| e.is_default_prevented())
            .unwrap_or(false);

        Ok(!default_prevented)
    }
}

// ── Registration ──────────────────────────────────────────────────────────────

pub fn register_globals(context: &mut Context) -> JsResult<()> {
    context.register_global_class::<Event>()?;
    context.register_global_class::<EventTarget>()?;
    Ok(())
}
