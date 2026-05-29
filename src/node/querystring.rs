use boa_engine::{Context, Module, Source};

fn querystring_js_source() -> String {
    String::from(
        r#"
function _escape(str) {
  return encodeURIComponent(typeof str === "string" ? str : String(str))
    .replace(/%20/g, "+");
}

function _unescape(str) {
  return decodeURIComponent(String(str).replace(/\+/g, " "));
}

function _parse(qs, sep, eq, options) {
  sep = sep || "&";
  eq = eq || "=";
  var maxKeys = 1000;
  if (options && typeof options.maxKeys === "number")
    maxKeys = options.maxKeys;

  if (typeof qs !== "string" || qs.length === 0)
    return {};

  var obj = Object.create(null);
  var parts = qs.split(sep);
  var limit = Math.min(parts.length, maxKeys);

  for (var i = 0; i < limit; i++) {
    var part = parts[i];
    if (part.length === 0) continue;
    var idx = part.indexOf(eq);
    var key, val;
    if (idx >= 0) {
      key = _unescape(part.slice(0, idx));
      val = _unescape(part.slice(idx + 1));
    } else {
      key = _unescape(part);
      val = "";
    }
    if (obj[key] === undefined) {
      obj[key] = val;
    } else if (Array.isArray(obj[key])) {
      obj[key].push(val);
    } else {
      obj[key] = [obj[key], val];
    }
  }
  return obj;
}

function _stringify(obj, sep, eq, options) {
  sep = sep || "&";
  eq = eq || "=";
  if (obj === null || obj === undefined) return "";

  var keys = Object.keys(obj);
  var result = [];
  for (var i = 0; i < keys.length; i++) {
    var key = _escape(keys[i]);
    var val = obj[keys[i]];
    if (Array.isArray(val)) {
      for (var j = 0; j < val.length; j++) {
        result.push(key + eq + _escape(_stringifyVal(val[j])));
      }
    } else {
      result.push(key + eq + _escape(_stringifyVal(val)));
    }
  }
  return result.join(sep);
}

function _stringifyVal(v) {
  if (v === null || v === undefined) return "";
  if (typeof v === "string") return v;
  return String(v);
}

var _qs = {
  parse: _parse,
  stringify: _stringify,
  escape: _escape,
  unescape: _unescape,
  decode: _parse,
  encode: _stringify,
};
export { _parse as parse, _stringify as stringify, _escape as escape, _unescape as unescape, _parse as decode, _stringify as encode };
export default _qs;
"#,
    )
}

pub fn create_node_querystring_module(context: &mut Context) -> Result<Module, String> {
    let js = querystring_js_source();
    let source = Source::from_bytes(js.as_bytes());
    Module::parse(source, None, context).map_err(|e| format!("创建 node:querystring 模块失败: {e}"))
}
