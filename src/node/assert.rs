use boa_engine::{Context, Module, Source};

fn assert_js_source() -> String {
    String::from(
        r#"
class AssertionError extends Error {
  constructor(options) {
    if (typeof options === "object" && options !== null) {
      super(options.message || "");
      this.name = "AssertionError";
      this.actual = options.actual;
      this.expected = options.expected;
      this.operator = options.operator || "==";
      if (options.stackStartFn) {
        var tmp = Error.captureStackTrace;
        if (typeof tmp === "function") tmp(this, options.stackStartFn);
      }
    } else {
      super(String(options));
      this.name = "AssertionError";
    }
  }
}

function _fail(msg) {
  throw new AssertionError({
    message: msg || "Failed",
    actual: undefined,
    expected: undefined,
    operator: "fail",
  });
}

function _ok(value, msg) {
  if (!value) {
    throw new AssertionError({
      message: msg || ("The expression evaluated to a falsy value: " + _formatValue(value)),
      actual: value,
      expected: true,
      operator: "==",
    });
  }
}

function _equal(actual, expected, msg) {
  if (actual != expected) {
    throw new AssertionError({
      message: msg || ("" + _formatValue(actual) + " != " + _formatValue(expected)),
      actual: actual,
      expected: expected,
      operator: "==",
    });
  }
}

function _notEqual(actual, expected, msg) {
  if (actual == expected) {
    throw new AssertionError({
      message: msg || ("" + _formatValue(actual) + " == " + _formatValue(expected)),
      actual: actual,
      expected: expected,
      operator: "!=",
    });
  }
}

function _strictEqual(actual, expected, msg) {
  if (actual !== expected) {
    throw new AssertionError({
      message: msg || ("" + _formatValue(actual) + " !== " + _formatValue(expected)),
      actual: actual,
      expected: expected,
      operator: "===",
    });
  }
}

function _notStrictEqual(actual, expected, msg) {
  if (actual === expected) {
    throw new AssertionError({
      message: msg || ("" + _formatValue(actual) + " === " + _formatValue(expected)),
      actual: actual,
      expected: expected,
      operator: "!==",
    });
  }
}

function _deepEqual(actual, expected, msg) {
  if (!_deepEq(actual, expected, false)) {
    throw new AssertionError({
      message: msg || ("" + _formatValue(actual) + " notDeepEqual " + _formatValue(expected)),
      actual: actual,
      expected: expected,
      operator: "deepEqual",
    });
  }
}

function _notDeepEqual(actual, expected, msg) {
  if (_deepEq(actual, expected, false)) {
    throw new AssertionError({
      message: msg || ("" + _formatValue(actual) + " deepEqual " + _formatValue(expected)),
      actual: actual,
      expected: expected,
      operator: "notDeepEqual",
    });
  }
}

function _deepStrictEqual(actual, expected, msg) {
  if (!_deepEq(actual, expected, true)) {
    throw new AssertionError({
      message: msg || ("" + _formatValue(actual) + " notDeepStrictEqual " + _formatValue(expected)),
      actual: actual,
      expected: expected,
      operator: "deepStrictEqual",
    });
  }
}

function _notDeepStrictEqual(actual, expected, msg) {
  if (_deepEq(actual, expected, true)) {
    throw new AssertionError({
      message: msg || ("" + _formatValue(actual) + " deepStrictEqual " + _formatValue(expected)),
      actual: actual,
      expected: expected,
      operator: "notDeepStrictEqual",
    });
  }
}

function _deepEq(a, b, strict) {
  if (a === b) return true;
  if (a === null || b === null) return a === b;
  if (typeof a !== typeof b) return false;
  if (typeof a !== "object") return strict ? a === b : a == b;
  if (Array.isArray(a) && Array.isArray(b)) {
    if (a.length !== b.length) return false;
    for (var i = 0; i < a.length; i++) {
      if (!_deepEq(a[i], b[i], strict)) return false;
    }
    return true;
  }
  var ka = Object.keys(a);
  var kb = Object.keys(b);
  if (ka.length !== kb.length) return false;
  ka.sort();
  kb.sort();
  for (var i = 0; i < ka.length; i++) {
    if (ka[i] !== kb[i]) return false;
    if (!_deepEq(a[ka[i]], b[kb[i]], strict)) return false;
  }
  return true;
}

function _throws(fn, error, msg) {
  if (typeof fn !== "function") {
    throw new TypeError("fn must be a function");
  }
  var thrown = false;
  try {
    fn();
  } catch (e) {
    thrown = true;
    if (error !== undefined) {
      if (typeof error === "function") {
        if (!(e instanceof error)) {
          throw new AssertionError({
            message: msg || ("The error was not an instance of the expected constructor"),
            actual: e,
            expected: error,
            operator: "throws",
          });
        }
      } else if (error instanceof RegExp) {
        if (!error.test(String(e))) {
          throw new AssertionError({
            message: msg || ("The error message did not match the expected pattern"),
            actual: e,
            expected: error,
            operator: "throws",
          });
        }
      } else if (typeof error === "object") {
        for (var key in error) {
          if (_deepEq(e[key], error[key], true)) {
            // skip
          } else {
            throw new AssertionError({
              message: msg || ("The error did not have the expected property " + key),
              actual: e,
              expected: error,
              operator: "throws",
            });
          }
        }
      }
    }
  }
  if (!thrown) {
    throw new AssertionError({
      message: msg || "Missing expected exception",
      actual: undefined,
      expected: error,
      operator: "throws",
    });
  }
}

function _doesNotThrow(fn, msg) {
  if (typeof fn !== "function") {
    throw new TypeError("fn must be a function");
  }
  try {
    fn();
  } catch (e) {
    throw new AssertionError({
      message: msg || ("The function threw an unexpected exception: " + String(e)),
      actual: e,
      expected: undefined,
      operator: "doesNotThrow",
    });
  }
}

function _ifError(value) {
  if (value) {
    throw new AssertionError({
      message: "ifError got unwanted exception: " + String(value),
      actual: value,
      expected: undefined,
      operator: "ifError",
    });
  }
}

function _formatValue(v) {
  if (v === null) return "null";
  if (v === undefined) return "undefined";
  if (typeof v === "string") return JSON.stringify(v);
  if (typeof v === "number" || typeof v === "boolean") return String(v);
  if (v instanceof RegExp) return String(v);
  if (Array.isArray(v)) return "[" + v.map(_formatValue).join(",") + "]";
  try { return JSON.stringify(v); } catch (_) { return String(v); }
}

var _strict = {};
_strict.ok = _ok;
_strict.equal = _strictEqual;
_strict.notEqual = _notStrictEqual;
_strict.strictEqual = _strictEqual;
_strict.notStrictEqual = _notStrictEqual;
_strict.deepEqual = _deepStrictEqual;
_strict.notDeepEqual = _notDeepStrictEqual;
_strict.deepStrictEqual = _deepStrictEqual;
_strict.notDeepStrictEqual = _notDeepStrictEqual;
_strict.throws = _throws;
_strict.doesNotThrow = _doesNotThrow;
_strict.ifError = _ifError;
_strict.fail = _fail;

var _assert = {
  AssertionError: AssertionError,
  ok: _ok,
  equal: _equal,
  notEqual: _notEqual,
  strictEqual: _strictEqual,
  notStrictEqual: _notStrictEqual,
  deepEqual: _deepEqual,
  notDeepEqual: _notDeepEqual,
  deepStrictEqual: _deepStrictEqual,
  notDeepStrictEqual: _notDeepStrictEqual,
  throws: _throws,
  doesNotThrow: _doesNotThrow,
  ifError: _ifError,
  fail: _fail,
  strict: _strict,
};
export {
  AssertionError,
  _ok as ok,
  _equal as equal,
  _notEqual as notEqual,
  _strictEqual as strictEqual,
  _notStrictEqual as notStrictEqual,
  _deepEqual as deepEqual,
  _notDeepEqual as notDeepEqual,
  _deepStrictEqual as deepStrictEqual,
  _notDeepStrictEqual as notDeepStrictEqual,
  _throws as throws,
  _doesNotThrow as doesNotThrow,
  _ifError as ifError,
  _fail as fail,
  _strict as strict,
};
export default _assert;
"#,
    )
}

pub fn create_node_assert_module(context: &mut Context) -> Result<Module, String> {
    let js = assert_js_source();
    let source = Source::from_bytes(js.as_bytes());
    Module::parse(source, None, context).map_err(|e| format!("创建 node:assert 模块失败: {e}"))
}
