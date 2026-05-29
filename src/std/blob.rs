//! OOLONG 的 `Blob` 和 `File` 类 — WHATWG 规范
//!
//! 参考：
//!   - <https://w3c.github.io/FileAPI/#blob-section>
//!   - Deno / Bun / MDN 实现

use boa_engine::object::builtins::{JsArrayBuffer, JsPromise, JsTypedArray};
use boa_engine::{
  Context, JsData, JsNativeError, JsObject, JsResult, JsString, JsValue, boa_class, js_error,
  js_string,
};
use boa_gc::{Finalize, Trace};

// ── Blob ──────────────────────────────────────────────────────────────────────

/// JavaScript `Blob` 类
  #[derive(Debug, Clone, JsData, Trace, Finalize)]
pub struct Blob {
  pub data: Vec<u8>,
  pub mime_type: String,
}

impl Blob {
  /// 用字节数据创建 Blob（内部用）
  pub fn from_bytes(data: Vec<u8>, mime_type: String) -> Self {
    Self { data, mime_type }
  }

  /// 获取内部字节（外部模块需要时调用）
  pub fn bytes(&self) -> &[u8] {
    &self.data
  }

  /// 获取 MIME 类型
  pub fn mime_type(&self) -> &str {
    &self.mime_type
  }
}

#[boa_class(rename = "Blob")]
impl Blob {
  /// `new Blob(parts?, options?)`
  ///
  /// - `parts`：可迭代的 blob parts（ArrayBuffer / TypedArray / DataView / Blob / string）
  /// - `options`：`{ type: "..." }`
  #[boa(constructor)]
  pub fn constructor(parts: Option<JsValue>, options: Option<JsObject>) -> JsResult<Self> {
    let mut data = Vec::new();
    let mut mime_type = String::new();

    if let Some(parts) = parts {
      let parts = parts.as_object().ok_or_else(|| {
        js_error!(TypeError: "Blob parts must be an array or iterable")
      })?;
      let length = parts.get(js_string!("length"), &mut Context::default())
        .and_then(|v| v.to_length(&mut Context::default()))
        .unwrap_or(0);
      for i in 0..length {
        let item = parts.get(i, &mut Context::default())
          .map_err(|_| js_error!(TypeError: "Cannot read blob part at index {i}"))?;
        append_blob_part(&item, &mut data)?;
      }
    }

    if let Some(opts) = options
      && let Ok(type_val) = opts.get(js_string!("type"), &mut Context::default())
      && let Ok(s) = type_val.to_string(&mut Context::default())
    {
      mime_type = s.to_std_string_escaped().to_lowercase();
    }

    Ok(Self { data, mime_type })
  }

  /// `blob.size` — 字节数（readonly）
  #[boa(getter)]
  pub fn size(&self) -> usize {
    self.data.len()
  }

  /// `blob.type` — MIME 类型（readonly）
  #[boa(getter)]
  #[boa(rename = "type")]
  pub fn r#type(&self) -> JsString {
    JsString::from(self.mime_type.as_str())
  }

  /// `blob.text()` → Promise<string>
  pub fn text(&self, context: &mut Context) -> JsPromise {
    let data = self.data.clone();
    JsPromise::from_async_fn(
      async move |_| {
        let s = String::from_utf8_lossy(&data);
        Ok(JsString::from(s).into())
      },
      context,
    )
  }

  /// `blob.arrayBuffer()` → Promise<ArrayBuffer>
  pub fn array_buffer(&self, context: &mut Context) -> JsPromise {
    let data = self.data.clone();
    JsPromise::from_async_fn(
      async move |context| {
        let ctx = &mut context.borrow_mut();
        let len = data.len();
        let buf = JsArrayBuffer::new(len, ctx)
          .map_err(|e| JsNativeError::error().with_message(format!("{e}")))?;
        if let Some(mut dst) = buf.data_mut() {
          dst.copy_from_slice(&data);
        }
        Ok(JsValue::from(buf))
      },
      context,
    )
  }

  /// `blob.slice(start?, end?, contentType?)` → Blob
  pub fn slice(
    &self,
    start: Option<i64>,
    end: Option<i64>,
    content_type: Option<JsString>,
  ) -> Self {
    let len = self.data.len() as i64;
    let relative_start = start.map_or(0, |s| {
      if s < 0 { (len + s).max(0) } else { s.min(len) }
    });
    let relative_end = end.map_or(len, |e| {
      if e < 0 { (len + e).max(0) } else { e.min(len) }
    });
    let start = relative_start as usize;
    let end = relative_end as usize;

    let sliced: Vec<u8> = if start < end && start < self.data.len() {
      self.data[start..end.min(self.data.len())].to_vec()
    } else {
      Vec::new()
    };

    let mime = content_type
      .map(|s| s.to_std_string_escaped().to_lowercase())
      .unwrap_or(self.mime_type.clone());

    Self { data: sliced, mime_type: mime }
  }

  /// `blob.stream()` — 暂未实现
  #[allow(unused_variables)]
  pub fn stream(&self, context: &mut Context) -> JsResult<JsValue> {
    Err(js_error!(Error: "Blob.stream() is not implemented yet"))
  }
}

// ── File ──────────────────────────────────────────────────────────────────────

/// JavaScript `File` 类（extends Blob）
#[derive(Debug, Clone, JsData, Trace, Finalize)]
pub struct JsFile {
  pub data: Vec<u8>,
  pub mime_type: String,
  pub name: String,
  pub last_modified: i64,
}

#[boa_class(rename = "File")]
impl JsFile {
  /// `new File(parts, name, options?)`
  #[boa(constructor)]
  pub fn constructor(
    parts: JsValue,
    name: JsString,
    options: Option<JsObject>,
  ) -> JsResult<Self> {
    // 复用 Blob 的 parts 解析逻辑
    let mut data = Vec::new();
    let parts_obj = parts.as_object().ok_or_else(|| {
      js_error!(TypeError: "File parts must be an array or iterable")
    })?;
    let length = parts_obj.get(js_string!("length"), &mut Context::default())
      .and_then(|v| v.to_length(&mut Context::default()))
      .unwrap_or(0);
    for i in 0..length {
      let item = parts_obj.get(i, &mut Context::default())
        .map_err(|_| js_error!(TypeError: "Cannot read file part at index {i}"))?;
      append_blob_part(&item, &mut data)?;
    }

    let file_name = name.to_std_string_escaped();
    let mut mime_type = String::new();
    let mut last_modified = std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .map(|d| d.as_millis() as i64)
      .unwrap_or(0);

    if let Some(opts) = options {
      if let Ok(type_val) = opts.get(js_string!("type"), &mut Context::default())
        && let Ok(s) = type_val.to_string(&mut Context::default())
      {
        mime_type = s.to_std_string_escaped().to_lowercase();
      }
      if let Ok(modified) = opts.get(js_string!("lastModified"), &mut Context::default())
        && let Ok(ts) = modified.to_length(&mut Context::default())
      {
        last_modified = ts as i64;
      }
    }

    Ok(Self {
      data,
      mime_type,
      name: file_name,
      last_modified,
    })
  }

  /// `file.name`
  #[boa(getter)]
  pub fn name(&self) -> JsString {
    JsString::from(self.name.as_str())
  }

  /// `file.lastModified`
  #[boa(getter)]
  pub fn last_modified(&self) -> f64 {
    self.last_modified as f64
  }

  /// `file.size`
  #[boa(getter)]
  pub fn size(&self) -> usize {
    self.data.len()
  }

  /// `file.type`
  #[boa(getter)]
  #[boa(rename = "type")]
  pub fn r#type(&self) -> JsString {
    JsString::from(self.mime_type.as_str())
  }

  /// `file.text()` → Promise<string>
  pub fn text(&self, context: &mut Context) -> JsPromise {
    let data = self.data.clone();
    JsPromise::from_async_fn(
      async move |_| {
        let s = String::from_utf8_lossy(&data);
        Ok(JsString::from(s).into())
      },
      context,
    )
  }

  /// `file.arrayBuffer()` → Promise<ArrayBuffer>
  pub fn array_buffer(&self, context: &mut Context) -> JsPromise {
    let data = self.data.clone();
    JsPromise::from_async_fn(
      async move |context| {
        let ctx = &mut context.borrow_mut();
        let len = data.len();
        let buf = JsArrayBuffer::new(len, ctx)
          .map_err(|e| JsNativeError::error().with_message(format!("{e}")))?;
        if let Some(mut dst) = buf.data_mut() {
          dst.copy_from_slice(&data);
        }
        Ok(JsValue::from(buf))
      },
      context,
    )
  }

  /// `file.slice(start?, end?, contentType?)` → Blob（注意返回 Blob 而非 File）
  pub fn slice(
    &self,
    start: Option<i64>,
    end: Option<i64>,
    content_type: Option<JsString>,
  ) -> Blob {
    let len = self.data.len() as i64;
    let relative_start = start.map_or(0, |s| {
      if s < 0 { (len + s).max(0) } else { s.min(len) }
    });
    let relative_end = end.map_or(len, |e| {
      if e < 0 { (len + e).max(0) } else { e.min(len) }
    });
    let start = relative_start as usize;
    let end = relative_end as usize;

    let sliced: Vec<u8> = if start < end && start < self.data.len() {
      self.data[start..end.min(self.data.len())].to_vec()
    } else {
      Vec::new()
    };

    let mime = content_type
      .map(|s| s.to_std_string_escaped().to_lowercase())
      .unwrap_or(self.mime_type.clone());

    Blob::from_bytes(sliced, mime)
  }
}

// ── Helper ────────────────────────────────────────────────────────────────────

/// 从 JsValue 提取字节追加到 blob 数据
fn append_blob_part(value: &JsValue, data: &mut Vec<u8>) -> JsResult<()> {
  // String
  if let Ok(s) = value.to_string(&mut Context::default()) {
    data.extend_from_slice(s.to_std_string_escaped().as_bytes());
    return Ok(());
  }

  let obj = value.as_object().ok_or_else(|| {
    js_error!(TypeError: "Blob part must be a string, ArrayBuffer, TypedArray, DataView, or Blob")
  })?;

  // Blob
  if let Ok(blob) = obj.clone().downcast::<Blob>() {
    let b = blob.borrow();
    data.extend_from_slice(&b.data().data);
    return Ok(());
  }

  // File
  if let Ok(file) = obj.clone().downcast::<JsFile>() {
    let f = file.borrow();
    data.extend_from_slice(&f.data().data);
    return Ok(());
  }

  // ArrayBuffer
  if let Ok(buf) = JsArrayBuffer::from_object(obj.clone())
    && let Some(src) = buf.data()
  {
    data.extend_from_slice(&src);
    return Ok(());
  }

  // TypedArray (via .buffer + byteOffset + byteLength)
  if let Ok(typed) = JsTypedArray::from_object(obj.clone()) {
    let mut ctx_mut = Context::default();
    let offset = typed.byte_offset(&mut ctx_mut)?;
    let length = typed.byte_length(&mut ctx_mut)?;
    if let Ok(buf_val) = typed.buffer(&mut ctx_mut)
      && let Some(buf_obj) = buf_val.as_object()
      && let Ok(buf) = JsArrayBuffer::from_object(buf_obj.clone())
      && let Some(src) = buf.data()
    {
      let start = offset;
      let end = (offset + length).min(src.len());
      data.extend_from_slice(&src[start..end]);
      return Ok(());
    }
  }

  Err(js_error!(
    TypeError: "Blob part must be a string, ArrayBuffer, TypedArray, DataView, or Blob"
  ))
}

// ── Registration ──────────────────────────────────────────────────────────────

/// 注册 Blob 和 File 到全局
pub fn register_globals(context: &mut Context) -> JsResult<()> {
  context.register_global_class::<Blob>()?;
  context.register_global_class::<JsFile>()?;
  Ok(())
}
