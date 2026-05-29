use boa_engine::{Context, Module, Source};

fn sys_sep() -> &'static str {
    if cfg!(windows) { "\\" } else { "/" }
}

fn sys_delimiter() -> &'static str {
    if cfg!(windows) { ";" } else { ":" }
}

fn path_js_source() -> String {
    let cwd = std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "/".to_string());
    let sep = sys_sep();
    let delim = sys_delimiter();

    let sep_escaped = if sep == "\\" { "\\\\" } else { sep };
    let delim_escaped = if delim == ";" { ";" } else { delim };
    let cwd_escaped = cwd.replace('\\', "\\\\");

    format!(
        r#"const _sep = "{sep_escaped}";
const _delim = "{delim_escaped}";
const _cwd = "{cwd_escaped}";

function _isSep(ch) {{
  return ch === "/" || ch === "\\";
}}

function _mustStr(v, name) {{
  if (typeof v !== "string") throw new TypeError("path." + name + ": argument must be a string");
}}

function _norm(p) {{
  _mustStr(p, "normalize");
  let s = p;
  while (s.length > 0 && _isSep(s[s.length - 1])) s = s.slice(0, -1);
  const stack = [];
  const abs = s.length > 0 && _isSep(s[0]);
  for (const seg of s.split(/[/\\]+/)) {{
    if (seg === "" || seg === ".") continue;
    if (seg === "..") {{
      if (stack.length > 0 && stack[stack.length - 1] !== "..") {{
        stack.pop();
      }} else if (!abs) {{
        stack.push("..");
      }}
    }} else {{
      stack.push(seg);
    }}
  }}
  return (abs ? _sep : "") + stack.join(_sep);
}}

function _makePath(sep, delim) {{
  function _join(...parts) {{
    for (const p of parts) _mustStr(p, "join");
    return _norm(parts.join(sep));
  }}
  function _dirname(p) {{
    _mustStr(p, "dirname");
    const n = _norm(p);
    if (n === sep) return sep;
    const i = n.lastIndexOf(sep);
    if (i === -1) return ".";
    if (i === 0) return sep;
    return n.slice(0, i);
  }}
  function _basename(p, ext) {{
    _mustStr(p, "basename");
    const n = _norm(p);
    const i = n.lastIndexOf(sep);
    const base = i === -1 ? n : n.slice(i + 1);
    if (ext !== undefined) {{
      _mustStr(ext, "basename");
      if (base.endsWith(ext)) return base.slice(0, -ext.length);
    }}
    return base;
  }}
  function _extname(p) {{
    _mustStr(p, "extname");
    const b = _basename(p);
    const i = b.lastIndexOf(".");
    if (i <= 0) return "";
    return b.slice(i);
  }}
  function _isAbsolute(p) {{
    _mustStr(p, "isAbsolute");
    return p.length > 0 && _isSep(p[0]);
  }}
  function _normalize(p) {{
    _mustStr(p, "normalize");
    return _norm(p);
  }}
  function _relative(from, to) {{
    _mustStr(from, "relative");
    _mustStr(to, "relative");
    const f = _norm(from).split(sep).filter(Boolean);
    const t = _norm(to).split(sep).filter(Boolean);
    let i = 0;
    while (i < f.length && i < t.length && f[i] === t[i]) i++;
    const up = f.slice(i).map(() => "..");
    const down = t.slice(i);
    const r = up.concat(down).join(sep);
    return r || ".";
  }}
  function _resolve(...parts) {{
    for (const p of parts) _mustStr(p, "resolve");
    let abs = false, i = 0;
    for (; i < parts.length; i++) {{
      if (parts[i].length > 0 && _isSep(parts[i][0])) {{ abs = true; break; }}
    }}
    const base = abs ? "" : (_cwd + sep);
    return _norm((base + parts.slice(i).join(sep)).split(/[/\\]+/).join(sep));
  }}
  function _parse(p) {{
    _mustStr(p, "parse");
    const n = _norm(p);
    if (n === "" || n === ".") return {{ root: "", dir: ".", base: n, ext: "", name: n }};
    const abs = n.length > 0 && _isSep(n[0]);
    const root = abs ? sep : "";
    const lastSep = n.lastIndexOf(sep);
    const base = lastSep === -1 ? n : n.slice(lastSep + 1);
    const extI = base.lastIndexOf(".");
    const ext = extI > 0 ? base.slice(extI) : "";
    const name = ext ? base.slice(0, extI) : base;
    const dir = lastSep === -1 ? "." : (lastSep === 0 ? sep : n.slice(0, lastSep));
    return {{ root, dir, base, ext, name }};
  }}
  function _format(obj) {{
    if (obj === null || typeof obj !== "object") throw new TypeError("path.format: argument must be an object");
    const dir = obj.dir !== undefined ? obj.dir : "";
    const base = obj.base !== undefined ? obj.base : (obj.name !== undefined ? obj.name + (obj.ext || "") : "");
    if (!dir) return base;
    if (dir.endsWith(sep)) return dir + base;
    return dir + sep + base;
  }}
  function _toNamespacedPath(p) {{
    _mustStr(p, "toNamespacedPath");
    return p;
  }}
  return {{
    sep,
    delimiter: delim,
    join: _join,
    dirname: _dirname,
    basename: _basename,
    extname: _extname,
    isAbsolute: _isAbsolute,
    normalize: _normalize,
    relative: _relative,
    resolve: _resolve,
    parse: _parse,
    format: _format,
    toNamespacedPath: _toNamespacedPath,
  }};
}}

const _path = _makePath(_sep, _delim);
_path.posix = _makePath("/", ":");
_path.win32 = _makePath("\\", ";");

export const sep = _sep;
export const delimiter = _delim;
export const join = _path.join;
export const dirname = _path.dirname;
export const basename = _path.basename;
export const extname = _path.extname;
export const isAbsolute = _path.isAbsolute;
export const normalize = _path.normalize;
export const relative = _path.relative;
export const resolve = _path.resolve;
export const parse = _path.parse;
export const format = _path.format;
export const toNamespacedPath = _path.toNamespacedPath;
export const posix = _path.posix;
export const win32 = _path.win32;
export default _path;
"#,
        cwd_escaped = cwd_escaped,
        sep_escaped = sep_escaped,
        delim_escaped = delim_escaped,
    )
}

pub fn create_node_path_module(context: &mut Context) -> Result<Module, String> {
    let js = path_js_source();
    let source = Source::from_bytes(js.as_bytes());
    Module::parse(source, None, context).map_err(|e| format!("创建 node:path 模块失败: {e}"))
}
