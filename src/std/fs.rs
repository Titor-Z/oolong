use std::collections::VecDeque;
use std::time::UNIX_EPOCH;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use boa_engine::job::{NativeJob, TimeoutJob};
use boa_engine::object::builtins::{JsArray, JsArrayBuffer, JsFunction, JsPromise};
use boa_engine::{
  object::FunctionObjectBuilder,
  Context, Finalize, IntoJsFunctionCopied, JsData, JsError, JsNativeError, JsObject, JsResult, JsString, JsValue,
  Module, NativeFunction, Trace, js_string,
};
use boa_engine::module::SyntheticModuleInitializer;
use boa_engine::symbol::JsSymbol;
use boa_gc::{Gc, GcRefCell};

fn to_js_fn(f: NativeFunction, name: &str, len: usize, ctx: &mut Context) -> JsValue {
  FunctionObjectBuilder::new(ctx.realm(), f)
    .name(JsString::from(name))
    .length(len)
    .build()
    .into()
}

fn resolved_promise(val: JsValue, ctx: &mut Context) -> JsValue {
  JsPromise::resolve(val, ctx).into()
}

fn rejected_promise(msg: &str, ctx: &mut Context) -> JsValue {
  let err = JsNativeError::typ().with_message(msg.to_string());
  let (promise, resolvers) = JsPromise::new_pending(ctx);
  let _ = resolvers.reject.call(
    &JsValue::undefined(),
    &[err.to_opaque(ctx).into()],
    ctx,
  );
  promise.into()
}

fn metadata_to_obj(md: &std::fs::Metadata, ctx: &mut Context) -> JsResult<JsValue> {
  let obj = JsObject::with_object_proto(ctx.intrinsics());
  let _ = obj.set(js_string!("isFile"), JsValue::from(md.is_file()), false, ctx);
  let _ = obj.set(js_string!("isDirectory"), JsValue::from(md.is_dir()), false, ctx);
  let _ = obj.set(js_string!("isSymlink"), JsValue::from(md.is_symlink()), false, ctx);
  let _ = obj.set(js_string!("size"), JsValue::from(md.len() as f64), false, ctx);
  let _ = obj.set(js_string!("size"), JsValue::from(md.len() as f64), false, ctx);
  #[cfg(unix)]
  let _ = obj.set(js_string!("mode"), JsValue::from(md.permissions().mode() as f64), false, ctx);
  #[cfg(not(unix))]
  let _ = obj.set(js_string!("mode"), JsValue::from(0.0), false, ctx);
  if let Ok(mt) = md.modified() {
    let ms = mt.duration_since(UNIX_EPOCH).map(|d| d.as_millis() as f64).unwrap_or(0.0);
    let _ = obj.set(js_string!("mtimeMs"), JsValue::from(ms), false, ctx);
  }
  if let Ok(at) = md.accessed() {
    let ms = at.duration_since(UNIX_EPOCH).map(|d| d.as_millis() as f64).unwrap_or(0.0);
    let _ = obj.set(js_string!("atimeMs"), JsValue::from(ms), false, ctx);
  }
  if let Ok(bt) = md.created() {
    let ms = bt.duration_since(UNIX_EPOCH).map(|d| d.as_millis() as f64).unwrap_or(0.0);
    let _ = obj.set(js_string!("birthtimeMs"), JsValue::from(ms), false, ctx);
  }
  Ok(obj.into())
}

/// 从 JsValue 中提取 opts.recursive
fn is_recursive(opts: Option<JsValue>, ctx: &mut Context) -> bool {
  match opts {
    Some(v) if v.is_object() => {
      let obj = v.as_object().unwrap();
      obj.get(js_string!("recursive"), ctx).map(|v| v.to_boolean()).unwrap_or(false)
    }
    _ => false,
  }
}

// ── fs.watch 状态管理 ─────────────────────────────────────────────────────

/// 单个 watch 项的轮询状态
#[derive(Trace, Finalize)]
struct FsWatchEntry {
    id: u32,
    path: String,
    /// 上次检测到的修改时间（Unix 纪元毫秒）
    #[unsafe_ignore_trace]
    last_mtime: u128,
    pending: VecDeque<JsFunction>,
    done: bool,
}

#[derive(Trace, Finalize, JsData)]
struct FsWatchState {
    entries: GcRefCell<Vec<FsWatchEntry>>,
    next_id: u32,
}

fn get_watch_state(ctx: &mut Context) -> Gc<GcRefCell<FsWatchState>> {
    if !ctx.has_data::<Gc<GcRefCell<FsWatchState>>>() {
        ctx.insert_data(Gc::new(GcRefCell::new(FsWatchState {
            entries: GcRefCell::new(Vec::new()),
            next_id: 1,
        })));
    }
    ctx.get_data::<Gc<GcRefCell<FsWatchState>>>()
        .expect("FsWatchState 应存在")
        .clone()
}

/// 获取路径当前 mtime 的毫秒时间戳
fn current_mtime(path: &str) -> u128 {
    std::fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

/// 从 JsValue 中提取字节数据（支持 ArrayBuffer / TypedArray）
fn extract_bytes_from_jsvalue(value: &JsValue, ctx: &mut Context) -> Option<Vec<u8>> {
  let obj = value.as_object()?;
  // Direct ArrayBuffer
  if let Ok(buf) = JsArrayBuffer::from_object(obj.clone())
    && let Some(data) = buf.data()
  {
    return Some(data.to_vec());
  }
  // TypedArray (Uint8Array etc.) via .buffer property
  if let Ok(buf_val) = obj.get(js_string!("buffer"), ctx)
    && let Some(buf_obj) = buf_val.as_object()
    && let Ok(buf) = JsArrayBuffer::from_object(buf_obj.clone())
    && let Some(data) = buf.data()
  {
    return Some(data.to_vec());
  }
  None
}

/// 创建 "fs" 内置模块
pub fn create_fs_module(context: &mut Context) -> Result<Module, String> {
  let export_names: &[JsString] = &[
    // P0
    js_string!("readTextFile"),
    js_string!("writeTextFile"),
    js_string!("readFile"),
    js_string!("writeFile"),
    js_string!("readFileSync"),
    js_string!("exists"),
    // P1
    js_string!("mkdir"),
    js_string!("remove"),
    js_string!("readdir"),
    js_string!("stat"),
    js_string!("lstat"),
    js_string!("appendFile"),
    js_string!("copyFile"),
    js_string!("rename"),
    js_string!("realpath"),
    js_string!("symlink"),
    // P2
    js_string!("existsSync"),
    js_string!("mkdirSync"),
    js_string!("removeSync"),
    js_string!("readdirSync"),
    js_string!("statSync"),
    js_string!("lstatSync"),
    js_string!("appendFileSync"),
    js_string!("copyFileSync"),
    js_string!("renameSync"),
    js_string!("realpathSync"),
    js_string!("symlinkSync"),
    // P3
    js_string!("chmod"),
    js_string!("chown"),
    js_string!("link"),
    js_string!("truncate"),
    js_string!("access"),
    js_string!("watch"),
    // default
    js_string!("default"),
  ];

  let module = Module::synthetic(
    export_names,
    SyntheticModuleInitializer::from_copy_closure(
      |m: &boa_engine::module::SyntheticModule, ctx: &mut Context| {
        // ── P0 ─────────────────────────────────────────────────────────────

        // readTextFile(path) → Promise<string>
        let read_text = to_js_fn(
          (|path: JsString, ctx: &mut Context| -> JsResult<JsValue> {
            let p = path.to_std_string_escaped();
            match std::fs::read_to_string(&p) {
              Ok(c) => Ok(resolved_promise(JsValue::from(JsString::from(c)), ctx)),
              Err(e) => Ok(rejected_promise(&format!("readTextFile: {e}"), ctx)),
            }
          }).into_js_function_copied(ctx),
          "readTextFile", 1, ctx,
        );

        // writeTextFile(path, data) → Promise<void>
        let write_text = to_js_fn(
          (|path: JsString, data: JsString, ctx: &mut Context| -> JsResult<JsValue> {
            let p = path.to_std_string_escaped();
            match std::fs::write(&p, data.to_std_string_escaped()) {
              Ok(()) => Ok(resolved_promise(JsValue::undefined(), ctx)),
              Err(e) => Ok(rejected_promise(&format!("writeTextFile: {e}"), ctx)),
            }
          }).into_js_function_copied(ctx),
          "writeTextFile", 2, ctx,
        );

        // readFileSync(path) → ArrayBuffer (like Deno.readFileSync)
        let read_sync = to_js_fn(
          (|path: JsString, ctx: &mut Context| -> JsResult<JsValue> {
            let p = path.to_std_string_escaped();
            match std::fs::read(&p) {
              Ok(bytes) => {
                let buf = JsArrayBuffer::new(bytes.len(), ctx)
                  .map_err(|e| JsError::from(JsNativeError::typ().with_message(format!("readFileSync: buffer alloc: {e}"))))?;
                if let Some(mut data) = buf.data_mut() { data.copy_from_slice(&bytes); }
                Ok(buf.into())
              }
              Err(e) => Err(JsError::from(JsNativeError::typ().with_message(format!("readFileSync: {e}")))),
            }
          }).into_js_function_copied(ctx),
          "readFileSync", 1, ctx,
        );

        // readFile(path) → Promise<ArrayBuffer>
        let read_file = to_js_fn(
          (|path: JsString, ctx: &mut Context| -> JsResult<JsValue> {
            let p = path.to_std_string_escaped();
            match std::fs::read(&p) {
              Ok(bytes) => match JsArrayBuffer::new(bytes.len(), ctx) {
                Ok(buf) => {
                  if let Some(mut data) = buf.data_mut() { data.copy_from_slice(&bytes); }
                  Ok(resolved_promise(buf.into(), ctx))
                }
                Err(e) => Ok(rejected_promise(&format!("readFile: buffer alloc: {e}"), ctx)),
              },
              Err(e) => Ok(rejected_promise(&format!("readFile: {e}"), ctx)),
            }
          }).into_js_function_copied(ctx),
          "readFile", 1, ctx,
        );

        // writeFile(path, data) → Promise<void>
        // Accepts string | ArrayBuffer | Uint8Array (aligned with Deno/Bun)
        let write_file = to_js_fn(
          (|path: JsString, data: JsValue, ctx: &mut Context| -> JsResult<JsValue> {
            let p = path.to_std_string_escaped();
            // Try string first
            if let Ok(s) = data.to_string(ctx) {
              return match std::fs::write(&p, s.to_std_string_escaped()) {
                Ok(()) => Ok(resolved_promise(JsValue::undefined(), ctx)),
                Err(e) => Ok(rejected_promise(&format!("writeFile: {e}"), ctx)),
              };
            }
            // Try ArrayBuffer / TypedArray
            let data_bytes = extract_bytes_from_jsvalue(&data, ctx);
            match data_bytes {
              Some(bytes) => match std::fs::write(&p, &bytes) {
                Ok(()) => Ok(resolved_promise(JsValue::undefined(), ctx)),
                Err(e) => Ok(rejected_promise(&format!("writeFile: {e}"), ctx)),
              },
              None => Ok(rejected_promise("writeFile: data must be a string, ArrayBuffer, or TypedArray", ctx)),
            }
          }).into_js_function_copied(ctx),
          "writeFile", 2, ctx,
        );

        // exists(path) → Promise<bool>
        let exists_fn = to_js_fn(
          (|path: JsString, ctx: &mut Context| -> JsResult<JsValue> {
            let ps = path.to_std_string_escaped();
            let e = std::path::Path::new(&ps).exists();
            Ok(resolved_promise(JsValue::from(e), ctx))
          }).into_js_function_copied(ctx),
          "exists", 1, ctx,
        );

        // ── P1 ─────────────────────────────────────────────────────────────

        // mkdir(path, opts?) → Promise<void>
        let mkdir_fn = to_js_fn(
          (|path: JsString, opts: Option<JsValue>, ctx: &mut Context| -> JsResult<JsValue> {
            let ps = path.to_std_string_escaped();
            let r = if is_recursive(opts, ctx) { std::fs::create_dir_all(&ps) }
                    else { std::fs::create_dir(&ps) };
            match r {
              Ok(()) => Ok(resolved_promise(JsValue::undefined(), ctx)),
              Err(e) => Ok(rejected_promise(&format!("mkdir: {e}"), ctx)),
            }
          }).into_js_function_copied(ctx),
          "mkdir", 2, ctx,
        );

        // remove(path, opts?) → Promise<void>
        let remove_fn = to_js_fn(
          (|path: JsString, opts: Option<JsValue>, ctx: &mut Context| -> JsResult<JsValue> {
            let ps = path.to_std_string_escaped();
            let r = if is_recursive(opts, ctx) { std::fs::remove_dir_all(&ps) }
                    else { std::fs::remove_file(&ps) };
            match r {
              Ok(()) => Ok(resolved_promise(JsValue::undefined(), ctx)),
              Err(e) => Ok(rejected_promise(&format!("remove: {e}"), ctx)),
            }
          }).into_js_function_copied(ctx),
          "remove", 2, ctx,
        );

        // readdir(path) → Promise<string[]>
        let readdir_fn = to_js_fn(
          (|path: JsString, ctx: &mut Context| -> JsResult<JsValue> {
            let p = path.to_std_string_escaped();
            match std::fs::read_dir(&p) {
              Ok(entries) => {
                let arr = JsArray::new(ctx);
                for entry in entries.flatten() {
                  if let Some(name) = entry.file_name().to_str() {
                    let _ = arr.push(JsValue::from(JsString::from(name)), ctx);
                  }
                }
                Ok(resolved_promise(arr.into(), ctx))
              }
              Err(e) => Ok(rejected_promise(&format!("readdir: {e}"), ctx)),
            }
          }).into_js_function_copied(ctx),
          "readdir", 1, ctx,
        );

        // stat(path) → Promise<FileInfo>
        let stat_fn = to_js_fn(
          (|path: JsString, ctx: &mut Context| -> JsResult<JsValue> {
            let p = path.to_std_string_escaped();
            match std::fs::metadata(&p) {
              Ok(md) => Ok(resolved_promise(metadata_to_obj(&md, ctx)?, ctx)),
              Err(e) => Ok(rejected_promise(&format!("stat: {e}"), ctx)),
            }
          }).into_js_function_copied(ctx),
          "stat", 1, ctx,
        );

        // lstat(path) → Promise<FileInfo>
        let lstat_fn = to_js_fn(
          (|path: JsString, ctx: &mut Context| -> JsResult<JsValue> {
            let p = path.to_std_string_escaped();
            match std::fs::symlink_metadata(&p) {
              Ok(md) => Ok(resolved_promise(metadata_to_obj(&md, ctx)?, ctx)),
              Err(e) => Ok(rejected_promise(&format!("lstat: {e}"), ctx)),
            }
          }).into_js_function_copied(ctx),
          "lstat", 1, ctx,
        );

        // appendFile(path, data) → Promise<void>
        let append_fn = to_js_fn(
          (|path: JsString, data: JsValue, ctx: &mut Context| -> JsResult<JsValue> {
            let p = path.to_std_string_escaped();
            match data.to_string(ctx) {
              Ok(s) => match std::fs::OpenOptions::new().append(true).create(true).open(&p) {
                Ok(mut f) => {
                  use std::io::Write;
                  match f.write_all(s.to_std_string_escaped().as_bytes()) {
                    Ok(()) => Ok(resolved_promise(JsValue::undefined(), ctx)),
                    Err(e) => Ok(rejected_promise(&format!("appendFile: {e}"), ctx)),
                  }
                }
                Err(e) => Ok(rejected_promise(&format!("appendFile: {e}"), ctx)),
              },
              Err(_) => Ok(rejected_promise("appendFile: data must be a string", ctx)),
            }
          }).into_js_function_copied(ctx),
          "appendFile", 2, ctx,
        );

        // copyFile(src, dst) → Promise<void>
        let copy_fn = to_js_fn(
          (|src: JsString, dst: JsString, ctx: &mut Context| -> JsResult<JsValue> {
            match std::fs::copy(src.to_std_string_escaped(), dst.to_std_string_escaped()) {
              Ok(_) => Ok(resolved_promise(JsValue::undefined(), ctx)),
              Err(e) => Ok(rejected_promise(&format!("copyFile: {e}"), ctx)),
            }
          }).into_js_function_copied(ctx),
          "copyFile", 2, ctx,
        );

        // rename(old, new) → Promise<void>
        let rename_fn = to_js_fn(
          (|old: JsString, new: JsString, ctx: &mut Context| -> JsResult<JsValue> {
            match std::fs::rename(old.to_std_string_escaped(), new.to_std_string_escaped()) {
              Ok(()) => Ok(resolved_promise(JsValue::undefined(), ctx)),
              Err(e) => Ok(rejected_promise(&format!("rename: {e}"), ctx)),
            }
          }).into_js_function_copied(ctx),
          "rename", 2, ctx,
        );

        // realpath(path) → Promise<string>
        let realpath_fn = to_js_fn(
          (|path: JsString, ctx: &mut Context| -> JsResult<JsValue> {
            match std::fs::canonicalize(path.to_std_string_escaped()) {
              Ok(p) => Ok(resolved_promise(JsValue::from(JsString::from(p.to_string_lossy().to_string())), ctx)),
              Err(e) => Ok(rejected_promise(&format!("realpath: {e}"), ctx)),
            }
          }).into_js_function_copied(ctx),
          "realpath", 1, ctx,
        );

        // symlink(target, path) → Promise<void>
        let symlink_fn = to_js_fn(
          (|target: JsString, path: JsString, ctx: &mut Context| -> JsResult<JsValue> {
            #[cfg(unix)]
            {
              match std::os::unix::fs::symlink(
                target.to_std_string_escaped(),
                path.to_std_string_escaped(),
              ) {
                Ok(()) => Ok(resolved_promise(JsValue::undefined(), ctx)),
                Err(e) => Ok(rejected_promise(&format!("symlink: {e}"), ctx)),
              }
            }
            #[cfg(not(unix))]
            {
              let _ = (target, path);
              Ok(rejected_promise("symlink: only supported on Unix", ctx))
            }
          }).into_js_function_copied(ctx),
          "symlink", 2, ctx,
        );

        // ── P2 同步版 ──────────────────────────────────────────────────────

        // existsSync(path) → bool
        let exists_sync = to_js_fn(
          (|path: JsString, _ctx: &mut Context| -> JsResult<JsValue> {
            let ps = path.to_std_string_escaped();
            Ok(JsValue::from(std::path::Path::new(&ps).exists()))
          }).into_js_function_copied(ctx),
          "existsSync", 1, ctx,
        );

        // mkdirSync(path, opts?) → void
        let mkdir_sync = to_js_fn(
          (|path: JsString, opts: Option<JsValue>, ctx: &mut Context| -> JsResult<JsValue> {
            let ps = path.to_std_string_escaped();
            let r = if is_recursive(opts, ctx) { std::fs::create_dir_all(&ps) }
                    else { std::fs::create_dir(&ps) };
            match r {
              Ok(()) => Ok(JsValue::undefined()),
              Err(e) => Err(JsError::from(JsNativeError::typ().with_message(format!("mkdirSync: {e}")))),
            }
          }).into_js_function_copied(ctx),
          "mkdirSync", 2, ctx,
        );

        // removeSync(path, opts?) → void
        let remove_sync = to_js_fn(
          (|path: JsString, opts: Option<JsValue>, ctx: &mut Context| -> JsResult<JsValue> {
            let ps = path.to_std_string_escaped();
            let r = if is_recursive(opts, ctx) { std::fs::remove_dir_all(&ps) }
                    else { std::fs::remove_file(&ps) };
            match r {
              Ok(()) => Ok(JsValue::undefined()),
              Err(e) => Err(JsError::from(JsNativeError::typ().with_message(format!("removeSync: {e}")))),
            }
          }).into_js_function_copied(ctx),
          "removeSync", 2, ctx,
        );

        // readdirSync(path) → string[]
        let readdir_sync = to_js_fn(
          (|path: JsString, ctx: &mut Context| -> JsResult<JsValue> {
            let p = path.to_std_string_escaped();
            match std::fs::read_dir(&p) {
              Ok(entries) => {
                let arr = JsArray::new(ctx);
                for entry in entries.flatten() {
                  if let Some(name) = entry.file_name().to_str() {
                    let _ = arr.push(JsValue::from(JsString::from(name)), ctx);
                  }
                }
                Ok(arr.into())
              }
              Err(e) => Err(JsError::from(JsNativeError::typ().with_message(format!("readdirSync: {e}")))),
            }
          }).into_js_function_copied(ctx),
          "readdirSync", 1, ctx,
        );

        // statSync(path) → FileInfo
        let stat_sync = to_js_fn(
          (|path: JsString, ctx: &mut Context| -> JsResult<JsValue> {
            let p = path.to_std_string_escaped();
            match std::fs::metadata(&p) {
              Ok(md) => metadata_to_obj(&md, ctx),
              Err(e) => Err(JsError::from(JsNativeError::typ().with_message(format!("statSync: {e}")))),
            }
          }).into_js_function_copied(ctx),
          "statSync", 1, ctx,
        );

        // lstatSync(path) → FileInfo
        let lstat_sync = to_js_fn(
          (|path: JsString, ctx: &mut Context| -> JsResult<JsValue> {
            let p = path.to_std_string_escaped();
            match std::fs::symlink_metadata(&p) {
              Ok(md) => metadata_to_obj(&md, ctx),
              Err(e) => Err(JsError::from(JsNativeError::typ().with_message(format!("lstatSync: {e}")))),
            }
          }).into_js_function_copied(ctx),
          "lstatSync", 1, ctx,
        );

        // appendFileSync(path, data) → void
        let append_sync = to_js_fn(
          (|path: JsString, data: JsValue, _ctx: &mut Context| -> JsResult<JsValue> {
            match data.to_string(_ctx) {
              Ok(s) => match std::fs::OpenOptions::new().append(true).create(true).open(path.to_std_string_escaped()) {
                Ok(mut f) => {
                  use std::io::Write;
                  f.write_all(s.to_std_string_escaped().as_bytes())
                    .map_err(|e| JsError::from(JsNativeError::typ().with_message(format!("appendFileSync: {e}"))))?;
                  Ok(JsValue::undefined())
                }
                Err(e) => Err(JsError::from(JsNativeError::typ().with_message(format!("appendFileSync: {e}")))),
              },
              Err(_) => Err(JsError::from(JsNativeError::typ().with_message("appendFileSync: data must be a string"))),
            }
          }).into_js_function_copied(ctx),
          "appendFileSync", 2, ctx,
        );

        // copyFileSync(src, dst) → void
        let copy_sync = to_js_fn(
          (|src: JsString, dst: JsString, _ctx: &mut Context| -> JsResult<JsValue> {
            std::fs::copy(src.to_std_string_escaped(), dst.to_std_string_escaped())
              .map_err(|e| JsError::from(JsNativeError::typ().with_message(format!("copyFileSync: {e}"))))?;
            Ok(JsValue::undefined())
          }).into_js_function_copied(ctx),
          "copyFileSync", 2, ctx,
        );

        // renameSync(old, new) → void
        let rename_sync = to_js_fn(
          (|old: JsString, new: JsString, _ctx: &mut Context| -> JsResult<JsValue> {
            std::fs::rename(old.to_std_string_escaped(), new.to_std_string_escaped())
              .map_err(|e| JsError::from(JsNativeError::typ().with_message(format!("renameSync: {e}"))))?;
            Ok(JsValue::undefined())
          }).into_js_function_copied(ctx),
          "renameSync", 2, ctx,
        );

        // realpathSync(path) → string
        let realpath_sync = to_js_fn(
          (|path: JsString, _ctx: &mut Context| -> JsResult<JsValue> {
            match std::fs::canonicalize(path.to_std_string_escaped()) {
              Ok(p) => Ok(JsValue::from(JsString::from(p.to_string_lossy().to_string()))),
              Err(e) => Err(JsError::from(JsNativeError::typ().with_message(format!("realpathSync: {e}")))),
            }
          }).into_js_function_copied(ctx),
          "realpathSync", 1, ctx,
        );

        // symlinkSync(target, path) → void
        let symlink_sync = to_js_fn(
          (|target: JsString, path: JsString, _ctx: &mut Context| -> JsResult<JsValue> {
            #[cfg(unix)]
            {
              std::os::unix::fs::symlink(target.to_std_string_escaped(), path.to_std_string_escaped())
                .map_err(|e| JsError::from(JsNativeError::typ().with_message(format!("symlinkSync: {e}"))))?;
              Ok(JsValue::undefined())
            }
            #[cfg(not(unix))]
            {
              let _ = (target, path);
              Err(JsError::from(JsNativeError::typ().with_message("symlinkSync: only supported on Unix")))
            }
          }).into_js_function_copied(ctx),
          "symlinkSync", 2, ctx,
        );

        // ── P3 ─────────────────────────────────────────────────────────────

        // chmod(path, mode) → Promise<void>
        let chmod_fn = to_js_fn(
          (|path: JsString, mode: JsValue, ctx: &mut Context| -> JsResult<JsValue> {
            let ps = path.to_std_string_escaped();
            let m = mode.to_number(ctx).unwrap_or(420.0) as u32;
            #[cfg(unix)]
            {
              use std::os::unix::fs::PermissionsExt;
              match std::fs::set_permissions(&ps, std::fs::Permissions::from_mode(m)) {
                Ok(()) => Ok(resolved_promise(JsValue::undefined(), ctx)),
                Err(e) => Ok(rejected_promise(&format!("chmod: {e}"), ctx)),
              }
            }
            #[cfg(not(unix))]
            {
              let _ = m;
              Ok(rejected_promise("chmod: only supported on Unix", ctx))
            }
          }).into_js_function_copied(ctx),
          "chmod", 2, ctx,
        );

        // chown(path, uid, gid) → Promise<void>
        let chown_fn = to_js_fn(
          (|path: JsString, uid: JsValue, gid: JsValue, ctx: &mut Context| -> JsResult<JsValue> {
            #[cfg(unix)]
            {
              let p = path.to_std_string_escaped();
              let u = uid.to_number(ctx).unwrap_or(-1.0) as u32;
              let g = gid.to_number(ctx).unwrap_or(-1.0) as u32;
              use std::os::unix::fs::chown;
              match chown(&p, Some(u), Some(g)) {
                Ok(()) => Ok(resolved_promise(JsValue::undefined(), ctx)),
                Err(e) => Ok(rejected_promise(&format!("chown: {e}"), ctx)),
              }
            }
            #[cfg(not(unix))]
            {
              let _ = (path, uid, gid);
              Ok(rejected_promise("chown: only supported on Unix", ctx))
            }
          }).into_js_function_copied(ctx),
          "chown", 3, ctx,
        );

        // link(existing, new) → Promise<void>
        let link_fn = to_js_fn(
          (|existing: JsString, new: JsString, ctx: &mut Context| -> JsResult<JsValue> {
            match std::fs::hard_link(existing.to_std_string_escaped(), new.to_std_string_escaped()) {
              Ok(()) => Ok(resolved_promise(JsValue::undefined(), ctx)),
              Err(e) => Ok(rejected_promise(&format!("link: {e}"), ctx)),
            }
          }).into_js_function_copied(ctx),
          "link", 2, ctx,
        );

        // truncate(path, len) → Promise<void>
        let truncate_fn = to_js_fn(
          (|path: JsString, len: JsValue, ctx: &mut Context| -> JsResult<JsValue> {
            let p = path.to_std_string_escaped();
            let l = len.to_number(ctx).unwrap_or(0.0) as u64;
            match std::fs::OpenOptions::new().write(true).open(&p) {
              Ok(f) => {
                match f.set_len(l) {
                  Ok(()) => Ok(resolved_promise(JsValue::undefined(), ctx)),
                  Err(e) => Ok(rejected_promise(&format!("truncate: {e}"), ctx)),
                }
              }
              Err(e) => Ok(rejected_promise(&format!("truncate: {e}"), ctx)),
            }
          }).into_js_function_copied(ctx),
          "truncate", 2, ctx,
        );

        // access(path, mode?) → Promise<void>
        let access_fn = to_js_fn(
          (|path: JsString, mode: Option<JsValue>, ctx: &mut Context| -> JsResult<JsValue> {
            let ps = path.to_std_string_escaped();
            let m = mode.and_then(|v| v.as_number()).unwrap_or(0f64) as u32;
            if !std::path::Path::new(&ps).exists() {
              return Ok(rejected_promise(&format!("access: ENOENT, {ps}"), ctx));
            }
            if m == 0 { return Ok(resolved_promise(JsValue::undefined(), ctx)); }
            #[cfg(unix)]
            {
              use std::os::unix::fs::PermissionsExt;
              match std::fs::metadata(&ps) {
                Ok(md) => {
                  let perm = md.permissions().mode();
                  if (m & 4) != 0 && (perm & 0o444) == 0 {
                    return Ok(rejected_promise(&format!("access: EACCES, not readable: {ps}"), ctx));
                  }
                  if (m & 2) != 0 && (perm & 0o222) == 0 {
                    return Ok(rejected_promise(&format!("access: EACCES, not writable: {ps}"), ctx));
                  }
                  if (m & 1) != 0 && (perm & 0o111) == 0 {
                    return Ok(rejected_promise(&format!("access: EACCES, not executable: {ps}"), ctx));
                  }
                  Ok(resolved_promise(JsValue::undefined(), ctx))
                }
                Err(e) => Ok(rejected_promise(&format!("access: {e}"), ctx)),
              }
            }
            #[cfg(not(unix))]
            { Ok(resolved_promise(JsValue::undefined(), ctx)) }
          }).into_js_function_copied(ctx),
          "access", 2, ctx,
        );

        // watch(path, opts?) → AsyncIterator
        //
        // 使用轮询（~200ms）检查 mtime 变化，返回 { next() → Promise<{value, done}> } 对象
        let watch_fn = to_js_fn(
          (|path: JsString, opts: Option<JsValue>, ctx: &mut Context| -> JsResult<JsValue> {
            let ps = path.to_std_string_escaped();
            let _p = std::path::Path::new(&ps);
            let _recursive = is_recursive(opts, ctx);

            // 读取当前 mtime，失败时也支持（文件可能尚不存在）
            let mtime = current_mtime(&ps);
            let state = get_watch_state(ctx);
            let entry_id = state.borrow_mut().next_id;
            state.borrow_mut().next_id += 1;

            // 添加条目
            state.borrow().entries.borrow_mut().push(FsWatchEntry {
                id: entry_id,
                path: ps.clone(),
                last_mtime: mtime,
                pending: VecDeque::new(),
                done: false,
            });

            // 为所有条目注册轮询 job（仅首次调用时注册一次）
            let poll_needed = {
                let s = state.borrow();
                let entries = s.entries.borrow();
                // 如果这是第一个条目，注册轮询 job
                entries.len() == 1
            };
            if poll_needed {
                let poll_state = state.clone();
                // 轮询函数：检查 mtime → 解决 pending → 自重新入队
                fn do_poll(state: &Gc<GcRefCell<FsWatchState>>, ctx: &mut Context) {
                    let s = state.borrow();
                    let mut entries = s.entries.borrow_mut();
                    for entry in entries.iter_mut() {
                        if entry.done { continue; }
                        let current = current_mtime(&entry.path);
                        if current != entry.last_mtime && current != 0 {
                            entry.last_mtime = current;
                            let event_obj = JsObject::with_object_proto(ctx.intrinsics());
                            let _ = event_obj.set(
                                js_string!("type"), JsValue::from(js_string!("modify")), false, ctx,
                            );
                            let _ = event_obj.set(
                                js_string!("paths"),
                                {
                                    let arr = JsArray::new(ctx);
                                    let _ = arr.push(JsValue::from(JsString::from(entry.path.clone())), ctx);
                                    JsValue::from(arr)
                                },
                                false, ctx,
                            );
                            let iter_result = {
                                let ir = JsObject::with_object_proto(ctx.intrinsics());
                                let _ = ir.set(js_string!("value"), event_obj, false, ctx);
                                let _ = ir.set(js_string!("done"), JsValue::from(false), false, ctx);
                                ir
                            };
                            while let Some(resolve) = entry.pending.pop_front() {
                                let resolve_fn: JsFunction = resolve;
                                let _ = resolve_fn.call(
                                    &JsValue::undefined(),
                                    &[iter_result.clone().into()],
                                    ctx,
                                );
                            }
                        }
                    }
                }
                // 创建自重新入队的轮询 job，类似 setInterval 模式
                fn schedule_poll(state: Gc<GcRefCell<FsWatchState>>, ctx: &mut Context) {
                    ctx.enqueue_job(
                        TimeoutJob::new(
                            NativeJob::new(move |ctx| {
                                do_poll(&state, ctx);
                                schedule_poll(state.clone(), ctx);
                                Ok(JsValue::undefined())
                            }),
                            200,
                        ).into()
                    );
                }
                schedule_poll(poll_state, ctx);
            }

            // 创建 watcher 对象
            let _async_iter_proto = ctx.intrinsics().objects().iterator_prototypes().async_iterator();
            let watcher_obj = JsObject::with_object_proto(ctx.intrinsics());
            let _ = watcher_obj.set(js_string!("__watch_id"), JsValue::from(entry_id as f64), false, ctx);

            // next()
            let next_fn = {
                let state_clone = state.clone();
                unsafe {
                    NativeFunction::from_closure_with_captures(
                        move |this: &JsValue, _args: &[JsValue], captures: &Gc<GcRefCell<FsWatchState>>, ctx: &mut Context| -> JsResult<JsValue> {
                            let id = this.as_object()
                                .and_then(|o| o.get(js_string!("__watch_id"), ctx).ok())
                                .and_then(|v| v.as_number())
                                .map(|n| n as u32)
                                .unwrap_or(0);
                            let s = captures.borrow();
                            let entries = s.entries.borrow();
                            let entry = entries.iter().find(|e| e.id == id);
                            match entry {
                                Some(entry) if entry.done => {
                                    let ir = JsObject::with_object_proto(ctx.intrinsics());
                                    let _ = ir.set(js_string!("value"), JsValue::undefined(), false, ctx);
                                    let _ = ir.set(js_string!("done"), JsValue::from(true), false, ctx);
                                    Ok(JsPromise::resolve(ir, ctx).into())
                                }
                                Some(entry) if !entry.pending.is_empty() => {
                                    let resolve = entry.pending.front().unwrap().clone();
                                    let ir = JsObject::with_object_proto(ctx.intrinsics());
                                    let _ = ir.set(js_string!("value"), JsValue::undefined(), false, ctx);
                                    let _ = ir.set(js_string!("done"), JsValue::from(false), false, ctx);
                                    let _ = resolve.call(&JsValue::undefined(), &[JsValue::from(ir)], ctx);
                                    let (promise, _resolvers) = JsPromise::new_pending(ctx);
                                    Ok(promise.into())
                                }
                                Some(_) => {
                                    // 没有排队事件，创建新 promise 等待轮询
                                    let (promise, resolvers) = JsPromise::new_pending(ctx);
                                    // 从 borrow 中释放才能插入
                                    drop(entries);
                                    let mut entries_mut = s.entries.borrow_mut();
                                    if let Some(e) = entries_mut.iter_mut().find(|e| e.id == id) {
                                        e.pending.push_back(resolvers.resolve);
                                    }
                                    Ok(promise.into())
                                }
                                None => {
                                    Ok(JsPromise::resolve(JsValue::undefined(), ctx).into())
                                }
                            }
                        },
                        state_clone,
                    )
                }
            };
            let next_val = to_js_fn(next_fn, "next", 0, ctx);
            let _ = watcher_obj.set(js_string!("next"), next_val.clone(), false, ctx);

            // [Symbol.asyncIterator]() → this
            let async_iter_fn = {
                unsafe {
                    NativeFunction::from_closure_with_captures(
                        |this: &JsValue, _args: &[JsValue], _: &(), _ctx: &mut Context| -> JsResult<JsValue> {
                            Ok(this.clone())
                        },
                        (),
                    )
                }
            };
            let async_iter_val = to_js_fn(async_iter_fn, "[Symbol.asyncIterator]", 0, ctx);
            let _ = watcher_obj.set(JsSymbol::async_iterator(), async_iter_val, false, ctx);

            Ok(watcher_obj.into())
          }).into_js_function_copied(ctx),
          "watch", 2, ctx,
        );

        // ── 设置命名导出 ────────────────────────────────────────────────────

        m.set_export(&js_string!("readTextFile"), read_text.clone())?;
        m.set_export(&js_string!("writeTextFile"), write_text.clone())?;
        m.set_export(&js_string!("readFile"), read_file.clone())?;
        m.set_export(&js_string!("writeFile"), write_file.clone())?;
        m.set_export(&js_string!("readFileSync"), read_sync.clone())?;
        m.set_export(&js_string!("exists"), exists_fn.clone())?;

        m.set_export(&js_string!("mkdir"), mkdir_fn.clone())?;
        m.set_export(&js_string!("remove"), remove_fn.clone())?;
        m.set_export(&js_string!("readdir"), readdir_fn.clone())?;
        m.set_export(&js_string!("stat"), stat_fn.clone())?;
        m.set_export(&js_string!("lstat"), lstat_fn.clone())?;
        m.set_export(&js_string!("appendFile"), append_fn.clone())?;
        m.set_export(&js_string!("copyFile"), copy_fn.clone())?;
        m.set_export(&js_string!("rename"), rename_fn.clone())?;
        m.set_export(&js_string!("realpath"), realpath_fn.clone())?;
        m.set_export(&js_string!("symlink"), symlink_fn.clone())?;

        m.set_export(&js_string!("existsSync"), exists_sync.clone())?;
        m.set_export(&js_string!("mkdirSync"), mkdir_sync.clone())?;
        m.set_export(&js_string!("removeSync"), remove_sync.clone())?;
        m.set_export(&js_string!("readdirSync"), readdir_sync.clone())?;
        m.set_export(&js_string!("statSync"), stat_sync.clone())?;
        m.set_export(&js_string!("lstatSync"), lstat_sync.clone())?;
        m.set_export(&js_string!("appendFileSync"), append_sync.clone())?;
        m.set_export(&js_string!("copyFileSync"), copy_sync.clone())?;
        m.set_export(&js_string!("renameSync"), rename_sync.clone())?;
        m.set_export(&js_string!("realpathSync"), realpath_sync.clone())?;
        m.set_export(&js_string!("symlinkSync"), symlink_sync.clone())?;

        m.set_export(&js_string!("chmod"), chmod_fn.clone())?;
        m.set_export(&js_string!("chown"), chown_fn.clone())?;
        m.set_export(&js_string!("link"), link_fn.clone())?;
        m.set_export(&js_string!("truncate"), truncate_fn.clone())?;
        m.set_export(&js_string!("access"), access_fn.clone())?;
        m.set_export(&js_string!("watch"), watch_fn.clone())?;

        // ── default — 整个 fs 对象 ──────────────────────────────────────────

        let fsobj = JsObject::with_object_proto(ctx.intrinsics());
        let all_exports: &[(JsString, JsValue)] = &[
          (js_string!("readTextFile"), read_text),
          (js_string!("writeTextFile"), write_text),
          (js_string!("readFile"), read_file),
          (js_string!("writeFile"), write_file),
          (js_string!("readFileSync"), read_sync),
          (js_string!("exists"), exists_fn),
          (js_string!("mkdir"), mkdir_fn),
          (js_string!("remove"), remove_fn),
          (js_string!("readdir"), readdir_fn),
          (js_string!("stat"), stat_fn),
          (js_string!("lstat"), lstat_fn),
          (js_string!("appendFile"), append_fn),
          (js_string!("copyFile"), copy_fn),
          (js_string!("rename"), rename_fn),
          (js_string!("realpath"), realpath_fn),
          (js_string!("symlink"), symlink_fn),
          (js_string!("existsSync"), exists_sync),
          (js_string!("mkdirSync"), mkdir_sync),
          (js_string!("removeSync"), remove_sync),
          (js_string!("readdirSync"), readdir_sync),
          (js_string!("statSync"), stat_sync),
          (js_string!("lstatSync"), lstat_sync),
          (js_string!("appendFileSync"), append_sync),
          (js_string!("copyFileSync"), copy_sync),
          (js_string!("renameSync"), rename_sync),
          (js_string!("realpathSync"), realpath_sync),
          (js_string!("symlinkSync"), symlink_sync),
          (js_string!("chmod"), chmod_fn),
          (js_string!("chown"), chown_fn),
          (js_string!("link"), link_fn),
          (js_string!("truncate"), truncate_fn),
          (js_string!("access"), access_fn),
          (js_string!("watch"), watch_fn),
        ];
        for (name, val) in all_exports {
          let _ = fsobj.set(name.clone(), val.clone(), false, ctx);
        }
        m.set_export(&js_string!("default"), fsobj.into())?;

        Ok(())
      },
    ),
    None,
    None,
    context,
  );

  Ok(module)
}
