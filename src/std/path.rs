use boa_engine::{Context, Module, Source};

/// 系统路径分隔符
fn sys_sep() -> &'static str {
    if cfg!(windows) { "\\" } else { "/" }
}

/// 系统路径定界符（PATH 环境变量分隔符）
fn sys_delimiter() -> &'static str {
    if cfg!(windows) { ";" } else { ":" }
}

/// 生成 path 模块的 JS 源码，平台特性在运行时注入
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
const _cwd = "{cwd_escaped}";
function _isSep(ch) {{
  return ch === "/" || ch === "\\";
}}

function _mustStr(v) {{
  if (typeof v !== "string") throw new TypeError("path: argument must be a string");
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

export const sep = "{sep_escaped}";
export const delimiter = "{delim_escaped}";

export function join(...parts) {{
  for (const p of parts) _mustStr(p, "join");
  return _norm(parts.join(_sep));
}}

export function dirname(p) {{
  _mustStr(p, "dirname");
  const n = _norm(p);
  if (n === _sep) return _sep;
  const i = n.lastIndexOf(_sep);
  if (i === -1) return ".";
  if (i === 0) return _sep;
  return n.slice(0, i);
}}

export function basename(p, ext) {{
  _mustStr(p, "basename");
  const n = _norm(p);
  const i = n.lastIndexOf(_sep);
  const base = i === -1 ? n : n.slice(i + 1);
  if (ext !== undefined) {{
    _mustStr(ext, "basename");
    if (base.endsWith(ext)) return base.slice(0, -ext.length);
  }}
  return base;
}}

export function extname(p) {{
  _mustStr(p, "extname");
  const b = basename(p);
  const i = b.lastIndexOf(".");
  if (i <= 0) return "";
  return b.slice(i);
}}

export function isAbsolute(p) {{
  _mustStr(p, "isAbsolute");
  return p.length > 0 && _isSep(p[0]);
}}

export function normalize(p) {{
  _mustStr(p, "normalize");
  return _norm(p);
}}

export function relative(from, to) {{
  _mustStr(from, "relative");
  _mustStr(to, "relative");
  const f = _norm(from).split(_sep).filter(Boolean);
  const t = _norm(to).split(_sep).filter(Boolean);
  let i = 0;
  while (i < f.length && i < t.length && f[i] === t[i]) i++;
  const up = f.slice(i).map(() => "..");
  const down = t.slice(i);
  const r = up.concat(down).join(_sep);
  return r || ".";
}}

export function resolve(...parts) {{
  for (const p of parts) _mustStr(p, "resolve");
  let abs = false, i = 0;
  for (; i < parts.length; i++) {{
    if (parts[i].length > 0 && _isSep(parts[i][0])) {{ abs = true; break; }}
  }}
  const base = abs ? "" : (_cwd + _sep);
  return _norm((base + parts.slice(i).join(_sep)).split(/[/\\]+/).join(_sep));
}}

export function parse(p) {{
  _mustStr(p);
  const n = _norm(p);
  if (n === "" || n === ".") return {{ root: "", dir: ".", base: n, ext: "", name: n }};
  const abs = n.length > 0 && _isSep(n[0]);
  const root = abs ? _sep : "";
  const lastSep = n.lastIndexOf(_sep);
  const base = lastSep === -1 ? n : n.slice(lastSep + 1);
  const extI = base.lastIndexOf(".");
  const ext = extI > 0 ? base.slice(extI) : "";
  const name = ext ? base.slice(0, extI) : base;
  const dir = lastSep === -1 ? "." : (lastSep === 0 ? _sep : n.slice(0, lastSep));
  return {{ root, dir, base, ext, name }};
}}

export function format(obj) {{
  if (obj === null || typeof obj !== "object") throw new TypeError("path.format: argument must be an object");
  const dir = obj.dir !== undefined ? obj.dir : "";
  const base = obj.base !== undefined ? obj.base : (obj.name !== undefined ? obj.name + (obj.ext || "") : "");
  if (!dir) return base;
  if (dir.endsWith(_sep)) return dir + base;
  return dir + _sep + base;
}}

const _path = {{
  sep, delimiter, join, dirname, basename, extname, isAbsolute, normalize, relative, resolve, parse, format
}};
export default _path;
"#,
        cwd_escaped = cwd_escaped,
        sep_escaped = sep_escaped,
        delim_escaped = delim_escaped,
    )
}

/// 创建 "path" 内置模块
pub fn create_path_module(context: &mut Context) -> Result<Module, String> {
    let js = path_js_source();
    let source = Source::from_bytes(js.as_bytes());
    Module::parse(source, None, context).map_err(|e| format!("创建 path 模块失败: {e}"))
}
