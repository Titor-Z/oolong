use std::cell::RefCell;
use std::collections::HashMap;

use boa_engine::{
    Context, JsObject, JsResult, JsString, JsValue, Module, NativeFunction, Source, js_string,
    object::FunctionObjectBuilder,
};

use sha2::digest::Digest;

thread_local! {
    static HASH_STORE: RefCell<HashMap<u32, HashState>> = RefCell::new(HashMap::new());
    static NEXT_HASH_ID: RefCell<u32> = const { RefCell::new(1) };
}

enum HashState {
    Sha256(sha2::Sha256),
    Sha384(sha2::Sha384),
    Sha512(sha2::Sha512),
    Sha1(sha1::Sha1),
    Md5(md5::Md5),
}

impl HashState {
    fn update(&mut self, data: &[u8]) {
        match self {
            Self::Sha256(h) => h.update(data),
            Self::Sha384(h) => h.update(data),
            Self::Sha512(h) => h.update(data),
            Self::Sha1(h) => h.update(data),
            Self::Md5(h) => h.update(data),
        }
    }
    fn digest(self) -> Vec<u8> {
        match self {
            Self::Sha256(h) => h.finalize().to_vec(),
            Self::Sha384(h) => h.finalize().to_vec(),
            Self::Sha512(h) => h.finalize().to_vec(),
            Self::Sha1(h) => h.finalize().to_vec(),
            Self::Md5(h) => h.finalize().to_vec(),
        }
    }
}

fn hash_create_impl(algo: &str) -> Result<HashState, String> {
    match algo.to_lowercase().as_str() {
        "sha256" | "sha-256" => Ok(HashState::Sha256(sha2::Sha256::new())),
        "sha384" | "sha-384" => Ok(HashState::Sha384(sha2::Sha384::new())),
        "sha512" | "sha-512" => Ok(HashState::Sha512(sha2::Sha512::new())),
        "sha1" | "sha-1" => Ok(HashState::Sha1(sha1::Sha1::new())),
        "md5" | "md-5" => Ok(HashState::Md5(md5::Md5::new())),
        other => Err(format!("unsupported hash algorithm: {other}")),
    }
}

fn hex_encode(data: &[u8]) -> String {
    data.iter().map(|b| format!("{b:02x}")).collect()
}

fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARS[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

fn make_fn<F>(f: F, name: &str, len: usize, ctx: &mut Context) -> JsValue
where
    F: Fn(&JsValue, &[JsValue], &mut Context) -> JsResult<JsValue> + 'static,
{
    let native = unsafe { NativeFunction::from_closure(f) };
    FunctionObjectBuilder::new(ctx.realm(), native)
        .name(JsString::from(name))
        .length(len)
        .build()
        .into()
}

fn str_val(s: &str) -> JsValue {
    JsValue::from(JsString::from(s))
}

fn crypto_js_source() -> String {
    String::from(
        r#"
var _cn = globalThis._crypto_native;

function Hash(algo) {
  this._id = _cn._hashCreate(algo);
  if (this._id < 0) throw new Error("Unsupported hash algorithm: " + algo);
}
Hash.prototype.update = function(data) {
  _cn._hashUpdate(this._id, data);
  return this;
};
Hash.prototype.digest = function(encoding) {
  return _cn._hashDigest(this._id, encoding || "hex");
};

function Hmac(algo, key) {
  this._id = _cn._hmacCreate(algo, key);
  if (this._id < 0) throw new Error("Unsupported hmac algorithm: " + algo);
}
Hmac.prototype.update = function(data) {
  _cn._hmacUpdate(this._id, data);
  return this;
};
Hmac.prototype.digest = function(encoding) {
  return _cn._hmacDigest(this._id, encoding || "hex");
};

function createHash(algo) { return new Hash(algo); }
function createHmac(algo, key) { return new Hmac(algo, key); }

function randomBytes(size, cb) {
  var raw = _cn._randomBytesRaw(size);
  var buf = Buffer.from(raw, "hex");
  if (typeof cb === "function") { cb(null, buf); }
  return buf;
}

function randomUUID() { return _cn._randomUUID(); }

var crypto = {
  createHash: createHash,
  createHmac: createHmac,
  randomBytes: randomBytes,
  randomUUID: randomUUID,
  Hash: Hash,
  Hmac: Hmac,
};

export { createHash, createHmac, randomBytes, randomUUID, Hash, Hmac };
export default crypto;
"#,
    )
}

fn register_crypto_native(ctx: &mut Context) -> Result<(), String> {
    let obj = JsObject::with_object_proto(ctx.intrinsics());

    let hash_create = make_fn(
        |_: &JsValue, args: &[JsValue], _ctx: &mut Context| -> JsResult<JsValue> {
            let algo = args
                .first()
                .and_then(|v| v.as_string())
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            match hash_create_impl(&algo) {
                Ok(state) => {
                    let id = NEXT_HASH_ID.with(|c| {
                        let mut c = c.borrow_mut();
                        let id = *c;
                        *c += 1;
                        id
                    });
                    HASH_STORE.with(|s| {
                        s.borrow_mut().insert(id, state);
                    });
                    Ok(JsValue::from(id as i32))
                }
                Err(_) => Ok(JsValue::from(-1)),
            }
        },
        "_hashCreate",
        1,
        ctx,
    );

    let hash_update = make_fn(
        |_: &JsValue, args: &[JsValue], _ctx: &mut Context| -> JsResult<JsValue> {
            let id = args
                .first()
                .and_then(|v| v.as_number())
                .map(|n| n as i32)
                .unwrap_or(-1) as u32;
            let data = js_val_to_bytes(args.get(1));
            HASH_STORE.with(|s| {
                if let Some(state) = s.borrow_mut().get_mut(&id) {
                    state.update(&data);
                }
            });
            Ok(JsValue::undefined())
        },
        "_hashUpdate",
        2,
        ctx,
    );

    let hash_digest = make_fn(
        |_: &JsValue, args: &[JsValue], _ctx: &mut Context| -> JsResult<JsValue> {
            let id = args
                .first()
                .and_then(|v| v.as_number())
                .map(|n| n as i32)
                .unwrap_or(-1) as u32;
            let encoding = args
                .get(1)
                .and_then(|v| v.as_string())
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            let result =
                HASH_STORE.with(|s| s.borrow_mut().remove(&id).map(|state| state.digest()));
            let out = match result {
                Some(hash) => match encoding.as_str() {
                    "base64" => str_val(&base64_encode(&hash)),
                    _ => str_val(&hex_encode(&hash)),
                },
                None => JsValue::undefined(),
            };
            Ok(out)
        },
        "_hashDigest",
        2,
        ctx,
    );

    let hmac_create = make_fn(
        |_: &JsValue, _args: &[JsValue], _ctx: &mut Context| -> JsResult<JsValue> {
            Ok(JsValue::from(-1))
        },
        "_hmacCreate",
        2,
        ctx,
    );

    let hmac_update = make_fn(
        |_: &JsValue, _args: &[JsValue], _ctx: &mut Context| -> JsResult<JsValue> {
            Ok(JsValue::undefined())
        },
        "_hmacUpdate",
        2,
        ctx,
    );

    let hmac_digest = make_fn(
        |_: &JsValue, _args: &[JsValue], _ctx: &mut Context| -> JsResult<JsValue> {
            Ok(JsValue::undefined())
        },
        "_hmacDigest",
        2,
        ctx,
    );

    let random_bytes_raw = make_fn(
        |_: &JsValue, args: &[JsValue], _ctx: &mut Context| -> JsResult<JsValue> {
            let size = args
                .first()
                .and_then(|v| v.as_number())
                .map(|n| n as usize)
                .unwrap_or(0);
            let mut buf = vec![0u8; size];
            let _ = getrandom::getrandom(&mut buf);
            Ok(str_val(&hex_encode(&buf)))
        },
        "_randomBytesRaw",
        1,
        ctx,
    );

    let random_uuid = make_fn(
        |_: &JsValue, _args: &[JsValue], _ctx: &mut Context| -> JsResult<JsValue> {
            Ok(str_val(&uuid::Uuid::new_v4().to_string()))
        },
        "_randomUUID",
        0,
        ctx,
    );

    let _ = obj.set(js_string!("_hashCreate"), hash_create, false, ctx);
    let _ = obj.set(js_string!("_hashUpdate"), hash_update, false, ctx);
    let _ = obj.set(js_string!("_hashDigest"), hash_digest, false, ctx);
    let _ = obj.set(js_string!("_hmacCreate"), hmac_create, false, ctx);
    let _ = obj.set(js_string!("_hmacUpdate"), hmac_update, false, ctx);
    let _ = obj.set(js_string!("_hmacDigest"), hmac_digest, false, ctx);
    let _ = obj.set(js_string!("_randomBytesRaw"), random_bytes_raw, false, ctx);
    let _ = obj.set(js_string!("_randomUUID"), random_uuid, false, ctx);

    let _ = ctx.register_global_property::<JsString, JsValue>(
        js_string!("_crypto_native"),
        obj.into(),
        boa_engine::property::Attribute::all(),
    );

    Ok(())
}

fn js_val_to_bytes(val: Option<&JsValue>) -> Vec<u8> {
    let val = match val {
        Some(v) => v,
        None => return Vec::new(),
    };
    if let Some(s) = val.as_string() {
        s.to_std_string_escaped().into_bytes()
    } else if let Some(obj) = val.as_object() {
        if let Ok(arr) = boa_engine::object::builtins::JsTypedArray::from_object(obj.clone()) {
            let mut bytes = Vec::new();
            let mut ctx = Context::default();
            if let Ok(len) = arr.length(&mut ctx) {
                for i in 0..len {
                    if let Ok(v) = arr.get(i, &mut ctx)
                        && let Some(n) = v.as_number()
                    {
                        bytes.push(n as u8);
                    }
                }
            }
            return bytes;
        }
        Vec::new()
    } else {
        let mut ctx = Context::default();
        val.to_string(&mut ctx)
            .map(|s| s.to_std_string_escaped().into_bytes())
            .unwrap_or_default()
    }
}

pub fn create_node_crypto_module(context: &mut Context) -> Result<Module, String> {
    register_crypto_native(context)?;
    let js = crypto_js_source();
    let source = Source::from_bytes(js.as_bytes());
    Module::parse(source, None, context).map_err(|e| format!("创建 node:crypto 模块失败: {e}"))
}
