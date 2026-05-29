use boa_engine::{Context, Module, Source};

fn util_js_source() -> String {
    String::from(
        r#"
function _promisify(original) {
  if (typeof original !== "function")
    throw new TypeError('"original" argument must be a function');

  function fn(...args) {
    return new Promise((resolve, reject) => {
      try {
        const cb = (err, ...results) => {
          if (err) reject(err);
          else resolve(results.length > 1 ? results : results[0]);
        };
        original.call(this, ...args, cb);
      } catch (e) {
        reject(e);
      }
    });
  }
  Object.setPrototypeOf(fn, Object.getPrototypeOf(original));
  return fn;
}

function _callbackify(original) {
  if (typeof original !== "function")
    throw new TypeError('"original" argument must be a function');

  function fn(...args) {
    const cb = args.pop();
    if (typeof cb !== "function")
      throw new TypeError("last argument must be a callback function");
    try {
      original.call(this, ...args).then(
        (result) => cb(null, result),
        (err) => cb(err),
      );
    } catch (e) {
      cb(e);
    }
  }
  return fn;
}

function _format(fmt, ...args) {
  if (typeof fmt !== "string") {
    return _inspect(fmt);
  }
  let i = 0;
  return fmt.replace(/%[sdifoOj%]/g, (match) => {
    if (match === "%%") return "%";
    const val = args[i++];
    switch (match) {
      case "%s":
        return val === null ? "null" : val === undefined ? "undefined" : String(val);
      case "%d":
        return Number(val || 0).toString();
      case "%i":
        return parseInt(val || 0, 10).toString();
      case "%f":
        return Number(val || 0).toString();
      case "%j":
        try { return JSON.stringify(val); } catch { return "[" + typeof val + "]"; }
      case "%o":
      case "%O":
        return _inspect(val, { depth: Infinity, colors: false });
      default:
        return match;
    }
  });
}

function _formatValue(val, depth, seen, colors, indent) {
  indent = indent || "";
  const nextIndent = indent + "  ";

  if (val === null) return "null";
  if (val === undefined) return "undefined";
  if (typeof val === "boolean") return colors ? "\x1b[33m" + val + "\x1b[0m" : String(val);
  if (typeof val === "number") return colors ? "\x1b[33m" + val + "\x1b[0m" : String(val);
  if (typeof val === "string") {
    const escaped = val.replace(/\\/g, "\\\\").replace(/'/g, "\\'").replace(/\n/g, "\\n").replace(/\r/g, "\\r").replace(/\t/g, "\\t");
    return colors ? "\x1b[32m'" + escaped + "'\x1b[0m" : "'" + escaped + "'";
  }
  if (typeof val === "function") {
    const name = val.name || "(anonymous)";
    return colors ? "\x1b[36m[Function: " + name + "]\x1b[0m" : "[Function: " + name + "]";
  }
  if (typeof val === "symbol") return val.toString();
  if (typeof val === "bigint") return val.toString() + "n";

  if (seen.has(val)) return "[Circular]";
  if (depth <= 0) return "[" + (val.constructor ? val.constructor.name : "Object") + "]";

  seen.add(val);

  try {
    if (Array.isArray(val)) {
      if (val.length === 0) return "[]";
      let items = [];
      for (let i = 0; i < val.length; i++) {
        items.push(nextIndent + _formatValue(val[i], depth - 1, seen, colors, nextIndent));
      }
      seen.delete(val);
      return "[\n" + items.join(",\n") + "\n" + indent + "]";
    }

    if (val instanceof Date) {
      seen.delete(val);
      return colors ? "\x1b[35m" + val.toISOString() + "\x1b[0m" : val.toISOString();
    }
    if (val instanceof RegExp) {
      seen.delete(val);
      return colors ? "\x1b[31m" + val.toString() + "\x1b[0m" : val.toString();
    }
    if (val instanceof Map) {
      if (val.size === 0) return "Map(0) {}";
      let items = [];
      for (const [k, v] of val) {
        items.push(nextIndent + _formatValue(k, depth - 1, seen, colors, nextIndent) + " => " + _formatValue(v, depth - 1, seen, colors, nextIndent));
      }
      seen.delete(val);
      return "Map(" + val.size + ") {\n" + items.join(",\n") + "\n" + indent + "}";
    }
    if (val instanceof Set) {
      if (val.size === 0) return "Set(0) {}";
      let items = [];
      for (const v of val) {
        items.push(nextIndent + _formatValue(v, depth - 1, seen, colors, nextIndent));
      }
      seen.delete(val);
      return "Set(" + val.size + ") {\n" + items.join(",\n") + "\n" + indent + "}";
    }
    if (val instanceof Error) {
      seen.delete(val);
      return val.stack || val.message || String(val);
    }

    const keys = Object.keys(val);
    if (keys.length === 0) {
      seen.delete(val);
      const ctor = val.constructor && val.constructor.name !== "Object" ? val.constructor.name : "";
      return ctor ? ctor + " {}" : "{}";
    }

    let items = [];
    for (const key of keys) {
      const formattedKey = colors ? "\x1b[34m" + key + "\x1b[0m" : key;
      items.push(nextIndent + formattedKey + ": " + _formatValue(val[key], depth - 1, seen, colors, nextIndent));
    }
    const ctor = val.constructor && val.constructor.name !== "Object" ? val.constructor.name : "";
    seen.delete(val);
    return (ctor || "") + "{\n" + items.join(",\n") + "\n" + indent + "}";
  } catch (e) {
    seen.delete(val);
    return "[" + typeof val + "]";
  }
}

function _inspect(obj, opts) {
  opts = opts || {};
  const depth = opts.depth !== undefined ? opts.depth : 2;
  const colors = !!opts.colors;
  return _formatValue(obj, depth, new Set(), colors, "");
}

function _deprecate(fn, msg) {
  if (typeof fn !== "function") throw new TypeError('"fn" must be a function');
  let warned = false;
  return function (...args) {
    if (!warned) {
      warned = true;
      if (typeof process !== "undefined" && process.emitWarning) {
        process.emitWarning(msg, "DeprecationWarning");
      } else {
        console.warn("DeprecationWarning:", msg);
      }
    }
    return fn.apply(this, args);
  };
}

function _inherits(ctor, superCtor) {
  if (typeof ctor !== "function") throw new TypeError('"ctor" must be a function');
  if (typeof superCtor !== "function") throw new TypeError('"superCtor" must be a function');
  Object.setPrototypeOf(ctor.prototype, superCtor.prototype);
  ctor.super_ = superCtor;
}

function _debuglog(section) {
  const NODE_DEBUG = typeof process !== "undefined" && process.env ? process.env.NODE_DEBUG || "" : "";
  const sections = NODE_DEBUG.split(",").map((s) => s.trim());
  const enabled = sections.includes(section) || sections.includes("*");

  return enabled
    ? function (...args) {
        const prefix = typeof process !== "undefined" && process.pid ? section + " " + process.pid : section;
        console.error(prefix + ":", _format(...args));
      }
    : function () {};
}

var _types = {
  isDate: function (v) { return v instanceof Date; },
  isRegExp: function (v) { return v instanceof RegExp; },
  isArrayBuffer: function (v) { return v instanceof ArrayBuffer; },
  isMap: function (v) { return v instanceof Map; },
  isSet: function (v) { return v instanceof Set; },
  isWeakMap: function (v) { return v instanceof WeakMap; },
  isWeakSet: function (v) { return v instanceof WeakSet; },
  isPromise: function (v) { return v instanceof Promise; },
  isBooleanObject: function (v) { return v instanceof Boolean; },
  isNumberObject: function (v) { return v instanceof Number; },
  isStringObject: function (v) { return v instanceof String; },
  isSymbolObject: function (v) { return v instanceof Symbol; },
  isNativeError: function (v) { return v instanceof Error; },
  isTypedArray: function (v) { return ArrayBuffer.isView(v) && !(v instanceof DataView); },
  isUint8Array: function (v) { return v instanceof Uint8Array; },
  isUint8ClampedArray: function (v) { return v instanceof Uint8ClampedArray; },
  isUint16Array: function (v) { return v instanceof Uint16Array; },
  isUint32Array: function (v) { return v instanceof Uint32Array; },
  isInt8Array: function (v) { return v instanceof Int8Array; },
  isInt16Array: function (v) { return v instanceof Int16Array; },
  isInt32Array: function (v) { return v instanceof Int32Array; },
  isFloat32Array: function (v) { return v instanceof Float32Array; },
  isFloat64Array: function (v) { return v instanceof Float64Array; },
  isBigInt64Array: function (v) { return v instanceof BigInt64Array; },
  isBigUint64Array: function (v) { return v instanceof BigUint64Array; },
  isDataView: function (v) { return v instanceof DataView; },
};

var _util = {
  promisify: _promisify,
  callbackify: _callbackify,
  format: _format,
  inspect: _inspect,
  deprecate: _deprecate,
  inherits: _inherits,
  debuglog: _debuglog,
  types: _types,
};

export { _promisify as promisify };
export { _callbackify as callbackify };
export { _format as format };
export { _inspect as inspect };
export { _deprecate as deprecate };
export { _inherits as inherits };
export { _debuglog as debuglog };
export { _types as types };
export default _util;
"#,
    )
}

pub fn create_node_util_module(context: &mut Context) -> Result<Module, String> {
    let js = util_js_source();
    let source = Source::from_bytes(js.as_bytes());
    Module::parse(source, None, context).map_err(|e| format!("创建 node:util 模块失败: {e}"))
}
