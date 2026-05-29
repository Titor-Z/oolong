#![allow(non_snake_case)]

use std::time::{Instant, SystemTime, UNIX_EPOCH};

use boa_engine::{
    Context, JsData, JsObject, JsResult, JsString, JsValue, boa_class, js_string,
    object::builtins::JsArray, object::ObjectInitializer, property::Attribute,
};
use boa_gc::{Finalize, Trace};

fn unix_time_ms() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
        * 1000.0
}

fn get_class_prototype(ctx: &mut Context, name: &str) -> JsObject {
    let global = ctx.global_object();
    let ctor = global.get(js_string!(name), ctx).ok().unwrap();
    let proto_val = ctor.as_object().unwrap().get(js_string!("prototype"), ctx).ok().unwrap();
    proto_val.as_object().unwrap().clone()
}

// ── PerformanceEntry ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, JsData, Trace, Finalize)]
pub struct PerformanceEntry {
    name: String,
    entry_type: String,
    #[unsafe_ignore_trace]
    start_time: f64,
    #[unsafe_ignore_trace]
    duration: f64,
}

#[boa_class(rename = "PerformanceEntry")]
impl PerformanceEntry {
    #[boa(constructor)]
    pub fn constructor(name: String, entry_type: String, start_time: f64, duration: f64) -> JsResult<Self> {
        Ok(Self { name, entry_type, start_time, duration })
    }

    #[boa(getter)]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    #[boa(getter)]
    pub fn entryType(&self) -> String {
        self.entry_type.clone()
    }

    #[boa(getter)]
    pub fn startTime(&self) -> f64 {
        self.start_time
    }

    #[boa(getter)]
    pub fn duration(&self) -> f64 {
        self.duration
    }

    pub fn toJSON(&self, ctx: &mut Context) -> JsResult<JsValue> {
        let obj = ObjectInitializer::new(ctx)
            .property(js_string!("name"), JsValue::from(JsString::from(self.name.clone())), Attribute::WRITABLE | Attribute::ENUMERABLE | Attribute::CONFIGURABLE)
            .property(js_string!("entryType"), JsValue::from(JsString::from(self.entry_type.clone())), Attribute::WRITABLE | Attribute::ENUMERABLE | Attribute::CONFIGURABLE)
            .property(js_string!("startTime"), JsValue::from(self.start_time), Attribute::WRITABLE | Attribute::ENUMERABLE | Attribute::CONFIGURABLE)
            .property(js_string!("duration"), JsValue::from(self.duration), Attribute::WRITABLE | Attribute::ENUMERABLE | Attribute::CONFIGURABLE)
            .build();
        Ok(obj.into())
    }
}

// ── PerformanceMark ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, JsData, Trace, Finalize)]
pub struct PerformanceMark {
    name: String,
    #[unsafe_ignore_trace]
    start_time: f64,
}

#[boa_class(rename = "PerformanceMark")]
impl PerformanceMark {
    #[boa(constructor)]
    pub fn constructor(name: String, options: Option<JsObject>, ctx: &mut Context) -> JsResult<Self> {
        let start_time = if let Some(opts) = &options {
            opts.get(js_string!("startTime"), ctx)
                .ok()
                .and_then(|v| v.to_number(ctx).ok())
                .unwrap_or(0.0)
        } else {
            0.0
        };
        Ok(Self { name, start_time })
    }

    #[boa(getter)]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    #[boa(getter)]
    pub fn entryType(&self) -> String {
        "mark".to_string()
    }

    #[boa(getter)]
    pub fn startTime(&self) -> f64 {
        self.start_time
    }

    #[boa(getter)]
    pub fn duration(&self) -> f64 {
        0.0
    }

    pub fn toJSON(&self, ctx: &mut Context) -> JsResult<JsValue> {
        let obj = ObjectInitializer::new(ctx)
            .property(
                js_string!("name"),
                JsValue::from(JsString::from(self.name.clone())),
                Attribute::WRITABLE | Attribute::ENUMERABLE | Attribute::CONFIGURABLE,
            )
            .property(
                js_string!("entryType"),
                js_string!("mark"),
                Attribute::WRITABLE | Attribute::ENUMERABLE | Attribute::CONFIGURABLE,
            )
            .property(
                js_string!("startTime"),
                JsValue::from(self.start_time),
                Attribute::WRITABLE | Attribute::ENUMERABLE | Attribute::CONFIGURABLE,
            )
            .property(
                js_string!("duration"),
                JsValue::from(0.0),
                Attribute::WRITABLE | Attribute::ENUMERABLE | Attribute::CONFIGURABLE,
            )
            .build();
        Ok(obj.into())
    }
}

// ── PerformanceMeasure ───────────────────────────────────────────────────────

#[derive(Debug, Clone, JsData, Trace, Finalize)]
pub struct PerformanceMeasure {
    name: String,
    #[unsafe_ignore_trace]
    start_time: f64,
    #[unsafe_ignore_trace]
    duration: f64,
}

#[boa_class(rename = "PerformanceMeasure")]
impl PerformanceMeasure {
    #[boa(constructor)]
    pub fn constructor(name: String, start_time: f64, duration: f64) -> JsResult<Self> {
        Ok(Self { name, start_time, duration })
    }

    #[boa(getter)]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    #[boa(getter)]
    pub fn entryType(&self) -> String {
        "measure".to_string()
    }

    #[boa(getter)]
    pub fn startTime(&self) -> f64 {
        self.start_time
    }

    #[boa(getter)]
    pub fn duration(&self) -> f64 {
        self.duration
    }

    pub fn toJSON(&self, ctx: &mut Context) -> JsResult<JsValue> {
        let obj = ObjectInitializer::new(ctx)
            .property(
                js_string!("name"),
                JsValue::from(JsString::from(self.name.clone())),
                Attribute::WRITABLE | Attribute::ENUMERABLE | Attribute::CONFIGURABLE,
            )
            .property(
                js_string!("entryType"),
                js_string!("measure"),
                Attribute::WRITABLE | Attribute::ENUMERABLE | Attribute::CONFIGURABLE,
            )
            .property(
                js_string!("startTime"),
                JsValue::from(self.start_time),
                Attribute::WRITABLE | Attribute::ENUMERABLE | Attribute::CONFIGURABLE,
            )
            .property(
                js_string!("duration"),
                JsValue::from(self.duration),
                Attribute::WRITABLE | Attribute::ENUMERABLE | Attribute::CONFIGURABLE,
            )
            .build();
        Ok(obj.into())
    }
}

// ── Performance ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, JsData, Trace, Finalize)]
pub struct Performance {
    #[unsafe_ignore_trace]
    start_time: f64,
    #[unsafe_ignore_trace]
    start_instant: Instant,
    marks: Vec<PerformanceMark>,
    measures: Vec<PerformanceMeasure>,
}

#[boa_class(rename = "Performance")]
impl Performance {
    #[boa(constructor)]
    pub fn constructor() -> JsResult<Self> {
        Ok(Self {
            start_time: unix_time_ms(),
            start_instant: Instant::now(),
            marks: Vec::new(),
            measures: Vec::new(),
        })
    }

    pub fn now(&self) -> f64 {
        self.start_instant.elapsed().as_secs_f64() * 1000.0
    }

    #[boa(getter)]
    pub fn timeOrigin(&self) -> f64 {
        self.start_time
    }

    pub fn mark(
        &mut self,
        name: String,
        options: Option<JsObject>,
        ctx: &mut Context,
    ) -> JsResult<JsValue> {
        let start_time = if let Some(opts) = &options {
            opts.get(js_string!("startTime"), ctx)
                .ok()
                .and_then(|v| v.to_number(ctx).ok())
                .unwrap_or_else(|| self.now())
        } else {
            self.now()
        };
        let mark = PerformanceMark { name: name.clone(), start_time };
        let proto = get_class_prototype(ctx, "PerformanceMark");
        let obj = ObjectInitializer::with_native_data_and_proto(mark.clone(), proto, ctx).build();
        self.marks.push(mark);
        Ok(obj.into())
    }

    pub fn measure(
        &mut self,
        name: String,
        start_mark: Option<JsValue>,
        end_mark: Option<JsValue>,
        ctx: &mut Context,
    ) -> JsResult<JsValue> {
        let start_time = if let Some(ref sm) = start_mark {
            if let Some(s) = sm.as_string() {
                self.find_mark_start_time(s.to_std_string_escaped())
            } else {
                sm.to_number(ctx).unwrap_or(0.0)
            }
        } else {
            0.0
        };

        let end_time = if let Some(ref em) = end_mark {
            if let Some(s) = em.as_string() {
                self.find_mark_start_time(s.to_std_string_escaped())
            } else if let Ok(n) = em.to_number(ctx) {
                n
            } else {
                self.now()
            }
        } else {
            self.now()
        };

        let duration = end_time - start_time;
        let measure = PerformanceMeasure { name: name.clone(), start_time, duration };
        let proto = get_class_prototype(ctx, "PerformanceMeasure");
        let obj = ObjectInitializer::with_native_data_and_proto(measure.clone(), proto, ctx).build();
        self.measures.push(measure);
        Ok(obj.into())
    }

    pub fn clearMarks(&mut self, name: Option<String>) {
        if let Some(ref n) = name {
            self.marks.retain(|m| m.name != *n);
        } else {
            self.marks.clear();
        }
    }

    pub fn clearMeasures(&mut self, name: Option<String>) {
        if let Some(ref n) = name {
            self.measures.retain(|m| m.name != *n);
        } else {
            self.measures.clear();
        }
    }

    pub fn getEntries(&self, ctx: &mut Context) -> JsResult<JsValue> {
        let mut entries: Vec<JsValue> = Vec::new();
        let mark_proto = get_class_prototype(ctx, "PerformanceMark");
        let measure_proto = get_class_prototype(ctx, "PerformanceMeasure");
        for m in &self.marks {
            let obj = ObjectInitializer::with_native_data_and_proto(m.clone(), mark_proto.clone(), ctx).build();
            entries.push(obj.into());
        }
        for m in &self.measures {
            let obj = ObjectInitializer::with_native_data_and_proto(m.clone(), measure_proto.clone(), ctx).build();
            entries.push(obj.into());
        }
        let arr: JsObject = JsArray::from_iter(entries, ctx).into();
        Ok(arr.into())
    }

    pub fn getEntriesByName(
        &self,
        name: String,
        entry_type: Option<String>,
        ctx: &mut Context,
    ) -> JsResult<JsValue> {
        let mut entries: Vec<JsValue> = Vec::new();
        let mark_proto = get_class_prototype(ctx, "PerformanceMark");
        let measure_proto = get_class_prototype(ctx, "PerformanceMeasure");
        for m in &self.marks {
            if m.name == name
                && (entry_type.is_none() || entry_type.as_deref() == Some("mark"))
            {
                let obj = ObjectInitializer::with_native_data_and_proto(m.clone(), mark_proto.clone(), ctx).build();
                entries.push(obj.into());
            }
        }
        for m in &self.measures {
            if m.name == name
                && (entry_type.is_none() || entry_type.as_deref() == Some("measure"))
            {
                let obj = ObjectInitializer::with_native_data_and_proto(m.clone(), measure_proto.clone(), ctx).build();
                entries.push(obj.into());
            }
        }
        let arr: JsObject = JsArray::from_iter(entries, ctx).into();
        Ok(arr.into())
    }

    pub fn getEntriesByType(
        &self,
        entry_type: String,
        ctx: &mut Context,
    ) -> JsResult<JsValue> {
        let mut entries: Vec<JsValue> = Vec::new();
        let mark_proto = get_class_prototype(ctx, "PerformanceMark");
        let measure_proto = get_class_prototype(ctx, "PerformanceMeasure");
        if entry_type == "mark" {
            for m in &self.marks {
                let obj = ObjectInitializer::with_native_data_and_proto(m.clone(), mark_proto.clone(), ctx).build();
                entries.push(obj.into());
            }
        } else if entry_type == "measure" {
            for m in &self.measures {
                let obj = ObjectInitializer::with_native_data_and_proto(m.clone(), measure_proto.clone(), ctx).build();
                entries.push(obj.into());
            }
        }
        let arr: JsObject = JsArray::from_iter(entries, ctx).into();
        Ok(arr.into())
    }
}

impl Performance {
    fn find_mark_start_time(&self, name: String) -> f64 {
        self.marks
            .iter()
            .rev()
            .find(|m| m.name == name)
            .map(|m| m.start_time)
            .unwrap_or(0.0)
    }
}

// ── Registration ─────────────────────────────────────────────────────────────

pub fn register_globals(context: &mut Context) -> JsResult<()> {
    context.register_global_class::<PerformanceEntry>()?;
    context.register_global_class::<PerformanceMark>()?;
    context.register_global_class::<PerformanceMeasure>()?;
    context.register_global_class::<Performance>()?;

    let perf_source = boa_engine::Source::from_bytes(b"new Performance()");
    let perf = context.eval(perf_source)?;
    context.register_global_property(js_string!("performance"), perf, Attribute::all())?;

    Ok(())
}
