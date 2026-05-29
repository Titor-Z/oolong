use boa_engine::{Context, Module, Source};

fn vm_js_source() -> String {
    String::from(
        r#"
function _runInThisContext(code, options) {
  if (typeof code !== "string") throw new TypeError("code must be a string");
  return eval(code);
}

function _runInNewContext(code, sandbox, options) {
  if (typeof code !== "string") throw new TypeError("code must be a string");
  sandbox = sandbox || {};
  var keys = Object.keys(sandbox);
  var vals = keys.map(function(k) { return sandbox[k]; });
  var body = '"use strict"; return (' + code + ')';
  var fn = new Function(keys.join(","), body);
  try {
    return fn.apply(undefined, vals);
  } catch (e) {
    throw e;
  }
}

function _compileFunction(code, params, options) {
  if (typeof code !== "string") throw new TypeError("code must be a string");
  params = params || [];
  var fn = new Function(params.join(","), code);
  return fn;
}

function _Script(code, options) {
  if (typeof code !== "string") throw new TypeError("code must be a string");
  this._code = code;
  this._filename = (options && options.filename) || "";
}
_Script.prototype.runInThisContext = function(options) {
  return _runInThisContext(this._code, options || {});
};
_Script.prototype.runInNewContext = function(sandbox, options) {
  return _runInNewContext(this._code, sandbox, options || {});
};

var _vm = {
  runInThisContext: _runInThisContext,
  runInNewContext: _runInNewContext,
  compileFunction: _compileFunction,
  Script: _Script,
};
export { _runInThisContext as runInThisContext, _runInNewContext as runInNewContext,
  _compileFunction as compileFunction, _Script as Script };
export default _vm;
"#,
    )
}

pub fn create_node_vm_module(context: &mut Context) -> Result<Module, String> {
    let js = vm_js_source();
    let source = Source::from_bytes(js.as_bytes());
    Module::parse(source, None, context).map_err(|e| format!("创建 node:vm 模块失败: {e}"))
}
