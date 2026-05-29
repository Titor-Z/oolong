use boa_engine::{Context, Module, Source};

fn perf_hooks_js_source() -> String {
    String::from(
        r#"
var _performance = globalThis.performance || { now: function() { return 0; }, timeOrigin: Date.now() };
var _PerformanceEntry = globalThis.PerformanceEntry;
var _PerformanceMark = globalThis.PerformanceMark;
var _PerformanceMeasure = globalThis.PerformanceMeasure;

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
    let js = perf_hooks_js_source();
    let source = Source::from_bytes(js.as_bytes());
    Module::parse(source, None, context).map_err(|e| format!("创建 node:perf_hooks 模块失败: {e}"))
}
