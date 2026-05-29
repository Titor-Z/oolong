use boa_engine::object::builtins::JsArrayBuffer;
use boa_engine::{
  boa_class, js_string,
  module::SyntheticModuleInitializer,
  Context, Finalize, JsData, JsError, JsObject, JsResult, JsString, JsValue,
  Module, Trace,
};
use boa_gc::GcRefCell;

// ── Hex helpers ─────────────────────────────────────────────────────────────

fn hex_encode(data: &[u8]) -> String {
  let mut s = String::with_capacity(data.len() * 2);
  for b in data {
    s.push_str(&format!("{:02x}", b));
  }
  s
}

fn hex_decode(s: &str) -> Vec<u8> {
  let s = s.trim();
  let mut data = Vec::with_capacity(s.len() / 2);
  let chars: Vec<char> = s.chars().collect();
  for i in (0..chars.len()).step_by(2) {
    if i + 1 < chars.len() {
      if let Ok(b) = u8::from_str_radix(&format!("{}{}", chars[i], chars[i + 1]), 16) {
        data.push(b);
      }
    }
  }
  data
}

// ── Buffer class ────────────────────────────────────────────────────────────

/// JavaScript `Buffer` 类 — Node.js 兼容
///
/// 内部使用 `GcRefCell<Vec<u8>>` 支持原地变异（fill/write/copy）
#[derive(Debug, Clone, JsData, Trace, Finalize)]
pub struct Buffer {
  data: GcRefCell<Vec<u8>>,
}

impl Buffer {
  pub fn from_vec(data: Vec<u8>) -> Self {
    Self { data: GcRefCell::new(data) }
  }

  pub fn bytes(&self) -> impl std::ops::Deref<Target = Vec<u8>> + '_ {
    self.data.borrow()
  }

}

#[allow(non_snake_case)]
#[boa_class(rename = "Buffer")]
impl Buffer {
  /// `new Buffer(size)` — 分配（零填充）
  /// `new Buffer(array)` — 从数字数组创建
  /// `new Buffer(string, encoding?)` — 从字符串创建
  /// `new Buffer(ArrayBuffer)` — 从 ArrayBuffer 创建
  #[boa(constructor)]
  pub fn constructor(value: Option<JsValue>, encoding: Option<JsString>) -> JsResult<Self> {
    let value = value.unwrap_or(JsValue::undefined());

    // new Buffer(number)
    if let Some(n) = value.as_number() {
      let size = n as usize;
      return Ok(Self { data: GcRefCell::new(vec![0u8; size]) });
    }

    // new Buffer(string, encoding?)
    // 在 Boa #[boa_class] 中构造器无 ctx 参数，用 Context::default() 做简单转换
    if let Ok(s) = value.to_string(&mut Context::default()) {
      let s = s.to_std_string_escaped();
      let enc = encoding
        .as_ref()
        .map(|e| e.to_std_string_escaped())
        .unwrap_or_else(|| "utf8".to_string());
      return Ok(Self {
        data: GcRefCell::new(match enc.as_str() {
          "hex" => hex_decode(&s),
          _ => s.into_bytes(),
        }),
      });
    }

    // new Buffer(ArrayBuffer) / new Buffer(TypedArray)
    if let Some(obj) = value.as_object() {
      if let Ok(buf) = JsArrayBuffer::from_object(obj.clone()) {
        if let Some(data) = buf.data() {
          return Ok(Self { data: GcRefCell::new(data.to_vec()) });
        }
      }
      // TypedArray via .buffer
      if let Ok(buf_val) = obj.get(js_string!("buffer"), &mut Context::default()) {
        if let Some(buf_obj) = buf_val.as_object() {
          if let Ok(buf) = JsArrayBuffer::from_object(buf_obj.clone()) {
            if let Some(data) = buf.data() {
              return Ok(Self { data: GcRefCell::new(data.to_vec()) });
            }
          }
        }
      }
      // Array-like
      let mut ctx = Context::default();
      if let Ok(len) = obj.get(js_string!("length"), &mut ctx).and_then(|v| v.to_length(&mut ctx)) {
        let mut data = Vec::with_capacity(len as usize);
        for i in 0..len {
          if let Ok(v) = obj.get(i, &mut ctx).map(|v| v.to_number(&mut ctx).unwrap_or(0.0) as u8) {
            data.push(v);
          }
        }
        return Ok(Self { data: GcRefCell::new(data) });
      }
    }

    Ok(Self { data: GcRefCell::new(Vec::new()) })
  }

  /// `buf.length` — 字节数（readonly）
  #[boa(getter)]
  pub fn length(&self) -> usize {
    self.data.borrow().len()
  }

  /// `buf.toString(encoding?, start?, end?)` → string
  pub fn toString(&self, encoding: Option<JsString>, start: Option<usize>, end: Option<usize>) -> JsResult<JsString> {
    let data = self.data.borrow();
    let start = start.unwrap_or(0);
    let end = end.unwrap_or(data.len());
    let slice = if start < end && end <= data.len() {
      &data[start..end]
    } else {
      &[]
    };
    let enc = encoding
      .as_ref()
      .map(|e| e.to_std_string_escaped())
      .unwrap_or_else(|| "utf8".to_string());
    Ok(js_string!(match enc.as_str() {
      "hex" => hex_encode(slice),
      _ => String::from_utf8_lossy(slice).into_owned(),
    }))
  }

  /// `buf.toJSON()` → `{ type: "Buffer", data: number[] }`
  pub fn toJSON(&self, ctx: &mut Context) -> JsResult<JsValue> {
    let data = self.data.borrow();
    let obj = JsObject::with_object_proto(ctx.intrinsics());
    let _ = obj.set(js_string!("type"), JsValue::from(js_string!("Buffer")), false, ctx);
    let arr = boa_engine::object::builtins::JsArray::new(ctx);
    for &b in data.iter() {
      let _ = arr.push(JsValue::from(b as f64), ctx);
    }
    let _ = obj.set::<JsString, JsValue>(js_string!("data"), arr.into(), false, ctx);
    Ok(obj.into())
  }

  /// `buf.equals(other)` → bool
  pub fn equals(&self, other: JsValue) -> bool {
    if let Some(obj) = other.as_object() {
      if obj.is::<Buffer>() {
        // SAFETY: We just checked that obj is a Buffer
        let other_buf = obj.downcast_ref::<Buffer>().unwrap();
        return *self.data.borrow() == *other_buf.data.borrow();
      }
    }
    false
  }

  /// `buf.slice(start?, end?)` → Buffer
  pub fn slice(&self, start: Option<usize>, end: Option<usize>) -> Self {
    let data = self.data.borrow();
    let start = start.unwrap_or(0);
    let end = end.unwrap_or(data.len());
    if start >= end || start >= data.len() {
      return Self { data: GcRefCell::new(Vec::new()) };
    }
    let end = end.min(data.len());
    Self { data: GcRefCell::new(data[start..end].to_vec()) }
  }

  /// `buf.fill(value, offset?, end?)` → Buffer
  pub fn fill(&self, value: JsValue, offset: Option<usize>, end: Option<usize>) -> JsResult<JsValue> {
    let fill_byte = if let Some(n) = value.as_number() {
      n as u8
    } else if let Ok(s) = value.to_string(&mut Context::default()) {
      s.to_std_string_escaped().as_bytes().first().copied().unwrap_or(0)
    } else {
      0
    };
    let mut data = self.data.borrow_mut();
    let start = offset.unwrap_or(0);
    let end = end.unwrap_or(data.len());
    for i in start..end.min(data.len()) {
      data[i] = fill_byte;
    }
    Ok(JsValue::undefined())
  }

  /// `buf.write(string, offset?, length?)` → number of bytes written
  pub fn write(&self, string: JsString, offset: Option<usize>, _length: Option<usize>) -> usize {
    let bytes = string.to_std_string_escaped().into_bytes();
    let mut data = self.data.borrow_mut();
    let start = offset.unwrap_or(0);
    let len = bytes.len().min(data.len().saturating_sub(start));
    for i in 0..len {
      data[start + i] = bytes[i];
    }
    len
  }

  /// `buf.indexOf(value, byteOffset?)` → number
  pub fn indexOf(&self, value: JsValue, byte_offset: Option<usize>) -> i64 {
    let data = self.data.borrow();
    let start = byte_offset.unwrap_or(0).min(data.len());
    let needle = if let Some(n) = value.as_number() {
      vec![n as u8]
    } else if let Some(obj) = value.as_object() {
      if let Some(b) = obj.downcast_ref::<Buffer>() {
        b.data.borrow().clone()
      } else {
        return -1;
      }
    } else {
      return -1;
    };
    if needle.is_empty() {
      return start as i64;
    }
    data[start..]
      .windows(needle.len())
      .position(|w| w == needle)
      .map(|i| (start + i) as i64)
      .unwrap_or(-1)
  }

  // ── Static methods ────────────────────────────────────────────────────

  /// `Buffer.byteLength(string, encoding?)` → number
  pub fn byteLength(value: JsValue, encoding: Option<JsString>) -> usize {
    if let Ok(s) = value.to_string(&mut Context::default()) {
      let s = s.to_std_string_escaped();
      let enc = encoding
        .as_ref()
        .map(|e| e.to_std_string_escaped())
        .unwrap_or_else(|| "utf8".to_string());
      match enc.as_str() {
        "hex" => s.len() / 2,
        _ => s.len(),
      }
    } else if let Some(n) = value.as_number() {
      n as usize
    } else {
      0
    }
  }

  /// `Buffer.isBuffer(obj)` → bool
  pub fn isBuffer(value: JsValue) -> bool {
    value.is_object()
      && value.as_object().is_some_and(|o| o.is::<Buffer>())
  }

  /// `Buffer.concat(list, totalLength?)` → Buffer
  pub fn concat(list: JsValue, total_length: Option<usize>, ctx: &mut Context) -> JsResult<Self> {
    if let Some(obj) = list.as_object() {
      let len = obj
        .get(js_string!("length"), ctx)
        .and_then(|v| v.to_length(ctx))
        .unwrap_or(0);
      let total = total_length.unwrap_or_else(|| {
        let mut sum = 0usize;
        for i in 0..len {
          if let Ok(item) = obj.get(i, ctx) {
            if let Some(o) = item.as_object() {
              if let Some(buf_obj) = o.downcast_ref::<Buffer>() {
                sum += buf_obj.data.borrow().len();
              }
            }
          }
        }
        sum
      });
      let mut data = Vec::with_capacity(total);
      for i in 0..len {
        if let Ok(item) = obj.get(i, ctx) {
          if let Some(o) = item.as_object() {
            if let Some(buf_obj) = o.downcast_ref::<Buffer>() {
              let buf_data = buf_obj.data.borrow();
              let remaining = total.saturating_sub(data.len());
              let to_copy = buf_data.len().min(remaining);
              data.extend_from_slice(&buf_data[..to_copy]);
              if data.len() >= total {
                break;
              }
            }
          }
        }
      }
      return Ok(Self { data: GcRefCell::new(data) });
    }
    Ok(Self { data: GcRefCell::new(Vec::new()) })
  }

  /// `Buffer.compare(buf1, buf2)` → -1 | 0 | 1
  pub fn compare(a: JsValue, b: JsValue) -> i32 {
    let a_is_buffer = a.as_object().is_some_and(|o| o.is::<Buffer>());
    let b_is_buffer = b.as_object().is_some_and(|o| o.is::<Buffer>());
    if a_is_buffer && b_is_buffer {
      let a_ref = a.as_object().unwrap();
      let b_ref = b.as_object().unwrap();
      let a_buf = a_ref.downcast_ref::<Buffer>().unwrap();
      let b_buf = b_ref.downcast_ref::<Buffer>().unwrap();
      let a_data = a_buf.data.borrow();
      let b_data = b_buf.data.borrow();
      let min_len = a_data.len().min(b_data.len());
      for i in 0..min_len {
        if a_data[i] < b_data[i] { return -1; }
        if a_data[i] > b_data[i] { return 1; }
      }
      if a_data.len() < b_data.len() { -1 }
      else if a_data.len() > b_data.len() { 1 }
      else { 0 }
    } else {
      0
    }
  }

  /// `Buffer.from(value, encoding?)` — 工厂方法
  pub fn from(value: JsValue, encoding: Option<JsString>) -> JsResult<Self> {
    Self::constructor(Some(value), encoding)
  }

  /// `Buffer.alloc(size, fill?, encoding?)` — 分配 + 可选填充
  pub fn alloc(size: usize, fill: Option<JsValue>, _encoding: Option<JsString>) -> JsResult<Self> {
    let buf = Self { data: GcRefCell::new(vec![0u8; size]) };
    if let Some(f) = fill {
      let _ = buf.fill(f, Some(0), Some(size));
    }
    Ok(buf)
  }

  /// `Buffer.allocUnsafe(size)` — 分配未初始化
  pub fn allocUnsafe(size: usize) -> JsResult<Self> {
    Ok(Self { data: GcRefCell::new(vec![0u8; size]) })
  }
}

/// 注册 Buffer 全局类
pub fn register_buffer_global(context: &mut Context) -> Result<(), JsError> {
  context.register_global_class::<Buffer>()?;
  Ok(())
}

/// 创建 "node:buffer" 内置模块
pub fn create_node_buffer_module(context: &mut Context) -> Result<Module, String> {
  let export_names: &[JsString] = &[
    js_string!("Buffer"),
    js_string!("INSPECT_MAX_BYTES"),
    js_string!("kMaxLength"),
    js_string!("default"),
  ];

  let module = Module::synthetic(
    export_names,
    SyntheticModuleInitializer::from_copy_closure(
      |m: &boa_engine::module::SyntheticModule, ctx: &mut Context| {
        // 从全局获取 Buffer 构造函数（已由 register_buffer_global 注册）
          let buffer_ctor = ctx
          .global_object()
          .get(js_string!("Buffer"), ctx)
          .unwrap_or(JsValue::undefined());

        m.set_export(&js_string!("Buffer"), buffer_ctor.clone())?;
        m.set_export(&js_string!("INSPECT_MAX_BYTES"), JsValue::from(50.0))?;
        m.set_export(&js_string!("kMaxLength"), JsValue::from(0x7FFFFFFF as f64))?;

        let ns = JsObject::with_object_proto(ctx.intrinsics());
        let _ = ns.set(js_string!("Buffer"), buffer_ctor, false, ctx);
        let _ = ns.set(js_string!("INSPECT_MAX_BYTES"), JsValue::from(50.0), false, ctx);
        let _ = ns.set(js_string!("kMaxLength"), JsValue::from(0x7FFFFFFF as f64), false, ctx);
        m.set_export(&js_string!("default"), ns.into())?;

        Ok(())
      },
    ),
    None,
    None,
    context,
  );

  Ok(module)
}
