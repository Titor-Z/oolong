use boa_engine::object::builtins::{JsArray, JsFunction};
use boa_engine::{
  Context, JsData, JsObject, JsResult, JsString, JsValue, boa_class, js_error, js_string,
};
use boa_gc::{Finalize, Trace};
use std::fmt::Write;

/// JavaScript `URLSearchParams` 类
#[derive(Debug, Clone, JsData, Trace, Finalize)]
pub struct JsUrlSearchParams {
  params: Vec<(String, String)>,
}

#[boa_class(rename = "URLSearchParams")]
impl JsUrlSearchParams {
  /// `new URLSearchParams(init?)`
  #[boa(constructor)]
  pub fn constructor(init: Option<JsValue>) -> JsResult<Self> {
    let params = match init {
      None => Vec::new(),
      Some(val) => {
        if let Ok(s) = val.to_string(&mut Context::default()) {
          let raw = s.to_std_string_escaped();
          let query = raw.strip_prefix('?').unwrap_or(&raw);
          parse_query_string(query)
        } else       if let Some(obj) = val.as_object() {
          parse_object_init(&obj)?
        } else {
          return Err(js_error!(TypeError: "URLSearchParams constructor: invalid init"));
        }
      }
    };
    Ok(Self { params })
  }

  /// `params.append(name, value)`
  pub fn append(&mut self, name: JsString, value: JsString) {
    self
      .params
      .push((name.to_std_string_escaped(), value.to_std_string_escaped()));
  }

  /// `params.delete(name)`
  pub fn delete(&mut self, name: JsString) {
    let key = name.to_std_string_escaped();
    self.params.retain(|(k, _)| k != &key);
  }

  /// `params.get(name)` → string | null
  pub fn get(&self, name: JsString) -> Option<JsString> {
    let key = name.to_std_string_escaped();
    self
      .params
      .iter()
      .find(|(k, _)| k == &key)
      .map(|(_, v)| JsString::from(v.as_str()))
  }

  /// `params.getAll(name)` → string[]
  pub fn get_all(&self, name: JsString) -> Vec<JsString> {
    let key = name.to_std_string_escaped();
    self
      .params
      .iter()
      .filter(|(k, _)| k == &key)
      .map(|(_, v)| JsString::from(v.as_str()))
      .collect()
  }

  /// `params.has(name)` → boolean
  pub fn has(&self, name: JsString) -> bool {
    let key = name.to_std_string_escaped();
    self.params.iter().any(|(k, _)| k == &key)
  }

  /// `params.set(name, value)`
  pub fn set(&mut self, name: JsString, value: JsString) {
    let key = name.to_std_string_escaped();
    let val = value.to_std_string_escaped();
    let mut found = false;
    for (k, v) in self.params.iter_mut() {
      if k == &key {
        v.clone_from(&val);
        found = true;
        break;
      }
    }
    if !found {
      self.params.push((key, val));
    }
  }

  /// `params.sort()`
  pub fn sort(&mut self) {
    self.params.sort_by(|a, b| a.0.cmp(&b.0));
  }

  /// `params.toString()` → string
  pub fn to_string(&self) -> JsString {
    let mut s = String::new();
    for (i, (key, val)) in self.params.iter().enumerate() {
      if i > 0 {
        s.push('&');
      }
      let _ = write!(s, "{}={}", url_encode(key), url_encode(val));
    }
    JsString::from(s)
  }

  /// `params.forEach(callback)`
  pub fn for_each(&self, callback: JsFunction, context: &mut Context) -> JsResult<()> {
    for (key, val) in &self.params {
      let this = JsValue::undefined();
      let args = [
        JsValue::from(JsString::from(val.as_str())),
        JsValue::from(JsString::from(key.as_str())),
        JsValue::from(self.to_string()),
      ];
      callback.call(&this, &args, context)?;
    }
    Ok(())
  }

  /// `params.entries()` → 键值对数组
  pub fn entries(&self, context: &mut Context) -> JsResult<JsValue> {
    let arr = JsArray::new(context);
    for (key, val) in &self.params {
      let pair = JsArray::new(context);
      let _ = pair.push(JsString::from(key.as_str()), context);
      let _ = pair.push(JsString::from(val.as_str()), context);
      let _ = arr.push(pair, context);
    }
    Ok(JsValue::from(arr))
  }

  /// `params.keys()` → key 数组
  pub fn keys(&self, context: &mut Context) -> JsResult<JsValue> {
    let arr = JsArray::new(context);
    for (key, _) in &self.params {
      let _ = arr.push(JsString::from(key.as_str()), context);
    }
    Ok(JsValue::from(arr))
  }

  /// `params.values()` → value 数组
  pub fn values(&self, context: &mut Context) -> JsResult<JsValue> {
    let arr = JsArray::new(context);
    for (_, val) in &self.params {
      let _ = arr.push(JsString::from(val.as_str()), context);
    }
    Ok(JsValue::from(arr))
  }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn parse_query_string(query: &str) -> Vec<(String, String)> {
  let mut params = Vec::new();
  for pair in query.split('&') {
    if pair.is_empty() {
      continue;
    }
    if let Some(eq_pos) = pair.find('=') {
      params.push((url_decode(&pair[..eq_pos]), url_decode(&pair[eq_pos + 1..])));
    } else {
      params.push((url_decode(pair), String::new()));
    }
  }
  params
}

fn parse_object_init(obj: &JsObject) -> JsResult<Vec<(String, String)>> {
  if let Ok(sp) = obj.clone().downcast::<JsUrlSearchParams>() {
    let sp_ref = sp.borrow();
    return Ok(sp_ref.data().params.clone());
  }

  let mut ctx = Context::default();
  let len = obj
    .get(js_string!("length"), &mut ctx)
    .and_then(|v| v.to_length(&mut ctx))
    .unwrap_or(0);

  if len > 0 {
    let mut params = Vec::new();
    for i in 0..len {
      let entry = obj
        .get(i, &mut ctx)
        .map_err(|_| js_error!(TypeError: "Cannot read entry at index {i}"))?;
      if let Some(entry_obj) = entry.as_object() {
        let k = entry_obj
          .get(0, &mut ctx)
          .and_then(|v| v.to_string(&mut ctx))
          .map(|s| s.to_std_string_escaped())
          .unwrap_or_default();
        let v = entry_obj
          .get(1, &mut ctx)
          .and_then(|v| v.to_string(&mut ctx))
          .map(|s| s.to_std_string_escaped())
          .unwrap_or_default();
        params.push((k, v));
      }
    }
    return Ok(params);
  }

  Err(js_error!(TypeError: "URLSearchParams constructor: invalid init"))
}

fn url_encode(s: &str) -> String {
  let mut out = String::with_capacity(s.len());
  for byte in s.bytes() {
    match byte {
      b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(byte as char),
      b' ' => out.push('+'),
      _ => {
        let _ = write!(out, "%{byte:02X}");
      }
    }
  }
  out
}

fn url_decode(s: &str) -> String {
  let mut out = String::with_capacity(s.len());
  let mut chars = s.chars();
  while let Some(c) = chars.next() {
    if c == '%' {
      let hex: String = chars.by_ref().take(2).collect();
      if hex.len() == 2 && let Ok(byte) = u8::from_str_radix(&hex, 16) {
        out.push(byte as char);
        continue;
      }
      out.push('%');
      out.push_str(&hex);
    } else if c == '+' {
      out.push(' ');
    } else {
      out.push(c);
    }
  }
  out
}

// ── Registration ──────────────────────────────────────────────────────────────

pub fn register_globals(context: &mut Context) -> JsResult<()> {
  context.register_global_class::<JsUrlSearchParams>()?;
  Ok(())
}
