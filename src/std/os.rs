use boa_engine::module::SyntheticModuleInitializer;
use boa_engine::{
  object::FunctionObjectBuilder,
  Context, IntoJsFunctionCopied, JsObject, JsResult, JsString, JsValue,
  Module, js_string,
};

fn make_fn(native: boa_engine::NativeFunction, name: &str, len: usize, ctx: &mut Context) -> JsValue {
  FunctionObjectBuilder::new(ctx.realm(), native)
    .name(JsString::from(name))
    .length(len)
    .build()
    .into()
}

fn os_type_str() -> &'static str {
  if cfg!(target_os = "macos") { "Darwin" }
  else if cfg!(target_os = "linux") { "Linux" }
  else if cfg!(target_os = "windows") { "Windows_NT" }
  else { "Unknown" }
}

fn eol_str() -> &'static str {
  if cfg!(windows) { "\r\n" } else { "\n" }
}

fn release_from_sysctl() -> Option<String> {
  std::process::Command::new("uname").arg("-r").output().ok()
    .and_then(|o| {
      if o.status.success() {
        String::from_utf8(o.stdout).ok().map(|s| s.trim().to_string())
      } else { None }
    })
}

fn mem_from_command() -> (f64, f64) {
  #[cfg(target_os = "macos")]
  {
    let total = std::process::Command::new("sysctl")
      .arg("-n").arg("hw.memsize")
      .output().ok()
      .and_then(|o| {
        if o.status.success() {
          String::from_utf8(o.stdout).ok()
            .and_then(|s| s.trim().parse::<u64>().ok())
        } else { None }
      })
      .unwrap_or(0);

    let page_size = std::process::Command::new("pagesize")
      .output().ok()
      .and_then(|o| {
        if o.status.success() {
          String::from_utf8(o.stdout).ok()
            .and_then(|s| s.trim().parse::<u64>().ok())
        } else { None }
      })
      .unwrap_or(4096);

    let vm_stat = std::process::Command::new("vm_stat")
      .output().ok()
      .and_then(|o| {
        if o.status.success() { String::from_utf8(o.stdout).ok() } else { None }
      });

    let free = vm_stat.and_then(|s| {
      for line in s.lines() {
        if line.contains("free") {
          let val = line.split(':').nth(1)?.trim().trim_end_matches('.');
          return val.parse::<u64>().ok().map(|pages| pages * page_size);
        }
      }
      None
    }).unwrap_or(0);

    (total as f64, free as f64)
  }
  #[cfg(target_os = "linux")]
  {
    let meminfo = try_read_file("/proc/meminfo");
    let mut total = 0f64;
    let mut free = 0f64;
    if let Some(content) = meminfo {
      for line in content.lines() {
        if line.starts_with("MemTotal:") {
          total = line.split_whitespace().nth(1).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0) * 1024.0;
        } else if line.starts_with("MemFree:") || line.starts_with("MemAvailable:") {
          let v = line.split_whitespace().nth(1).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
          free = if line.starts_with("MemAvailable:") { v * 1024.0 } else { free.max(v * 1024.0) };
        }
      }
    }
    (total, free)
  }
  #[cfg(not(any(target_os = "macos", target_os = "linux")))]
  {
    (0.0, 0.0)
  }
}

/// 创建 "os" 内置模块
pub fn create_os_module(context: &mut Context) -> Result<Module, String> {
  let export_names: &[JsString] = &[
    js_string!("platform"),
    js_string!("arch"),
    js_string!("EOL"),
    js_string!("hostname"),
    js_string!("type"),
    js_string!("release"),
    js_string!("homedir"),
    js_string!("tmpdir"),
    js_string!("totalmem"),
    js_string!("freemem"),
    js_string!("default"),
  ];

  let module = Module::synthetic(
    export_names,
    SyntheticModuleInitializer::from_copy_closure(
      |m: &boa_engine::module::SyntheticModule, ctx: &mut Context| {
        // ── P0 ──────────────────────────────────────────────────────────────

        // platform()
        let platform_fn = make_fn(
          (|_: &mut Context| -> JsResult<JsValue> {
            Ok(JsValue::from(js_string!(platform_str())))
          })
          .into_js_function_copied(ctx),
          "platform", 0, ctx,
        );
        m.set_export(&js_string!("platform"), platform_fn.clone())?;

        // arch()
        let arch_fn = make_fn(
          (|_: &mut Context| -> JsResult<JsValue> {
            Ok(JsValue::from(js_string!(arch_str())))
          })
          .into_js_function_copied(ctx),
          "arch", 0, ctx,
        );
        m.set_export(&js_string!("arch"), arch_fn.clone())?;

        // EOL
        m.set_export(&js_string!("EOL"), JsValue::from(js_string!(eol_str())))?;

        // ── P1 ──────────────────────────────────────────────────────────────

        // hostname()
        let hostname_fn = make_fn(
          (|_: &mut Context| -> JsResult<JsValue> {
            let hostname = std::process::Command::new("hostname")
              .output().ok()
              .and_then(|o| {
                if o.status.success() {
                  String::from_utf8(o.stdout).ok().map(|s| JsString::from(s.trim()))
                } else { None }
              })
              .unwrap_or_else(|| js_string!("localhost"));
            Ok(JsValue::from(hostname))
          })
          .into_js_function_copied(ctx),
          "hostname", 0, ctx,
        );
        m.set_export(&js_string!("hostname"), hostname_fn.clone())?;

        // type()
        let type_fn = make_fn(
          (|_: &mut Context| -> JsResult<JsValue> {
            Ok(JsValue::from(js_string!(os_type_str())))
          })
          .into_js_function_copied(ctx),
          "type", 0, ctx,
        );
        m.set_export(&js_string!("type"), type_fn.clone())?;

        // release()
        let release_fn = make_fn(
          (|_: &mut Context| -> JsResult<JsValue> {
            let r = release_from_sysctl().unwrap_or_else(|| "0.0.0".to_string());
            Ok(JsValue::from(js_string!(r)))
          })
          .into_js_function_copied(ctx),
          "release", 0, ctx,
        );
        m.set_export(&js_string!("release"), release_fn.clone())?;

        // homedir()
        let homedir_fn = make_fn(
          (|_: &mut Context| -> JsResult<JsValue> {
            let dir = {
              #[cfg(unix)]
              { std::env::var("HOME").ok() }
              #[cfg(windows)]
              { std::env::var("USERPROFILE").ok() }
              #[cfg(not(any(unix, windows)))]
              { None }
            }.unwrap_or_else(|| "/".to_string());
            Ok(JsValue::from(js_string!(dir)))
          })
          .into_js_function_copied(ctx),
          "homedir", 0, ctx,
        );
        m.set_export(&js_string!("homedir"), homedir_fn.clone())?;

        // tmpdir()
        let tmpdir_fn = make_fn(
          (|_: &mut Context| -> JsResult<JsValue> {
            let dir = {
              #[cfg(unix)]
              { std::env::var("TMPDIR").or_else(|_| std::env::var("TMP")).or_else(|_| std::env::var("TEMPDIR")).ok() }
              #[cfg(windows)]
              { std::env::var("TEMP").or_else(|_| std::env::var("TMP")).ok() }
              #[cfg(not(any(unix, windows)))]
              { None }
            }.unwrap_or_else(|| "/tmp".to_string());
            Ok(JsValue::from(js_string!(dir)))
          })
          .into_js_function_copied(ctx),
          "tmpdir", 0, ctx,
        );
        m.set_export(&js_string!("tmpdir"), tmpdir_fn.clone())?;

        // ── P2 ──────────────────────────────────────────────────────────────

        // totalmem()
        let totalmem_fn = make_fn(
          (|_: &mut Context| -> JsResult<JsValue> {
            let (total, _) = mem_from_command();
            Ok(JsValue::from(total))
          })
          .into_js_function_copied(ctx),
          "totalmem", 0, ctx,
        );
        m.set_export(&js_string!("totalmem"), totalmem_fn.clone())?;

        // freemem()
        let freemem_fn = make_fn(
          (|_: &mut Context| -> JsResult<JsValue> {
            let (_, free) = mem_from_command();
            Ok(JsValue::from(free))
          })
          .into_js_function_copied(ctx),
          "freemem", 0, ctx,
        );
        m.set_export(&js_string!("freemem"), freemem_fn.clone())?;

        // ── default — 整个 os 对象 ─────────────────────────────────────────

        let oobj = JsObject::with_object_proto(ctx.intrinsics());
        let _ = oobj.set(js_string!("platform"), platform_fn, false, ctx);
        let _ = oobj.set(js_string!("arch"), arch_fn, false, ctx);
        let _ = oobj.set(js_string!("EOL"), JsValue::from(js_string!(eol_str())), false, ctx);
        let _ = oobj.set(js_string!("hostname"), hostname_fn, false, ctx);
        let _ = oobj.set(js_string!("type"), type_fn, false, ctx);
        let _ = oobj.set(js_string!("release"), release_fn, false, ctx);
        let _ = oobj.set(js_string!("homedir"), homedir_fn, false, ctx);
        let _ = oobj.set(js_string!("tmpdir"), tmpdir_fn, false, ctx);
        let _ = oobj.set(js_string!("totalmem"), totalmem_fn, false, ctx);
        let _ = oobj.set(js_string!("freemem"), freemem_fn, false, ctx);
        m.set_export(&js_string!("default"), oobj.into())?;

        Ok(())
      },
    ),
    None,
    None,
    context,
  );

  Ok(module)
}

fn platform_str() -> &'static str {
  if cfg!(target_os = "macos") { "darwin" }
  else if cfg!(target_os = "linux") { "linux" }
  else if cfg!(target_os = "windows") { "win32" }
  else { "unknown" }
}

fn arch_str() -> &'static str {
  if cfg!(target_arch = "x86_64") { "x64" }
  else if cfg!(target_arch = "aarch64") { "arm64" }
  else { "unknown" }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_platform_str_non_empty() {
    let p = platform_str();
    assert!(!p.is_empty());
    assert!(p == "darwin" || p == "linux" || p == "win32" || p == "unknown");
  }

  #[test]
  fn test_arch_str_non_empty() {
    let a = arch_str();
    assert!(!a.is_empty());
    assert!(a == "x64" || a == "arm64" || a == "unknown");
  }

  #[test]
  fn test_eol_str() {
    let e = eol_str();
    assert!(e == "\n" || e == "\r\n");
  }

  #[test]
  fn test_os_type_str_non_empty() {
    let t = os_type_str();
    assert!(!t.is_empty());
  }
}
