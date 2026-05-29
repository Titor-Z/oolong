use boa_engine::{Context, Module, Source};

fn timers_js_source() -> String {
    String::from(
        r#"
var _timers = {
  setTimeout: globalThis.setTimeout,
  clearTimeout: globalThis.clearTimeout,
  setInterval: globalThis.setInterval,
  clearInterval: globalThis.clearInterval,
  setImmediate: globalThis.setImmediate,
  clearImmediate: globalThis.clearImmediate,
  promises: {
    setTimeout: function(delay, value) {
      return new Promise(function(resolve) {
        globalThis.setTimeout(function() { resolve(value); }, delay);
      });
    },
    setImmediate: function(value) {
      return new Promise(function(resolve) {
        globalThis.setImmediate(function() { resolve(value); });
      });
    },
  },
};
var _timeout = globalThis.setTimeout;
var _interval = globalThis.setInterval;
var _immediate = globalThis.setImmediate;
var _clearTimeout = globalThis.clearTimeout;
var _clearInterval = globalThis.clearInterval;
var _clearImmediate = globalThis.clearImmediate;

export { _timeout as setTimeout, _clearTimeout as clearTimeout, _interval as setInterval, _clearInterval as clearInterval, _immediate as setImmediate, _clearImmediate as clearImmediate };
export default _timers;
"#,
    )
}

pub fn create_node_timers_module(context: &mut Context) -> Result<Module, String> {
    let js = timers_js_source();
    let source = Source::from_bytes(js.as_bytes());
    Module::parse(source, None, context).map_err(|e| format!("创建 node:timers 模块失败: {e}"))
}
