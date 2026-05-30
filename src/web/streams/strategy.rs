#![allow(non_snake_case)]

use boa_engine::{Context, JsData, JsNativeError, JsResult, JsValue, boa_class, js_string};
use boa_gc::{Finalize, Trace};

// ── CountQueuingStrategy ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Trace, Finalize, JsData)]
pub struct CountQueuingStrategy {
    high_water_mark: f64,
}

#[boa_class(rename = "CountQueuingStrategy")]
impl CountQueuingStrategy {
    #[boa(constructor)]
    pub fn constructor(init: JsValue, ctx: &mut Context) -> JsResult<Self> {
        let obj = init.as_object().ok_or_else(|| {
            JsNativeError::typ()
                .with_message("CountQueuingStrategy 构造参数必须是 { highWaterMark }")
        })?;
        let hwm = obj.get(js_string!("highWaterMark"), ctx)?;
        Ok(Self {
            high_water_mark: hwm.to_number(ctx)?,
        })
    }

    #[boa(getter)]
    pub fn highWaterMark(&self) -> f64 {
        self.high_water_mark
    }

    pub fn size(&self, _chunk: JsValue) -> f64 {
        1.0
    }
}

// ── ByteLengthQueuingStrategy ────────────────────────────────────────────────

#[derive(Debug, Clone, Trace, Finalize, JsData)]
pub struct ByteLengthQueuingStrategy {
    high_water_mark: f64,
}

#[boa_class(rename = "ByteLengthQueuingStrategy")]
impl ByteLengthQueuingStrategy {
    #[boa(constructor)]
    pub fn constructor(init: JsValue, ctx: &mut Context) -> JsResult<Self> {
        let obj = init.as_object().ok_or_else(|| {
            JsNativeError::typ()
                .with_message("ByteLengthQueuingStrategy 构造参数必须是 { highWaterMark }")
        })?;
        let hwm = obj.get(js_string!("highWaterMark"), ctx)?;
        Ok(Self {
            high_water_mark: hwm.to_number(ctx)?,
        })
    }

    #[boa(getter)]
    pub fn highWaterMark(&self) -> f64 {
        self.high_water_mark
    }

    pub fn size(&self, chunk: JsValue, ctx: &mut Context) -> f64 {
        if let Some(obj) = chunk.as_object() {
            if let Ok(bl) = obj.get(js_string!("byteLength"), ctx) {
                return bl.to_number(ctx).unwrap_or(1.0);
            }
        }
        1.0
    }
}

// ── StreamQueue（内部结构，供后续步骤使用）───────────────────────────────

#[derive(Debug, Clone, Trace, Finalize)]
pub struct StreamQueue {
    chunks: Vec<JsValue>,
}

impl StreamQueue {
    pub fn new() -> Self {
        Self { chunks: Vec::new() }
    }

    pub fn enqueue(&mut self, chunk: JsValue) {
        self.chunks.push(chunk);
    }

    pub fn dequeue(&mut self) -> Option<JsValue> {
        if self.chunks.is_empty() {
            None
        } else {
            Some(self.chunks.remove(0))
        }
    }

    pub fn size(&self) -> usize {
        self.chunks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    pub fn clear(&mut self) {
        self.chunks.clear();
    }
}

// ── Registration ─────────────────────────────────────────────────────────────

pub fn register_globals(context: &mut Context) -> JsResult<()> {
    context.register_global_class::<CountQueuingStrategy>()?;
    context.register_global_class::<ByteLengthQueuingStrategy>()?;
    Ok(())
}
