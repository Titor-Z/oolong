use std::io::Read;

use boa_engine::{
    Context, JsResult, JsValue, Module, NativeFunction, Source, js_string,
    object::FunctionObjectBuilder, object::builtins::JsArrayBuffer,
};
use flate2::Compression;
use flate2::read::{
    DeflateDecoder, DeflateEncoder, GzDecoder, GzEncoder, ZlibDecoder, ZlibEncoder,
};

fn make_fn<F>(f: F, name: &str, len: usize, ctx: &mut Context) -> JsValue
where
    F: Fn(&JsValue, &[JsValue], &mut Context) -> JsResult<JsValue> + 'static,
{
    let native = unsafe { NativeFunction::from_closure(f) };
    FunctionObjectBuilder::new(ctx.realm(), native)
        .name(js_string!(name))
        .length(len)
        .build()
        .into()
}

fn extract_bytes(value: &JsValue, ctx: &mut Context) -> Vec<u8> {
    if let Some(obj) = value.as_object() {
        // Direct ArrayBuffer
        if let Ok(buf) = JsArrayBuffer::from_object(obj.clone())
            && let Some(data) = buf.data()
        {
            return data.to_vec();
        }
        // TypedArray via .buffer
        if let Ok(buf_val) = obj.get(js_string!("buffer"), ctx)
            && let Some(buf_obj) = buf_val.as_object()
            && let Ok(buf) = JsArrayBuffer::from_object(buf_obj.clone())
            && let Some(data) = buf.data()
        {
            return data.to_vec();
        }
    }
    // String fallback
    if let Some(s) = value.as_string() {
        return s.to_std_string_escaped().into_bytes();
    }
    vec![]
}

fn bytes_to_js_array_buffer(bytes: Vec<u8>, ctx: &mut Context) -> JsValue {
    let len = bytes.len();
    let buf = JsArrayBuffer::new(len, ctx).expect("创建 ArrayBuffer 失败");
    if let Some(mut data) = buf.data_mut() {
        data.copy_from_slice(&bytes);
    }
    buf.into()
}

fn register_zlib_native(ctx: &mut Context) -> Result<(), String> {
    let gzip_fn = make_fn(
        |_: &JsValue, args: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
            let data = args
                .first()
                .map(|v| extract_bytes(v, ctx))
                .unwrap_or_default();
            let mut encoder = GzEncoder::new(&data[..], Compression::default());
            let mut out = Vec::new();
            if encoder.read_to_end(&mut out).is_ok() {
                Ok(bytes_to_js_array_buffer(out, ctx))
            } else {
                Ok(JsValue::undefined())
            }
        },
        "_zlibGzip",
        1,
        ctx,
    );

    let gunzip_fn = make_fn(
        |_: &JsValue, args: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
            let data = args
                .first()
                .map(|v| extract_bytes(v, ctx))
                .unwrap_or_default();
            let mut decoder = GzDecoder::new(&data[..]);
            let mut out = Vec::new();
            if decoder.read_to_end(&mut out).is_ok() {
                Ok(bytes_to_js_array_buffer(out, ctx))
            } else {
                Ok(JsValue::undefined())
            }
        },
        "_zlibGunzip",
        1,
        ctx,
    );

    let deflate_fn = make_fn(
        |_: &JsValue, args: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
            let data = args
                .first()
                .map(|v| extract_bytes(v, ctx))
                .unwrap_or_default();
            let mut encoder = ZlibEncoder::new(&data[..], Compression::default());
            let mut out = Vec::new();
            if encoder.read_to_end(&mut out).is_ok() {
                Ok(bytes_to_js_array_buffer(out, ctx))
            } else {
                Ok(JsValue::undefined())
            }
        },
        "_zlibDeflate",
        1,
        ctx,
    );

    let inflate_fn = make_fn(
        |_: &JsValue, args: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
            let data = args
                .first()
                .map(|v| extract_bytes(v, ctx))
                .unwrap_or_default();
            let mut decoder = ZlibDecoder::new(&data[..]);
            let mut out = Vec::new();
            if decoder.read_to_end(&mut out).is_ok() {
                Ok(bytes_to_js_array_buffer(out, ctx))
            } else {
                Ok(JsValue::undefined())
            }
        },
        "_zlibInflate",
        1,
        ctx,
    );

    let deflate_raw_fn = make_fn(
        |_: &JsValue, args: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
            let data = args
                .first()
                .map(|v| extract_bytes(v, ctx))
                .unwrap_or_default();
            let mut encoder = DeflateEncoder::new(&data[..], Compression::default());
            let mut out = Vec::new();
            if encoder.read_to_end(&mut out).is_ok() {
                Ok(bytes_to_js_array_buffer(out, ctx))
            } else {
                Ok(JsValue::undefined())
            }
        },
        "_zlibDeflateRaw",
        1,
        ctx,
    );

    let inflate_raw_fn = make_fn(
        |_: &JsValue, args: &[JsValue], ctx: &mut Context| -> JsResult<JsValue> {
            let data = args
                .first()
                .map(|v| extract_bytes(v, ctx))
                .unwrap_or_default();
            let mut decoder = DeflateDecoder::new(&data[..]);
            let mut out = Vec::new();
            if decoder.read_to_end(&mut out).is_ok() {
                Ok(bytes_to_js_array_buffer(out, ctx))
            } else {
                Ok(JsValue::undefined())
            }
        },
        "_zlibInflateRaw",
        1,
        ctx,
    );

    let _ = ctx.register_global_property(
        js_string!("_zlibGzip"),
        gzip_fn,
        boa_engine::property::Attribute::all(),
    );
    let _ = ctx.register_global_property(
        js_string!("_zlibGunzip"),
        gunzip_fn,
        boa_engine::property::Attribute::all(),
    );
    let _ = ctx.register_global_property(
        js_string!("_zlibDeflate"),
        deflate_fn,
        boa_engine::property::Attribute::all(),
    );
    let _ = ctx.register_global_property(
        js_string!("_zlibInflate"),
        inflate_fn,
        boa_engine::property::Attribute::all(),
    );
    let _ = ctx.register_global_property(
        js_string!("_zlibDeflateRaw"),
        deflate_raw_fn,
        boa_engine::property::Attribute::all(),
    );
    let _ = ctx.register_global_property(
        js_string!("_zlibInflateRaw"),
        inflate_raw_fn,
        boa_engine::property::Attribute::all(),
    );

    Ok(())
}

fn zlib_js_source() -> String {
    String::from(
        r#"
function _callNative(fnName, buffer) {
  var fn = globalThis[fnName];
  if (typeof fn !== "function") throw new Error("native zlib function not found: " + fnName);
  var result = fn(buffer);
  if (result === undefined) throw new Error("zlib operation failed");
  return result;
}

function _gzipSync(buffer) { return _callNative("_zlibGzip", buffer); }
function _gunzipSync(buffer) { return _callNative("_zlibGunzip", buffer); }
function _deflateSync(buffer) { return _callNative("_zlibDeflate", buffer); }
function _inflateSync(buffer) { return _callNative("_zlibInflate", buffer); }
function _deflateRawSync(buffer) { return _callNative("_zlibDeflateRaw", buffer); }
function _inflateRawSync(buffer) { return _callNative("_zlibInflateRaw", buffer); }

function _unzipSync(buffer) {
  try { return _gunzipSync(buffer); } catch(e) {}
  try { return _inflateSync(buffer); } catch(e) {}
  try { return _inflateRawSync(buffer); } catch(e) {}
  throw new Error("unzipSync: unable to determine compression format");
}

var _constants = {
    Z_OK: 0,
    Z_ERRNO: -1,
    Z_STREAM_ERROR: -2,
    Z_DATA_ERROR: -3,
    Z_MEM_ERROR: -4,
    Z_BUF_ERROR: -5,
    Z_VERSION_ERROR: -6,
    Z_NO_COMPRESSION: 0,
    Z_BEST_SPEED: 1,
    Z_BEST_COMPRESSION: 9,
    Z_DEFAULT_COMPRESSION: -1,
    Z_FILTERED: 1,
    Z_HUFFMAN_ONLY: 2,
    Z_RLE: 3,
    Z_FIXED: 4,
    Z_DEFAULT_STRATEGY: 0,
    ZLIB_VERNUM: 0x12a0,
    DEFLATE: 1,
    INFLATE: 2,
    GZIP: 3,
    GUNZIP: 4,
    DEFLATERAW: 5,
    INFLATERAW: 6,
    UNZIP: 7,
  };

var _zlib = {
  gzipSync: _gzipSync,
  gunzipSync: _gunzipSync,
  deflateSync: _deflateSync,
  inflateSync: _inflateSync,
  deflateRawSync: _deflateRawSync,
  inflateRawSync: _inflateRawSync,
  unzipSync: _unzipSync,
  constants: _constants,
};
export {
  _gzipSync as gzipSync, _gunzipSync as gunzipSync,
  _deflateSync as deflateSync, _inflateSync as inflateSync,
  _deflateRawSync as deflateRawSync, _inflateRawSync as inflateRawSync,
  _unzipSync as unzipSync,
  _constants as constants,
};
export default _zlib;
"#,
    )
}

pub fn create_node_zlib_module(context: &mut Context) -> Result<Module, String> {
    register_zlib_native(context)?;
    let js = zlib_js_source();
    let source = Source::from_bytes(js.as_bytes());
    Module::parse(source, None, context).map_err(|e| format!("创建 node:zlib 模块失败: {e}"))
}
