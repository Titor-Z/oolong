use std::sync::OnceLock;
use std::time::Instant;

use boa_engine::{
    Context, JsResult, JsValue, Module, NativeFunction, Source,
    js_string, object::FunctionObjectBuilder,
};

fn start_time() -> &'static Instant {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now)
}

fn make_fn<F>(f: F, name: &str, len: usize, ctx: &mut Context) -> JsValue
where
    F: Fn(&JsValue, &[JsValue], &mut Context) -> JsResult<JsValue> + 'static,
{
    let native = unsafe { NativeFunction::from_closure(f) };
    FunctionObjectBuilder::new(ctx.realm(), native)
        .name(js_string!(name))
        .length(len)
        .build()
        .into()
}

fn register_perf_hooks_native(ctx: &mut Context) -> Result<(), String> {
    let now_fn = make_fn(
        |_: &JsValue, _args: &[JsValue], _ctx: &mut Context| -> JsResult<JsValue> {
            let elapsed = start_time().elapsed();
            let ms = elapsed.as_secs_f64() * 1000.0;
            Ok(JsValue::from(ms))
        },
        "_perfNow",
        0,
        ctx,
    );

    let _ = ctx.register_global_property(
        js_string!("_perfNow"),
        now_fn,
        boa_engine::property::Attribute::all(),
    );

    Ok(())
}

fn perf_hooks_js_source() -> String {
    String::from(
        r#"
var _timeOrigin = Date.now();

function _now() {
  return typeof globalThis._perfNow === "function" ? globalThis._perfNow() : 0;
}

class _PerformanceEntry {
  constructor(name, entryType, startTime, duration) {
    this.name = name;
    this.entryType = entryType;
    this.startTime = startTime;
    this.duration = duration;
  }
  toJSON() {
    return { name: this.name, entryType: this.entryType, startTime: this.startTime, duration: this.duration };
  }
}

class _PerformanceMark extends _PerformanceEntry {
  constructor(name, options) {
    var startTime = (options && options.startTime) || _now();
    super(name, "mark", startTime, 0);
  }
}

class _PerformanceMeasure extends _PerformanceEntry {
  constructor(name, startTime, duration) {
    super(name, "measure", startTime, duration);
  }
}

var _performance = {
  now: _now,
  timeOrigin: _timeOrigin,
  mark: function(name) { return new _PerformanceMark(name); },
  measure: function(name, startMark, endMark) {
    var start = startMark ? (typeof startMark === "string" ? _now() : startMark) : 0;
    var end = endMark ? (typeof endMark === "string" ? _now() : endMark) : _now();
    return new _PerformanceMeasure(name, start, end - start);
  },
  clearMarks: function() {},
  clearMeasures: function() {},
  getEntries: function() { return []; },
  getEntriesByName: function() { return []; },
  getEntriesByType: function() { return []; },
};

var _perf_hooks = {
  performance: _performance,
  PerformanceEntry: _PerformanceEntry,
  PerformanceMark: _PerformanceMark,
  PerformanceMeasure: _PerformanceMeasure,
  constants: {
    NODE_PERFORMANCE_GC_MAJOR: 2,
    NODE_PERFORMANCE_GC_MINOR: 1,
    NODE_PERFORMANCE_GC_INCREMENTAL: 4,
    NODE_PERFORMANCE_GC_WEAKCB: 8,
  },
  monitorEventLoopDelay: function() {
    return { enable: function() {}, disable: function() {}, percentile: function() { return 0; } };
  },
};
export { _performance as performance, _PerformanceEntry as PerformanceEntry,
  _PerformanceMark as PerformanceMark, _PerformanceMeasure as PerformanceMeasure };
export default _perf_hooks;
"#,
    )
}

pub fn create_node_perf_hooks_module(context: &mut Context) -> Result<Module, String> {
    register_perf_hooks_native(context)?;
    let js = perf_hooks_js_source();
    let source = Source::from_bytes(js.as_bytes());
    Module::parse(source, None, context).map_err(|e| format!("创建 node:perf_hooks 模块失败: {e}"))
}
